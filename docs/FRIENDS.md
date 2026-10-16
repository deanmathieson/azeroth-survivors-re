# Launch night: Toast House plays Azeroth Survivors

One page, five steps, zero compiling. You need a Windows PC, about 6 GB of disk, and a friend
willing to host. Mounts not included.

## 1. Get the WoW 1.12.1 client data (start this first)

The kit has no Blizzard files in it. You need the game data from an English **WoW 1.12.1
(build 5875)** client, the same one the repo's `scripts/fetch-data.sh` uses:

- Page: <https://archive.org/details/WoW_1.12.1-5875>
- Direct download: <https://archive.org/download/WoW_1.12.1-5875/1.12.1-5875_enUS.zip> (~5.5 GB)

archive.org is often much slower than your line, so **start the download hours before launch
night**, not at 7:58pm. A download manager or `curl -L -C - -O <link>` lets it resume.

Unzip it and find the `Data` folder (it's inside the zip as `Data/`). You only need the `.MPQ` files
in it. A complete folder has these 14, about 5.1 GB in total:

| File | Size | File | Size |
| --- | --- | --- | --- |
| backup.MPQ | 6 MB | patch.MPQ | 1,961 MB |
| base.MPQ | 11 MB | patch-2.MPQ | 9 MB |
| dbc.MPQ | 4 MB | sound.MPQ | 675 MB |
| fonts.MPQ | 1 MB | speech.MPQ | 219 MB |
| interface.MPQ | 66 MB | terrain.MPQ | 1,049 MB |
| misc.MPQ | 6 MB | texture.MPQ | 634 MB |
| model.MPQ | 182 MB | wmo.MPQ | 347 MB |

If `patch.MPQ` or `terrain.MPQ` is missing or tiny, the download was cut short. Try again.
Already have a 1.12.1 client from back in the day? Its `Data` folder works too.

## 2. Unzip the kit

Download [AzerothSurvivors.zip](https://github.com/deanmathieson/azeroth-survivors-re/releases/latest/download/AzerothSurvivors.zip)
once and unzip it somewhere you can write to, such as Desktop or Documents (not Program Files).
Spaces in the path are fine. Inside you'll find:

- `Play.bat`: just play.
- `Host.bat`: play and host a party.
- `Join.bat`: play and join a party.
- `Update.bat`: check for an update without starting the game.
- `README-FRIENDS.txt`: this page, the short version.
- `benilla-config\`: your profile (settings, talents, achievements). It starts empty.

The first run asks where your `Data` folder is (paste the path) and remembers it in
`wow-data-path.txt`. Point it at the wrong place? Delete that file and run again. Each launcher
also checks for the latest kit automatically. It keeps your data path, last host, name files,
and `benilla-config` profile/saves.

## 3. Get on the same network: Tailscale

The easy way is [Tailscale](https://tailscale.com/download), a free VPN that makes four PCs act like
they're on one LAN. No router fiddling.

1. Everyone installs Tailscale and signs in.
2. The host either invites everyone to their tailnet, or shares their PC with each friend
   (admin console, the PC's `...` menu, **Share**). Friends accept the invite.
3. The host's address is the `100.x.y.z` one shown in the Tailscale tray menu (click it to copy).

**No Tailscale?** The host can forward **UDP port 5000** on their router to their PC and give
friends their public IP instead. It works, but it's a rogue-level stealth check on your router's
admin page.

## 4. Host and join

**Host:** double-click `Host.bat`, type your name, press Enter for port 5000. In the menu, pick
your class, race and zone. Open **Multiplayer** to watch the **Party** list fill up, then click
**Start Run**.

**Friends:** double-click `Join.bat`, type your name and the host's address, e.g.
`100.64.0.7:5000` (it remembers it next time). It joins as soon as the menu is up, bringing the
hero you last picked. Then wait in the lobby for the host to pull.

Want a different hero on your first night? Use `Play.bat` instead: pick your class and race, click
**Multiplayer**, type the host's address and click **Join**. The host can do the same with
**Host**.

## 5. Everyone on the same build

The game checks that everyone runs the same build. The launcher updates itself on startup, so if
you get “different build” while joining, everyone should close the game and restart their chosen
launcher. It will fetch the current release; no manual copying is needed.

## Troubleshooting

| Problem | Fix |
| --- | --- |
| "Windows protected your PC" / unrecognized app | Click **More info**, then **Run anyway**. It's unsigned, not cursed. |
| Firewall prompt the first time you host | Allow it on **Private networks**. Say no and nobody can join. |
| "different build" when joining | Everyone restarts their launcher to update, then retries. |
| Can't connect | Both of you check Tailscale is connected and you're joining the host's `100.x.y.z` address. Then the host checks the Windows firewall allows `warcraft-survivors.exe` (Windows Security, Firewall, Allow an app). |
| Asks for the Data folder every time | Unzip the kit somewhere writable, not Program Files. |
| The game closed with an error | Send the host the files in `benilla-config\Diagnostics`. |

Leeroy responsibly.
