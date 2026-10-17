//! Molten Core (Map.dbc 409) as a survivors raid map: the engine-spike checks behind
//! `docs/MC-SPIKE.md`. MC is a WMO-only map: its WDT authors no ADT tile and places one global
//! WMO, `blackrock_lower_guild.wmo`, which the tile streamer already registers like any building.
//! These tests pin what that WMO holds, and that every proposed stage centre (and every map-0
//! fallback centre) stands on walkable floor wide enough for its leash.

use std::collections::VecDeque;
use std::io::Cursor;

use benilla_formats::{
    load_map_catalog, load_tiles_around, load_wmo, load_wmo_collision_tris, open_chain,
    parse_wmo_root, wmo_group_header, wmo_group_liquid_mesh, Chain, GlobalWmo, LiquidKind,
    MapTiles, WdtReader, WowVersion,
};

const MC_WDT: &str = "World\\Maps\\MoltenCore\\MoltenCore.wdt";
const MC_WMO: &str = "world\\wmo\\dungeon\\az_blackrock\\blackrock_lower_guild.wmo";

/// A proposed arena: where the hero starts (raw WoW world coords) and how far he may walk.
struct Stage {
    name: &'static str,
    center: [f32; 3],
    leash: f32,
}

/// The MC stage centres `docs/MC-SPIKE.md` proposes. Ragnaros and Majordomo sit on their
/// remembered 1.12 spawns; the other rooms are the open floors the flood below finds, matched to
/// rooms by geography and still to be checked against the core's `creature` spawns.
const MC_STAGES: &[Stage] = &[
    Stage {
        name: "entrance (raid start)",
        center: [1080.0, -480.0, -108.2],
        leash: 14.0,
    },
    Stage {
        name: "Lucifron's cavern (T1/B1)",
        center: [1096.0, -732.0, -149.1],
        leash: 45.0,
    },
    Stage {
        name: "Magmadar's lair (T2/B2)",
        center: [1112.0, -1008.0, -187.3],
        leash: 24.0,
    },
    Stage {
        name: "south-centre cavern (Golemagg?)",
        center: [868.0, -1008.0, -196.4],
        leash: 34.0,
    },
    Stage {
        name: "west cavern (Garr?)",
        center: [732.0, -520.0, -217.0],
        leash: 40.0,
    },
    Stage {
        name: "north-west cavern (Geddon/Shazzrah?)",
        center: [644.0, -816.0, -208.5],
        leash: 45.0,
    },
    Stage {
        name: "centre cavern (Sulfuron/Gehennas?)",
        center: [892.0, -600.0, -203.6],
        leash: 34.0,
    },
    Stage {
        name: "Majordomo's camp (B9)",
        center: [736.0, -1168.0, -120.4],
        leash: 30.0,
    },
    Stage {
        name: "Ragnaros's lair, the dry ledge north of his pool (B10)",
        center: [844.0, -772.0, -226.0],
        leash: 16.0,
    },
];

/// Ragnaros's lair walked lava and all ([`Grid::wade`]): the ring and the lava channels read as
/// one arena once magma is a hazard rather than a hole, his 1.12 spawn (838.5, -829.7) 25 yd from
/// its centre.
const RAGNAROS_WADING: Stage = Stage {
    name: "Ragnaros's lair, lava walkable (B10)",
    center: [828.0, -852.0, -229.0],
    leash: 30.0,
};

/// Map-0 fallback arenas, on ADT terrain plus every WMO the tiles place.
const MAP0_STAGES: &[Stage] = &[
    Stage {
        name: "Burning Steppes, lava field below Blackrock Stronghold",
        center: [-7692.0, -1404.0, 133.9],
        leash: 36.0,
    },
    Stage {
        name: "Burning Steppes, open field below Blackrock Stronghold",
        center: [-7704.0, -1464.0, 133.0],
        leash: 50.0,
    },
    Stage {
        name: "Searing Gorge, the Cauldron's floor by its lava",
        center: [-6724.0, -1368.0, 181.8],
        leash: 18.0,
    },
    Stage {
        name: "Searing Gorge, the plateau east of the Cauldron",
        center: [-6758.0, -1224.0, 247.0],
        leash: 80.0,
    },
];

/// Inside Blackrock Mountain (`blackrock.wmo` alone, ceiling required).
const BRM_STAGES: &[Stage] = &[
    Stage {
        name: "Blackrock Mountain, the floor below the lava hall",
        center: [-7355.0, -1000.0, 173.0],
        leash: 24.0,
    },
    Stage {
        name: "Blackrock Mountain, the chained lava hall",
        center: [-7595.0, -1012.0, 244.0],
        leash: 14.0,
    },
];

const CELL: f32 = 2.0;

/// Per-cell floor, magma and ceiling heights over a world-space box, rasterised from triangles.
struct Grid {
    x0: f32,
    y0: f32,
    nx: usize,
    ny: usize,
    floors: Vec<Vec<f32>>,
    lava: Vec<Vec<f32>>,
    ceilings: Vec<Vec<f32>>,
    /// Outdoors a floor needs no ceiling; indoors one does, which drops a cavern shell's top.
    outdoors: bool,
    /// Walk the magma too, as a hazard rather than a hole: its surface counts as floor.
    wade: bool,
}

impl Grid {
    fn new(min: [f32; 2], max: [f32; 2], outdoors: bool) -> Self {
        let nx = ((max[0] - min[0]) / CELL).ceil() as usize + 1;
        let ny = ((max[1] - min[1]) / CELL).ceil() as usize + 1;
        Grid {
            x0: min[0],
            y0: min[1],
            nx,
            ny,
            floors: vec![Vec::new(); nx * ny],
            lava: vec![Vec::new(); nx * ny],
            ceilings: vec![Vec::new(); nx * ny],
            outdoors,
            wade: false,
        }
    }

    fn idx(&self, i: i64, j: i64) -> Option<usize> {
        (i >= 0 && j >= 0 && (i as usize) < self.nx && (j as usize) < self.ny)
            .then(|| j as usize * self.nx + i as usize)
    }

    fn centre(&self, i: usize, j: usize) -> [f32; 2] {
        [self.x0 + i as f32 * CELL, self.y0 + j as f32 * CELL]
    }

    fn cell_of(&self, x: f32, y: f32) -> (i64, i64) {
        (
            ((x - self.x0) / CELL).round() as i64,
            ((y - self.y0) / CELL).round() as i64,
        )
    }

    /// Rasterise one world-space triangle: magma, a walkable floor (within 50° of up, the
    /// mover's `GROUND_COS`), a ceiling (facing down), or nothing (a wall).
    fn add(&mut self, t: [[f32; 3]; 3], lava: bool) {
        let n = cross(sub(t[1], t[0]), sub(t[2], t[0]));
        let len = (n[0] * n[0] + n[1] * n[1] + n[2] * n[2]).sqrt();
        if len == 0.0 {
            return;
        }
        let up = n[2] / len;
        let ceiling = !lava && up < -0.2;
        if !lava && !ceiling && up < 0.642_788 {
            return;
        }
        let (i0, j0) = self.cell_of(
            t[0][0].min(t[1][0]).min(t[2][0]),
            t[0][1].min(t[1][1]).min(t[2][1]),
        );
        let (i1, j1) = self.cell_of(
            t[0][0].max(t[1][0]).max(t[2][0]),
            t[0][1].max(t[1][1]).max(t[2][1]),
        );
        for j in j0..=j1 {
            for i in i0..=i1 {
                let Some(k) = self.idx(i, j) else { continue };
                let [x, y] = self.centre(i as usize, j as usize);
                if let Some(z) = z_in(&t, x, y) {
                    let into = if ceiling {
                        &mut self.ceilings[k]
                    } else if lava {
                        &mut self.lava[k]
                    } else {
                        &mut self.floors[k]
                    };
                    into.push(z);
                }
            }
        }
    }

    /// The floor in cell `k` nearest `z` within `tol` that no magma covers (or, wading, the
    /// magma's surface) and, indoors, that has a ceiling over it.
    fn dry_floor(&self, k: usize, z: f32, tol: f32) -> Option<f32> {
        let roofed = |f: f32| self.outdoors || self.ceilings[k].iter().any(|&c| c > f + 2.0);
        let dry = self.floors[k]
            .iter()
            .copied()
            .filter(|&f| !self.lava[k].iter().any(|&l| l > f - 0.5 && l < f + 30.0));
        let wet = self.lava[k].iter().copied().filter(|_| self.wade);
        dry.chain(wet)
            .filter(|&f| (f - z).abs() <= tol && roofed(f))
            .min_by(|a, b| (a - z).abs().total_cmp(&(b - z).abs()))
    }

    /// The lowest ceiling over the floor at `z` in the cell under `(x, y)`.
    fn clearance(&self, x: f32, y: f32, z: f32) -> Option<f32> {
        let (i, j) = self.cell_of(x, y);
        let k = self.idx(i, j)?;
        self.ceilings[k]
            .iter()
            .filter(|&&c| c > z + 2.0)
            .map(|&c| c - z)
            .min_by(f32::total_cmp)
    }

    /// The largest disc (yd) about `(x, y)` that a walk from the floor at about `z` covers: a
    /// 4-way flood over dry floors, at most 2 yd up or down per 2 yd step, so cliffs and pits
    /// stop it where a plain height test would not.
    fn open_radius(&self, x: f32, y: f32, z: f32, offsets: &[(i64, i64, f32)]) -> f32 {
        let (ci, cj) = self.cell_of(x, y);
        let max = offsets.last().map_or(0.0, |o| o.2);
        let r = (max / CELL).ceil() as i64;
        let side = (2 * r + 1) as usize;
        let slot = |di: i64, dj: i64| ((dj + r) as usize) * side + (di + r) as usize;
        let mut reached = vec![false; side * side];
        let Some(z0) = self.idx(ci, cj).and_then(|k| self.dry_floor(k, z, 1.0)) else {
            return 0.0;
        };
        reached[slot(0, 0)] = true;
        let mut queue = VecDeque::from([(0i64, 0i64, z0)]);
        while let Some((di, dj, fz)) = queue.pop_front() {
            for (si, sj) in [(1, 0), (-1, 0), (0, 1), (0, -1)] {
                let (ni, nj) = (di + si, dj + sj);
                if ni.abs() > r || nj.abs() > r || reached[slot(ni, nj)] {
                    continue;
                }
                let Some(k) = self.idx(ci + ni, cj + nj) else {
                    continue;
                };
                if let Some(nz) = self.dry_floor(k, fz, 2.0) {
                    reached[slot(ni, nj)] = true;
                    queue.push_back((ni, nj, nz));
                }
            }
        }
        offsets
            .iter()
            .find(|&&(di, dj, _)| !reached[slot(di, dj)])
            .map_or(max, |o| o.2)
    }

    /// Every disc of at least `min_r`, largest first, each hiding the centres inside it.
    fn best_discs(&self, min_r: f32, max_r: f32, stride: usize) -> Vec<([f32; 3], f32)> {
        let off = offsets(max_r);
        let mut found = Vec::new();
        for j in (0..self.ny).step_by(stride) {
            for i in (0..self.nx).step_by(stride) {
                let k = j * self.nx + i;
                let [x, y] = self.centre(i, j);
                let wet = self.lava[k].iter().filter(|_| self.wade);
                for &z in self.floors[k].iter().chain(wet) {
                    if self.dry_floor(k, z, 0.0).is_none() {
                        continue;
                    }
                    let r = self.open_radius(x, y, z, &off);
                    if r >= min_r {
                        found.push(([x, y, z], r));
                    }
                }
            }
        }
        found.sort_by(|a, b| b.1.total_cmp(&a.1));
        let mut kept: Vec<([f32; 3], f32)> = Vec::new();
        for (p, r) in found {
            if kept.iter().all(|(q, s)| {
                let d = ((p[0] - q[0]).powi(2) + (p[1] - q[1]).powi(2)).sqrt();
                d > s.max(r) || (p[2] - q[2]).abs() > 15.0
            }) {
                kept.push((p, r));
            }
        }
        kept
    }
}

fn sub(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [a[0] - b[0], a[1] - b[1], a[2] - b[2]]
}

fn cross(a: [f32; 3], b: [f32; 3]) -> [f32; 3] {
    [
        a[1] * b[2] - a[2] * b[1],
        a[2] * b[0] - a[0] * b[2],
        a[0] * b[1] - a[1] * b[0],
    ]
}

/// The triangle's height over the column `(x, y)`, if the column passes through it.
fn z_in(t: &[[f32; 3]; 3], x: f32, y: f32) -> Option<f32> {
    let (a, b, c) = (t[0], t[1], t[2]);
    let d = (b[1] - c[1]) * (a[0] - c[0]) + (c[0] - b[0]) * (a[1] - c[1]);
    if d.abs() < 1e-9 {
        return None;
    }
    let u = ((b[1] - c[1]) * (x - c[0]) + (c[0] - b[0]) * (y - c[1])) / d;
    let v = ((c[1] - a[1]) * (x - c[0]) + (a[0] - c[0]) * (y - c[1])) / d;
    let w = 1.0 - u - v;
    (u >= -1e-4 && v >= -1e-4 && w >= -1e-4).then(|| u * a[2] + v * b[2] + w * c[2])
}

/// Disc offsets in cells, nearest first, out to `max` yd.
fn offsets(max: f32) -> Vec<(i64, i64, f32)> {
    let r = (max / CELL).ceil() as i64;
    let mut out = Vec::new();
    for dj in -r..=r {
        for di in -r..=r {
            let d = ((di * di + dj * dj) as f32).sqrt() * CELL;
            if d <= max {
                out.push((di, dj, d));
            }
        }
    }
    out.sort_by(|a, b| a.2.total_cmp(&b.2));
    out
}

/// A MODF placement, model space to WoW world: `benilla_assets::placement_rotation` before its
/// Bevy basis change, `Rx(90°)·Ry(ry−180°)·Rz(−rx)·Rx(rz−90°)`, then the position. At rotation 0,
/// as MC's global WMO has, it is a half-turn about up: world = (−x, −y, z).
fn placement(rot: [f32; 3], pos: [f32; 3]) -> impl Fn([f32; 3]) -> [f32; 3] {
    use std::f32::consts::{FRAC_PI_2, PI};
    type M = [[f32; 3]; 3];
    fn mul(a: M, b: M) -> M {
        let mut o = [[0.0; 3]; 3];
        for (i, row) in o.iter_mut().enumerate() {
            for (j, v) in row.iter_mut().enumerate() {
                *v = (0..3).map(|k| a[i][k] * b[k][j]).sum();
            }
        }
        o
    }
    fn rx(t: f32) -> M {
        let (s, c) = t.sin_cos();
        [[1.0, 0.0, 0.0], [0.0, c, -s], [0.0, s, c]]
    }
    fn ry(t: f32) -> M {
        let (s, c) = t.sin_cos();
        [[c, 0.0, s], [0.0, 1.0, 0.0], [-s, 0.0, c]]
    }
    fn rz(t: f32) -> M {
        let (s, c) = t.sin_cos();
        [[c, -s, 0.0], [s, c, 0.0], [0.0, 0.0, 1.0]]
    }
    let [a, b, c] = rot.map(f32::to_radians);
    let m = mul(
        mul(mul(rx(FRAC_PI_2), ry(b - PI)), rz(-a)),
        rx(c - FRAC_PI_2),
    );
    move |p| {
        let mut o = pos;
        for (i, oi) in o.iter_mut().enumerate() {
            *oi += m[i][0] * p[0] + m[i][1] * p[1] + m[i][2] * p[2];
        }
        o
    }
}

/// Add a placed WMO's walking-collision faces and its groups' magma to `grid`.
fn add_wmo(chain: &mut Chain, grid: &mut Grid, model: &str, rot: [f32; 3], pos: [f32; 3]) {
    let xf = placement(rot, pos);
    let col = load_wmo_collision_tris(chain, model).expect("WMO collision");
    for t in col.indices.as_chunks::<3>().0 {
        grid.add([0, 1, 2].map(|i| xf(col.positions[t[i] as usize])), false);
    }
    let path = model.to_ascii_lowercase();
    let root = parse_wmo_root(&chain.read_file(&path).expect("WMO root")).expect("WMO root");
    let stem = path.strip_suffix(".wmo").unwrap();
    for gi in 0..root.group_count() {
        let Ok(gb) = chain.read_file(&format!("{stem}_{gi:03}.wmo")) else {
            continue;
        };
        if let Some(m) = wmo_group_liquid_mesh(&gb).filter(|m| m.kind == LiquidKind::Magma) {
            for t in m.indices.as_chunks::<3>().0 {
                grid.add([0, 1, 2].map(|i| xf(m.positions[t[i] as usize])), true);
            }
        }
    }
}

/// The ADT terrain, its magma, and every WMO the tiles place, within one tile of `(x, y)`; with
/// `wmo_filter`, only the WMOs whose path ends so, and no terrain.
fn add_map0(chain: &mut Chain, grid: &mut Grid, x: f32, y: f32, wmo_filter: Option<&str>) {
    let tiles = load_tiles_around(chain, "Azeroth", x, y, 1).expect("Azeroth tiles");
    let mut wmos = std::collections::BTreeMap::new();
    for (_, t) in &tiles {
        if wmo_filter.is_none() {
            for ch in &t.chunks {
                for tri in ch.indices.as_chunks::<3>().0 {
                    grid.add([0, 1, 2].map(|i| ch.positions[tri[i] as usize]), false);
                }
                for l in ch.liquids.iter().filter(|l| l.kind == LiquidKind::Magma) {
                    for tri in l.indices.as_chunks::<3>().0 {
                        grid.add([0, 1, 2].map(|i| l.positions[tri[i] as usize]), true);
                    }
                }
            }
        }
        for w in &t.wmos {
            wmos.insert(w.unique_id, w.clone());
        }
    }
    for w in wmos.values() {
        if wmo_filter.is_none_or(|f| w.model.to_ascii_lowercase().ends_with(f)) {
            add_wmo(chain, grid, &w.model, w.rotation, w.position);
        }
    }
}

fn mc_wdt(chain: &mut Chain) -> (Vec<u8>, benilla_formats::WdtFile) {
    let bytes = chain.read_file(MC_WDT).expect("MoltenCore.wdt");
    let wdt = WdtReader::new(Cursor::new(bytes.clone()), WowVersion::Classic)
        .read()
        .expect("parse MoltenCore.wdt");
    (bytes, wdt)
}

fn mc_global_wmo(chain: &mut Chain) -> GlobalWmo {
    mc_wdt(chain).1.global_wmo().expect("global WMO").clone()
}

fn mc_grid(chain: &mut Chain) -> Grid {
    let g = mc_global_wmo(chain);
    let mut grid = Grid::new([400.0, -1300.0], [1400.0, -200.0], false);
    add_wmo(chain, &mut grid, &g.model, g.rotation, g.position);
    grid
}

/// The first dword of the WDT's `MPHD`, the header flags.
fn mphd_flags(wdt: &[u8]) -> Option<u32> {
    let mut off = 0;
    while off + 8 <= wdt.len() {
        let size = u32::from_le_bytes(wdt[off + 4..off + 8].try_into().unwrap()) as usize;
        if &wdt[off..off + 4] == b"DHPM" {
            return Some(u32::from_le_bytes(
                wdt[off + 8..off + 12].try_into().unwrap(),
            ));
        }
        off += 8 + size;
    }
    None
}

/// (a) Map 409 is the raid "Molten Core" in `MoltenCore\`; its WDT authors no tile and places one
/// global WMO, `blackrock_lower_guild.wmo`, at the origin, unrotated (MPHD flag bit 0).
#[test]
fn molten_core_is_one_global_wmo_and_no_terrain() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");

    let maps = load_map_catalog(&mut chain).expect("Map.dbc");
    assert_eq!(maps.directory(409), Some("MoltenCore"));
    assert_eq!(maps.name(409), Some("Molten Core"));
    assert_eq!(
        maps.instance_type(409),
        Some(2),
        "instance type 2 is a raid"
    );

    let (bytes, wdt) = mc_wdt(&mut chain);
    let flags = mphd_flags(&bytes);
    println!("MoltenCore.wdt MPHD flags: {flags:x?}");
    assert_eq!(flags.map(|f| f & 0x1), Some(0x1), "MPHD bit 0: WMO-only");
    let tiles = (0..64)
        .flat_map(|y| (0..64).map(move |x| (x, y)))
        .filter(|&(x, y)| wdt.get_tile(x, y).is_some_and(|t| t.has_adt))
        .count();
    assert_eq!(tiles, 0, "MC authors no ADT tile");

    let g = wdt.global_wmo().expect("MC is a WMO-only map");
    assert_eq!(g.model.to_ascii_lowercase(), MC_WMO);
    assert_eq!(g.position, [0.0, 0.0, 0.0]);
    assert_eq!(g.rotation, [0.0, 0.0, 0.0]);
    assert_eq!((g.doodad_set, g.name_set), (0, 0));

    // The placement is a half-turn about up, so the server's MC entrance (the areatrigger lands
    // at 1096, -467, -104.6) falls inside group 0's box, mirrored: x -1333..-1080, y 271..559.
    let world = placement(g.rotation, g.position)([-1096.0, 467.0, -104.6]);
    assert!((world[0] - 1096.0).abs() < 1e-3 && (world[1] + 467.0).abs() < 1e-3);
}

/// (b) The WMO: 12 interior groups, 5 of them with magma; every render texture and every doodad
/// model is in the chain.
#[test]
fn molten_core_wmo_groups_load_and_count() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");

    let root_bytes = chain.read_file(MC_WMO).expect("MC root");
    let root = parse_wmo_root(&root_bytes).expect("parse MC root");
    assert_eq!(root.group_count(), 12);
    assert!(
        root.group_infos().iter().all(|g| g.interior),
        "MC is all interior"
    );
    assert_eq!(root.portals().refs.len(), 24);
    assert_eq!(root.fogs().len(), 10);
    assert_eq!(root.doodads().len(), 252);
    assert_eq!(root.doodad_sets().len(), 1);
    assert_eq!(root.skybox(), None);

    let stem = MC_WMO.strip_suffix(".wmo").unwrap();
    let mut magma = Vec::new();
    for gi in 0..root.group_count() {
        let gb = chain
            .read_file(&format!("{stem}_{gi:03}.wmo"))
            .unwrap_or_else(|e| panic!("group {gi}: {e}"));
        assert!(wmo_group_header(&gb).is_some(), "group {gi} header");
        if let Some(m) = wmo_group_liquid_mesh(&gb) {
            assert_eq!(m.kind, LiquidKind::Magma, "group {gi}'s liquid is lava");
            magma.push(gi);
        }
    }
    assert_eq!(magma, [0, 3, 6, 9, 10], "the lava-carrying groups");

    let subs = load_wmo(&mut chain, MC_WMO).expect("load MC render mesh");
    let render_tris: usize = subs.iter().map(|s| s.indices.len() / 3).sum();
    let collision = load_wmo_collision_tris(&mut chain, MC_WMO).expect("MC collision");
    println!(
        "MC: {} submeshes, {render_tris} render triangles, {} collision triangles",
        subs.len(),
        collision.triangle_count()
    );
    assert_eq!(collision.triangle_count(), 15_735);

    let mut missing: Vec<&str> = subs
        .iter()
        .filter_map(|s| s.texture.as_deref())
        .filter(|t| chain.read_file(t).is_err())
        .collect();
    missing.sort_unstable();
    missing.dedup();
    assert!(
        missing.is_empty(),
        "textures missing from the chain: {missing:?}"
    );

    let mut absent: Vec<String> = root
        .doodads()
        .iter()
        .map(|d| {
            let m = d.model.to_ascii_lowercase();
            m.strip_suffix(".mdx")
                .or_else(|| m.strip_suffix(".mdl"))
                .map_or(m.clone(), |s| format!("{s}.m2"))
        })
        .filter(|m| chain.read_file(m).is_err())
        .collect();
    absent.sort_unstable();
    absent.dedup();
    assert!(absent.is_empty(), "doodad models missing: {absent:?}");
}

/// (c) The ADT loaders answer a WMO-only map as they should: `MapTiles` parses its WDT and finds
/// no tile anywhere, and `load_tiles_around` bails. The streamer's own WDT branch is what turns
/// the global WMO into a placement (`benilla-world`'s `terrain_stream`), so there is no
/// map-by-name tile load to make work here.
#[test]
fn the_tile_loaders_find_no_terrain_on_molten_core() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");

    let tiles = MapTiles::load(&mut chain, "MoltenCore").expect("MapTiles parses MC's WDT");
    assert!(tiles.existing_in_radius(1096.0, -467.0, 64).is_empty());

    let err = load_tiles_around(&mut chain, "MoltenCore", 1096.0, -467.0, 1)
        .expect_err("no terrain to load")
        .to_string();
    assert!(err.contains("no loadable tiles"), "{err}");
}

/// (d) Every proposed MC stage stands on a walkable, ceilinged floor at its height, with a flood
/// of walkable floor at least its leash wide all round.
#[test]
fn molten_core_stage_centres_are_open_floor() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");
    let mut grid = mc_grid(&mut chain);
    check_stages(&grid, MC_STAGES);
    grid.wade = true;
    check_stages(&grid, &[RAGNAROS_WADING]);
}

/// (e) The map-0 fallback arenas, on terrain plus placed WMOs, and inside Blackrock Mountain.
#[test]
fn map0_fallback_centres_are_open_floor() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");
    for s in MAP0_STAGES {
        let [x, y, _] = s.center;
        let mut grid = Grid::new([x - 150.0, y - 150.0], [x + 150.0, y + 150.0], true);
        add_map0(&mut chain, &mut grid, x, y, None);
        check_stages(&grid, std::slice::from_ref(s));
    }
    let [x, y, _] = BRM_STAGES[0].center;
    let mut grid = Grid::new([x - 400.0, y - 400.0], [x + 400.0, y + 400.0], false);
    add_map0(
        &mut chain,
        &mut grid,
        x,
        y,
        Some("\\az_blackrock\\blackrock.wmo"),
    );
    check_stages(&grid, BRM_STAGES);
}

fn check_stages(grid: &Grid, stages: &[Stage]) {
    let off = offsets(90.0);
    let mut bad = Vec::new();
    for s in stages {
        let [x, y, z] = s.center;
        let r = grid.open_radius(x, y, z, &off);
        let clear = grid.clearance(x, y, z);
        println!(
            "{:<55} {:?} open r={r:.0} leash={} ceiling clearance={clear:?}",
            s.name, s.center, s.leash
        );
        if r < s.leash {
            bad.push(format!("{}: open {r:.0} < leash {}", s.name, s.leash));
        }
    }
    assert!(bad.is_empty(), "{bad:#?}");
}

/// Not a check: lists MC's open floors, largest first, for picking room centres.
/// `cargo test -p benilla-formats --test molten_core -- --ignored --nocapture`
#[test]
#[ignore = "a survey, not a check"]
fn survey_molten_core_open_floors() {
    let data = benilla_formats::wow_data_or_skip!();
    let mut chain = open_chain(&data).expect("open chain");
    let mut grid = mc_grid(&mut chain);
    // `MC_WADE=1` walks the magma as floor.
    grid.wade = std::env::var("MC_WADE").is_ok();
    // `MC_NEAR=x,y,radius` keeps the floors near one spot, smaller ones included.
    let near: Option<Vec<f32>> = std::env::var("MC_NEAR")
        .ok()
        .map(|v| v.split(',').map(|s| s.trim().parse().unwrap()).collect());
    let min_r = if near.is_some() { 6.0 } else { 18.0 };
    let discs = grid.best_discs(min_r, 70.0, 2);
    let shown = discs.iter().filter(|(p, _)| {
        near.as_ref()
            .is_none_or(|n| (p[0] - n[0]).hypot(p[1] - n[1]) <= n[2])
    });
    for (p, r) in shown.take(40) {
        println!(
            "{:?} open r={r:.0} clearance={:?}",
            p.map(f32::round),
            grid.clearance(p[0], p[1], p[2]).map(f32::round)
        );
    }
}
