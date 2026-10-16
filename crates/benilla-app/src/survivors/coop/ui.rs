//! The menu's Multiplayer panel: Host on a port (with this machine's LAN address and how friends
//! reach it), Join an address, the lobby of everyone connected (class icon, level, title, mount and
//! hat, and the ready check's marks), the side bets ([`super::social`]), and the host's Ready Check
//! (with Start Anyway while it runs). Built into the menu the way Run Options is
//! ([`super::super::run_options`]).

use std::net::IpAddr;

use bevy::input::keyboard::{Key, KeyboardInput};
use bevy::input::ButtonState;
use bevy::prelude::*;
use bevy::ui::FocusPolicy;
use bevy_replicon::prelude::RepliconChannels;

use super::super::hud::{start_run, HudFont, MenuRoot};
use super::super::menu::{MenuBuilt, MenuSlot};
use super::super::run_options::GREEN;
use super::super::zones::Unlocks;
use super::super::{data, HeroState, LocalHero, MetaBonuses, Phase, Run, Selection};
use super::social::{self, LobbyIcon, LobbyMark, SocialSlot};
use super::{client, host, net, Link, PlayerName, Session};
use crate::glue::art::{GlueArt, GOLD, INFO_TEXT};
use crate::glue::backdrop::{backdrop_border, tiled_bg_node};
use crate::glue::widgets::{glue_button, outlined_text, GlueBtnKind, GlueDisabled, GlueText};
use crate::glue::GlueClicks;

const PANEL_W: f32 = 600.0;
const RED: Color = Color::srgb(1.0, 0.3, 0.25);
const FIELD_FILL: Color = Color::srgba(0.0, 0.0, 0.0, 0.75);
const FIELD_EDGE: Color = Color::srgb(0.45, 0.4, 0.3);
const FIELD_HOT: Color = Color::srgb(1.0, 0.82, 0.0);

/// The panel's state: open, what is typed, and which field takes the keys.
#[derive(Resource)]
pub(in crate::survivors) struct NetPanel {
    open: bool,
    port: String,
    addr: String,
    focus: Field,
    lan: Option<IpAddr>,
    /// The `WOW_SURVIVORS_HOST` / `WOW_SURVIVORS_JOIN` test aid, done once.
    env_done: bool,
}

impl Default for NetPanel {
    fn default() -> Self {
        Self {
            open: false,
            port: net::DEFAULT_PORT.to_string(),
            addr: String::new(),
            focus: Field::Addr,
            lan: None,
            env_done: false,
        }
    }
}

/// Run condition: the panel is shut, so the menu's own keys and clicks may act.
pub(in crate::survivors) fn closed(panel: Res<NetPanel>) -> bool {
    !panel.open
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Field {
    Port,
    Addr,
}

#[derive(Component, Clone, Copy, PartialEq, Eq, Debug)]
enum NetAction {
    Open,
    Host,
    Join,
    Leave,
    /// The host's Ready Check.
    Start,
    /// Start the run whoever is ready, while a check runs.
    Force,
    Done,
}

#[derive(Component)]
struct NetBuilt;
#[derive(Component)]
struct NetModal;
#[derive(Component)]
struct FieldBox(Field);
#[derive(Component)]
struct FieldText(Field);
#[derive(Component)]
struct StatusText;
#[derive(Component)]
struct HostInfo;
#[derive(Component)]
struct LobbyLine(usize);
#[derive(Component)]
struct MenuLink;

pub(super) fn plugin(app: &mut App) {
    app.init_resource::<NetPanel>().add_systems(
        Update,
        (
            build_ui,
            env_aids,
            panel_input,
            refresh_ui.before(crate::glue::GlueVisuals),
        )
            .chain()
            .before(super::super::hud::keys),
    );
}

#[allow(clippy::type_complexity)]
fn build_ui(
    mut commands: Commands,
    roots: Query<Entity, (With<MenuRoot>, With<MenuBuilt>, Without<NetBuilt>)>,
    buttons: Query<(Entity, &MenuSlot)>,
    art: Res<GlueArt>,
    font: Res<HudFont>,
) {
    let (Ok(root), Some(buttons)) = (
        roots.single(),
        buttons.iter().find_map(|(entity, slot)| {
            // Multiplayer owns only its fixed slot in the menu stack.
            (*slot == MenuSlot::Multiplayer).then_some(entity)
        }),
    ) else {
        return;
    };
    let f = &font.0;
    commands.entity(root).insert(NetBuilt).with_children(|ui| {
        modal(ui, &art, f);
    });
    commands.entity(buttons).with_children(|ui| {
        menu_button(ui, &art, f);
    });
}

/// The Multiplayer button and link state, as one block in the menu action row.
fn menu_button(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn(Node {
        width: px(150.0),
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        row_gap: px(1.0),
        ..default()
    })
    .with_children(|c| {
        glue_button(
            c,
            art,
            f,
            NetAction::Open,
            "Multiplayer",
            150.0,
            38.0,
            GlueBtnKind::Small,
            1.0,
        );
        outlined_text(
            c,
            Node::default(),
            (),
            MenuLink,
            GlueText {
                text: "",
                size: 12.0,
                color: GREEN,
                wrap: false,
            },
            f,
            1.0,
        );
    });
}

fn line(p: &mut ChildSpawnerCommands, f: &Handle<Font>, marker: impl Bundle, size: f32, c: Color) {
    outlined_text(
        p,
        Node::default(),
        (),
        marker,
        GlueText {
            text: "",
            size,
            color: c,
            wrap: false,
        },
        f,
        1.0,
    );
}

fn label(p: &mut ChildSpawnerCommands, f: &Handle<Font>, text: &str) {
    outlined_text(
        p,
        Node {
            width: Val::Px(110.0),
            ..default()
        },
        (),
        (),
        GlueText {
            text,
            size: 15.0,
            color: GOLD,
            wrap: false,
        },
        f,
        1.0,
    );
}

fn field(p: &mut ChildSpawnerCommands, f: &Handle<Font>, which: Field, width: f32) {
    p.spawn((
        FieldBox(which),
        Button,
        Interaction::default(),
        Node {
            width: Val::Px(width),
            height: Val::Px(28.0),
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(8.0)),
            border: UiRect::all(Val::Px(1.0)),
            ..default()
        },
        BackgroundColor(FIELD_FILL),
        BorderColor::all(FIELD_EDGE),
    ))
    .with_children(|b| {
        b.spawn((
            FieldText(which),
            Text::new(""),
            TextFont {
                font: f.clone(),
                font_size: 15.0,
                ..default()
            },
            TextColor(Color::WHITE),
        ));
    });
}

fn modal(ui: &mut ChildSpawnerCommands, art: &GlueArt, f: &Handle<Font>) {
    let px = Val::Px;
    ui.spawn((
        NetModal,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            display: Display::None,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        FocusPolicy::Block,
        ZIndex(100),
        Visibility::Hidden,
    ))
    .with_children(|dim| {
        dim.spawn(Node {
            width: px(PANEL_W),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            padding: UiRect::axes(px(32.0), px(26.0)),
            row_gap: px(7.0),
            ..default()
        })
        .with_children(|p| {
            if let (Some(bg), Some(border)) = (&art.dialog_bg, &art.dialog_border) {
                p.spawn((
                    tiled_bg_node(bg.clone(), 32.0, 1.0, Color::WHITE),
                    Node {
                        position_type: PositionType::Absolute,
                        left: px(11.0),
                        right: px(12.0),
                        top: px(12.0),
                        bottom: px(11.0),
                        ..default()
                    },
                ));
                backdrop_border(p, border, 32.0, Color::WHITE);
            } else {
                p.spawn((
                    BackgroundColor(Color::srgba(0.05, 0.05, 0.08, 0.95)),
                    crate::glue::widgets::overlay(),
                ));
            }
            let text = |p: &mut ChildSpawnerCommands, t: &str, size: f32, c: Color| {
                outlined_text(
                    p,
                    Node::default(),
                    (),
                    (),
                    GlueText {
                        text: t,
                        size,
                        color: c,
                        wrap: false,
                    },
                    f,
                    1.0,
                );
            };
            text(p, "Multiplayer", 20.0, GOLD);
            text(
                p,
                "One run with friends: one of you hosts, the others join.",
                13.0,
                INFO_TEXT,
            );
            let row = || Node {
                width: px(PANEL_W - 64.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: px(10.0),
                margin: UiRect::top(px(6.0)),
                ..default()
            };
            p.spawn(row()).with_children(|r| {
                label(r, f, "Host on port");
                field(r, f, Field::Port, 90.0);
                glue_button(
                    r,
                    art,
                    f,
                    NetAction::Host,
                    "Host",
                    130.0,
                    34.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
            });
            line(p, f, (HostInfo, LobbyLine(90)), 12.0, INFO_TEXT);
            line(p, f, (HostInfo, LobbyLine(91)), 12.0, INFO_TEXT);
            p.spawn(row()).with_children(|r| {
                label(r, f, "Join address");
                field(r, f, Field::Addr, 250.0);
                glue_button(
                    r,
                    art,
                    f,
                    NetAction::Join,
                    "Join",
                    130.0,
                    34.0,
                    GlueBtnKind::Dialog,
                    1.0,
                );
            });
            line(p, f, StatusText, 14.0, Color::WHITE);
            text(p, "Party", 16.0, GOLD);
            for i in 0..usize::from(super::super::party::MAX_PARTY) {
                p.spawn(Node {
                    width: px(PANEL_W - 64.0),
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: px(5.0),
                    ..default()
                })
                .with_children(|r| {
                    let icon = |size: f32| Node {
                        width: px(size),
                        height: px(size),
                        display: Display::None,
                        ..default()
                    };
                    r.spawn((
                        LobbyIcon(i),
                        icon(20.0),
                        social::Art::whole(social::CLASS_ICONS),
                    ));
                    r.spawn((
                        LobbyMark(i),
                        icon(16.0),
                        social::Art::whole(social::CLASS_ICONS),
                    ));
                    line(r, f, LobbyLine(i), 13.0, Color::WHITE);
                });
            }
            p.spawn((
                SocialSlot,
                Node {
                    width: px(PANEL_W - 64.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: px(3.0),
                    margin: UiRect::top(px(6.0)),
                    ..default()
                },
            ));
            p.spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: px(13.0),
                margin: UiRect::top(px(10.0)),
                ..default()
            })
            .with_children(|r| {
                for (action, caption) in [
                    (NetAction::Start, "Ready Check"),
                    (NetAction::Force, "Start Anyway"),
                    (NetAction::Leave, "Leave"),
                    (NetAction::Done, "Done"),
                ] {
                    glue_button(
                        r,
                        art,
                        f,
                        action,
                        caption,
                        124.0,
                        38.0,
                        GlueBtnKind::Dialog,
                        1.0,
                    );
                }
            });
        });
    });
}

/// `WOW_SURVIVORS_HOST=<port>` hosts and `WOW_SURVIVORS_JOIN=<address>` joins, once the menu has a
/// pick to bring.
fn env_aids(
    mut commands: Commands,
    mut panel: ResMut<NetPanel>,
    selection: Res<Selection>,
    phase: Res<State<Phase>>,
) {
    if panel.env_done || *phase.get() != Phase::Menu || selection.look.is_none() {
        return;
    }
    panel.env_done = true;
    if let Some(port) = crate::webenv::var("WOW_SURVIVORS_HOST") {
        let port = net::parse_port(&port).unwrap_or(net::DEFAULT_PORT);
        commands.queue(move |world: &mut World| begin_hosting(world, port));
    } else if let Some(addr) = crate::webenv::var("WOW_SURVIVORS_JOIN") {
        commands.queue(move |world: &mut World| begin_joining(world, addr));
    }
}

/// Host on `port`.
pub(super) fn begin_hosting(world: &mut World, port: u16) {
    if world.resource::<Session>().link != Link::Off {
        return;
    }
    let made = net::host(world.resource::<RepliconChannels>(), port);
    let lan = net::lan_ip();
    let mut session = world.resource_mut::<Session>();
    match made {
        Ok((server, transport, port)) => {
            info!("survivors: co-op: hosting on UDP port {port}");
            session.link = Link::Hosting { port };
            session.me = 0;
            session.status = format!("Hosting on port {port}: friends can join now");
            world.insert_resource(server);
            world.insert_resource(transport);
            world.resource_mut::<NetPanel>().lan = lan;
        }
        Err(e) => session.status = format!("Cannot host on port {port}: {e}"),
    }
}

/// Join the host at `text`.
pub(super) fn begin_joining(world: &mut World, text: String) {
    if world.resource::<Session>().link != Link::Off {
        return;
    }
    let made = net::parse_addr(&text).and_then(|addr| {
        net::join(world.resource::<RepliconChannels>(), addr)
            .map(|c| (addr, c))
            .map_err(|e| format!("Cannot join {text}: {e}"))
    });
    match made {
        Ok((addr, (client, transport))) => {
            info!("survivors: co-op: joining {addr}");
            world.insert_resource(client);
            world.insert_resource(transport);
            let mut session = world.resource_mut::<Session>();
            session.link = Link::Joining {
                addr: addr.to_string(),
            };
            session.attempt += 1;
            session.status = format!("Connecting to {addr}...");
        }
        Err(e) => world.resource_mut::<Session>().status = e,
    }
}

/// Leave whatever session this machine is in.
fn end_session(world: &mut World) {
    match world.resource::<Session>().link {
        Link::Off => {}
        Link::Hosting { .. } => {
            host::stop(world);
            world.resource_mut::<Session>().status = "Stopped hosting".into();
        }
        Link::Joining { .. } | Link::Joined { .. } => {
            client::leave(world, "You left the session".into());
        }
    }
}

/// The run begins as Enter World begins it, and the clients follow: the host's Ready Check once
/// everyone is ready, or its Start Anyway.
pub(super) fn start_now(
    mut run: ResMut<Run>,
    mut local: Query<&mut HeroState, With<LocalHero>>,
    mut selection: ResMut<Selection>,
    meta: Res<MetaBonuses>,
    unlocks: Res<Unlocks>,
    mut next: ResMut<NextState<Phase>>,
    mut session: ResMut<Session>,
) {
    if !unlocks.is_open(selection.map) {
        session.status = "That zone is not open yet: pick another".into();
        return;
    }
    if start_run(
        &mut run,
        local.single_mut().ok().as_deref_mut(),
        &mut selection,
        &meta,
    ) {
        next.set(Phase::Playing);
    }
}

/// Clicks on the panel's buttons and fields, typing into the focused field (Ctrl+V pastes),
/// Tab between fields, Enter to host or join from the field, Esc to close.
#[allow(clippy::too_many_arguments)]
fn panel_input(
    mut commands: Commands,
    clicks: Res<GlueClicks>,
    keys: Res<ButtonInput<KeyCode>>,
    mut typed: MessageReader<KeyboardInput>,
    phase: Res<State<Phase>>,
    actions: Query<(Entity, &NetAction, &GlueDisabled)>,
    fields: Query<(&FieldBox, &Interaction), Changed<Interaction>>,
    mut panel: ResMut<NetPanel>,
) {
    if *phase.get() != Phase::Menu {
        typed.clear();
        if panel.open {
            panel.open = false;
        }
        return;
    }
    if !panel.open {
        typed.clear();
    } else {
        if keys.just_pressed(KeyCode::Escape) {
            panel.open = false;
            typed.clear();
            return;
        }
        for (b, i) in &fields {
            if *i == Interaction::Pressed {
                panel.focus = b.0;
            }
        }
        let ctrl = keys.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]);
        for k in typed.read() {
            if k.state != ButtonState::Pressed {
                continue;
            }
            let focus = panel.focus;
            match &k.logical_key {
                Key::Tab => {
                    panel.focus = match focus {
                        Field::Port => Field::Addr,
                        Field::Addr => Field::Port,
                    }
                }
                Key::Backspace => {
                    text_of(&mut panel, focus).pop();
                }
                Key::Enter => match focus {
                    Field::Port => submit_host(&mut commands, &panel),
                    Field::Addr => submit_join(&mut commands, &panel),
                },
                Key::Character(c) if ctrl && c.eq_ignore_ascii_case("v") => {
                    if let Some(paste) = clipboard() {
                        type_into(text_of(&mut panel, focus), focus, &paste);
                    }
                }
                _ if !ctrl => {
                    if let Some(t) = &k.text {
                        type_into(text_of(&mut panel, focus), focus, t);
                    }
                }
                _ => {}
            }
        }
    }
    let Some(action) = actions
        .iter()
        .find(|(e, _, off)| !off.0 && clicks.hit(*e))
        .map(|(_, a, _)| *a)
    else {
        return;
    };
    match action {
        NetAction::Open => {
            panel.open = true;
            if panel.lan.is_none() {
                panel.lan = net::lan_ip();
            }
        }
        NetAction::Done => panel.open = false,
        NetAction::Host => submit_host(&mut commands, &panel),
        NetAction::Join => submit_join(&mut commands, &panel),
        NetAction::Leave => commands.queue(end_session),
        // The Ready Check: the run starts once everyone is ready ([`social::begin_check`]).
        NetAction::Start => commands.queue(social::begin_check),
        NetAction::Force => {
            panel.open = false;
            commands.queue(social::force_start);
        }
    }
}

fn text_of(panel: &mut NetPanel, field: Field) -> &mut String {
    match field {
        Field::Port => &mut panel.port,
        Field::Addr => &mut panel.addr,
    }
}

/// Typed text into a field: digits for the port, an address's characters for the address.
fn type_into(text: &mut String, field: Field, typed: &str) {
    for c in typed.chars() {
        let ok = match field {
            Field::Port => c.is_ascii_digit() && text.len() < 5,
            Field::Addr => {
                (c.is_ascii_alphanumeric() || matches!(c, '.' | ':' | '-' | '[' | ']'))
                    && text.len() < 64
            }
        };
        if ok {
            text.push(c);
        }
    }
}

fn submit_host(commands: &mut Commands, panel: &NetPanel) {
    match net::parse_port(&panel.port) {
        Ok(port) => commands.queue(move |world: &mut World| begin_hosting(world, port)),
        Err(e) => commands.queue(move |world: &mut World| {
            world.resource_mut::<Session>().status = e;
        }),
    }
}

fn submit_join(commands: &mut Commands, panel: &NetPanel) {
    let addr = panel.addr.clone();
    commands.queue(move |world: &mut World| begin_joining(world, addr));
}

fn clipboard() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

/// The lobby's line for `m`: its number, name, class and race, and what its card adds (level,
/// title, mount and hat).
fn member_line(m: &super::wire::Member, me: u8, extra: &str) -> String {
    let class = data::CLASSES
        .get(usize::from(m.class))
        .map_or("?", |c| c.name);
    let race = crate::char_select::race_name(m.race);
    let host = if m.player == 0 { " (host)" } else { "" };
    let you = if m.player == me { " (you)" } else { "" };
    format!(
        "{}. {}{host}{you}  -  {race} {class}{extra}",
        m.player + 1,
        m.name
    )
}

/// What the host tells its friends: where to reach it, on the LAN and over the internet.
fn host_lines(port: u16, lan: Option<IpAddr>) -> [String; 2] {
    let lan = lan.map_or_else(
        || "Your LAN address: unknown".to_string(),
        |ip| format!("Same network: friends join {ip}:{port}"),
    );
    [
        lan,
        format!("Internet: share your public IP and forward UDP {port}, or use Tailscale/ZeroTier"),
    ]
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
fn refresh_ui(
    panel: Res<NetPanel>,
    session: Res<Session>,
    name: Res<PlayerName>,
    mut modal: Query<(&mut Visibility, &mut Node), With<NetModal>>,
    mut buttons: Query<(&NetAction, &mut GlueDisabled, &mut Node), Without<NetModal>>,
    social_state: (Res<social::ReadyShown>, Res<social::BookShown>),
    mut boxes: Query<(&FieldBox, &mut BorderColor)>,
    mut texts: ParamSet<(
        Query<(&FieldText, &mut Text)>,
        Query<(&mut Text, &mut TextColor), With<StatusText>>,
        Query<(&LobbyLine, &mut Text), Without<StatusText>>,
        Query<(&mut Text, &mut TextColor), With<MenuLink>>,
    )>,
) {
    let (vis, display) = if panel.open {
        (Visibility::Inherited, Display::Flex)
    } else {
        (Visibility::Hidden, Display::None)
    };
    for (mut v, mut node) in &mut modal {
        if *v != vis {
            *v = vis;
        }
        if node.display != display {
            node.display = display;
        }
    }
    let off = session.link == Link::Off;
    let hosting = matches!(session.link, Link::Hosting { .. });
    let (ready, book) = social_state;
    let checking = ready.view.open;
    for (action, mut disabled, mut node) in &mut buttons {
        let want = match action {
            NetAction::Host | NetAction::Join => !off,
            NetAction::Leave => off,
            NetAction::Start => !hosting || session.in_run || checking,
            NetAction::Force => !hosting || session.in_run || !checking,
            NetAction::Open | NetAction::Done => false,
        };
        if disabled.0 != want {
            disabled.0 = want;
        }
        // Start Anyway shows only while the host's check runs.
        let display = if *action == NetAction::Force && !(hosting && checking) {
            Display::None
        } else {
            Display::Flex
        };
        if node.display != display {
            node.display = display;
        }
    }
    for (b, mut edge) in &mut boxes {
        let want = if b.0 == panel.focus && off {
            FIELD_HOT
        } else {
            FIELD_EDGE
        };
        if edge.top != want {
            *edge = BorderColor::all(want);
        }
    }
    let blink = panel.open && off;
    for (field, mut t) in &mut texts.p0() {
        let mut want = match field.0 {
            Field::Port => panel.port.clone(),
            Field::Addr => panel.addr.clone(),
        };
        if blink && field.0 == panel.focus {
            want.push('|');
        }
        if t.0 != want {
            t.0 = want;
        }
    }
    let status = if session.status.is_empty() {
        format!("You are {}", name.0)
    } else {
        session.status.clone()
    };
    let bad = status.starts_with("Cannot") || status.starts_with("Could not");
    for (mut t, mut c) in &mut texts.p1() {
        if t.0 != status {
            t.0 = status.clone();
        }
        let want = if bad { RED } else { Color::WHITE };
        if c.0 != want {
            c.0 = want;
        }
    }
    let info = match session.link {
        Link::Hosting { port } => host_lines(port, panel.lan),
        _ => [
            "Hosting listens on this UDP port; friends type your address to join.".into(),
            String::new(),
        ],
    };
    for (l, mut t) in &mut texts.p2() {
        let want = match l.0 {
            90 => info[0].clone(),
            91 => info[1].clone(),
            i => session
                .lobby
                .get(i)
                .map(|m| {
                    let extra = social::member_extra(book.card(m.player));
                    member_line(m, session.me, &extra)
                })
                .unwrap_or_default(),
        };
        if t.0 != want {
            t.0 = want;
        }
    }
    let menu = match &session.link {
        Link::Off => String::new(),
        Link::Hosting { .. } => format!("Hosting: {} in party", session.lobby.len().max(1)),
        Link::Joining { .. } => "Connecting...".into(),
        Link::Joined { .. } => format!("Joined: {} in party", session.lobby.len()),
    };
    for (mut t, _) in &mut texts.p3() {
        if t.0 != menu {
            t.0 = menu.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fields_take_only_what_they_can_hold() {
        let mut port = String::new();
        type_into(&mut port, Field::Port, "50a0x0123");
        assert_eq!(port, "50001");
        let mut addr = String::new();
        type_into(&mut addr, Field::Addr, " 100.64.0.7:5000\t/");
        assert_eq!(addr, "100.64.0.7:5000");
        let lines = host_lines(5000, Some("192.168.1.20".parse().unwrap()));
        assert!(lines[0].contains("192.168.1.20:5000"));
        assert!(lines[1].contains("Tailscale"));
    }
}
