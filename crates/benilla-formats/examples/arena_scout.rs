//! Score candidate survivor-arena centres: flat open ground, no building or big doodad within
//! reach, no liquid, not impassable. Reads candidates from stdin: `name map x y` per line; prints
//! the best `name x y z score` for each. Output is derived from Blizzard data: never commit it.
//! `cargo run -p benilla-formats --example arena_scout < candidates.txt`

use std::io::BufRead;

/// A candidate arena: name, map and its candidate `(x, y)` centres.
type Group = (String, String, Vec<(f32, f32)>);

fn main() -> anyhow::Result<()> {
    let data = benilla_formats::wow_data().expect("no WoW install found (set $WOW_DATA)");
    let mut chain = benilla_formats::open_chain(&data)?;
    let mut tiles_by_map = std::collections::HashMap::new();
    let mut mesh_cache: std::collections::HashMap<(String, u32, u32), benilla_formats::TileMesh> =
        std::collections::HashMap::new();

    let mut groups: Vec<Group> = Vec::new();
    for line in std::io::stdin().lock().lines() {
        let line = line?;
        let p: Vec<&str> = line.split_whitespace().collect();
        if p.len() < 4 {
            continue;
        }
        let (name, map) = (p[0].to_string(), p[1].to_string());
        let (x, y): (f32, f32) = (p[2].parse()?, p[3].parse()?);
        match groups.last_mut() {
            Some(g) if g.0 == name => g.2.push((x, y)),
            _ => groups.push((name, map, vec![(x, y)])),
        }
    }

    for (name, map, cands) in groups {
        if !tiles_by_map.contains_key(&map) {
            tiles_by_map.insert(
                map.clone(),
                benilla_formats::MapTiles::load(&mut chain, &map)?,
            );
        }
        let tiles = &tiles_by_map[&map];
        let mut best: Option<(f32, f32, f32, f32, String)> = None;
        for (x, y) in cands {
            // Every tile within 160 yd.
            let mut meshes = Vec::new();
            for (tx, ty) in tiles.existing_in_radius(x, y, 1) {
                let key = (map.clone(), tx, ty);
                if !mesh_cache.contains_key(&key) {
                    let m = benilla_formats::load_tile_mesh(&mut chain, &map, tx, ty)?;
                    mesh_cache.insert(key.clone(), m);
                }
                meshes.push(key);
            }
            let chunks: Vec<&benilla_formats::ChunkMesh> = meshes
                .iter()
                .flat_map(|k| mesh_cache[k].chunks.iter())
                .collect();
            let height = |px: f32, py: f32| chunks.iter().find_map(|c| c.height_at([px, py, 0.0]));
            let Some(z0) = height(x, y) else { continue };
            // Slope over a 40 yd disc, sampled every 8 yd: spread of heights.
            let mut zs = Vec::new();
            let mut r = 8.0;
            while r <= 40.0 {
                for k in 0..12 {
                    let a = k as f32 * std::f32::consts::TAU / 12.0;
                    if let Some(z) = height(x + r * a.cos(), y + r * a.sin()) {
                        zs.push(z);
                    }
                }
                r += 8.0;
            }
            let spread = zs.iter().fold(0.0f32, |m, z| m.max((z - z0).abs()));
            // Liquid or impassable under the disc.
            let mut wet = 0;
            let mut walls = 0;
            for c in &chunks {
                let nw = c.positions[0];
                let in_disc = (nw[0] - 16.0 - x).hypot(nw[1] - 16.0 - y) < 60.0;
                if in_disc {
                    if c.impassable {
                        walls += 1;
                    }
                    for l in &c.liquids {
                        if l.wet.iter().any(|w| *w) {
                            wet += 1;
                        }
                    }
                }
            }
            // Buildings within 70 yd, and doodads (scaled) within 18 yd.
            let mut wmo_d = f32::INFINITY;
            let mut props = 0;
            for k in &meshes {
                let t = &mesh_cache[k];
                for w in &t.wmos {
                    wmo_d = wmo_d.min((w.position[0] - x).hypot(w.position[1] - y));
                }
                for d in &t.doodads {
                    let dd = (d.position[0] - x).hypot(d.position[1] - y);
                    let m = d.model.to_ascii_lowercase();
                    let tree = m.contains("tree") || m.contains("rock") || m.contains("bush");
                    if dd < 18.0 && !m.contains("grass") && !m.contains("flower") {
                        props += if tree { 2 } else { 1 };
                    }
                }
            }
            let score = spread * 3.0
                + wet as f32 * 40.0
                + walls as f32 * 60.0
                + props as f32 * 4.0
                + if wmo_d < 70.0 {
                    (70.0 - wmo_d) * 2.0
                } else {
                    0.0
                };
            let why =
                format!("spread={spread:.1} wet={wet} walls={walls} props={props} wmo={wmo_d:.0}");
            if best.as_ref().is_none_or(|b| score < b.3) {
                best = Some((x, y, z0, score, why));
            }
        }
        match best {
            Some((x, y, z, s, why)) => println!("{name} {x:.1} {y:.1} {z:.1} {s:.1} {why}"),
            None => println!("{name} none"),
        }
    }
    Ok(())
}
