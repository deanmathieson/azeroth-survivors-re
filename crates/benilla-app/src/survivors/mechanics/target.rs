//! Where a mechanic lands: pure targeting over a snapshot of the horde, every distance on the
//! ground plane as the mode measures range. Results are indices into the snapshot.

use bevy::prelude::*;

use super::super::units::flat;
use super::super::Rng;

/// One living enemy as a mechanic sees it.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Foe {
    pub e: Entity,
    pub pos: Vec3,
    pub hp: f32,
    pub max_hp: f32,
    pub radius: f32,
    pub boss: bool,
}

fn dist(a: Vec3, b: Vec3) -> f32 {
    flat(a - b).length()
}

fn by_distance(foes: &[Foe], at: Vec3, mut picked: Vec<usize>) -> Vec<usize> {
    picked.sort_by(|&a, &b| dist(foes[a].pos, at).total_cmp(&dist(foes[b].pos, at)));
    picked
}

/// Every foe within `radius` of `center`, half its body counting, as the abilities' areas reach.
pub(crate) fn around(foes: &[Foe], center: Vec3, radius: f32) -> Vec<usize> {
    (0..foes.len())
        .filter(|&i| dist(foes[i].pos, center) <= radius + foes[i].radius * 0.5)
        .collect()
}

/// The `n` nearest foes to `at` within `radius`, other than `not`, nearest first: a split's shards
/// and a spread's landings.
pub(crate) fn nearest_n(
    foes: &[Foe],
    at: Vec3,
    radius: f32,
    n: usize,
    not: Option<Entity>,
) -> Vec<usize> {
    let near = (0..foes.len())
        .filter(|&i| Some(foes[i].e) != not && dist(foes[i].pos, at) <= radius)
        .collect();
    let mut out = by_distance(foes, at, near);
    out.truncate(n);
    out
}

/// The nearest foe to `at` within `reach`, other than `not`.
pub(crate) fn nearest(foes: &[Foe], at: Vec3, reach: f32, not: Option<Entity>) -> Option<usize> {
    nearest_n(foes, at, reach, 1, not).first().copied()
}

/// A chain's path from `from` standing at `start`: each jump to the nearest foe not yet struck
/// within `reach` of the last, `jumps` at most.
pub(crate) fn chain(
    foes: &[Foe],
    from: Option<Entity>,
    start: Vec3,
    jumps: usize,
    reach: f32,
) -> Vec<usize> {
    let mut path: Vec<usize> = Vec::with_capacity(jumps);
    let mut last = start;
    while path.len() < jumps {
        let next = (0..foes.len())
            .filter(|&i| Some(foes[i].e) != from && !path.contains(&i))
            .filter(|&i| dist(foes[i].pos, last) <= reach)
            .min_by(|&a, &b| dist(foes[a].pos, last).total_cmp(&dist(foes[b].pos, last)));
        match next {
            Some(i) => {
                last = foes[i].pos;
                path.push(i);
            }
            None => break,
        }
    }
    path
}

/// Where a point lies along the line from `a` toward `b`: its distance along it, and its distance
/// off it.
fn along(a: Vec3, b: Vec3, p: Vec3) -> (f32, f32) {
    let dir = flat(b - a).normalize_or_zero();
    let rel = flat(p - a);
    let t = rel.dot(dir);
    (t, (rel - dir * t).length())
}

/// What a blow from `origin` passing through `through` goes on to strike: up to `n` foes within
/// `width` of the line, up to `length` past `through`, nearest first; never `not`.
pub(crate) fn pierce(
    foes: &[Foe],
    origin: Vec3,
    through: Vec3,
    not: Option<Entity>,
    n: usize,
    width: f32,
    length: f32,
) -> Vec<usize> {
    let reach = dist(through, origin);
    if reach < 1e-3 {
        return Vec::new();
    }
    let mut hit: Vec<(usize, f32)> = (0..foes.len())
        .filter(|&i| Some(foes[i].e) != not)
        .filter_map(|i| {
            let (t, off) = along(origin, through, foes[i].pos);
            (t > reach && t <= reach + length && off <= width + foes[i].radius * 0.5)
                .then_some((i, t))
        })
        .collect();
    hit.sort_by(|a, b| a.1.total_cmp(&b.1));
    hit.into_iter().take(n).map(|(i, _)| i).collect()
}

/// What a blow flying back from `from` to `to` passes through: every foe within `width` of the
/// way, in the order it meets them; never `not`.
pub(crate) fn boomerang(
    foes: &[Foe],
    from: Vec3,
    to: Vec3,
    not: Option<Entity>,
    width: f32,
) -> Vec<usize> {
    let span = dist(to, from);
    let mut hit: Vec<(usize, f32)> = (0..foes.len())
        .filter(|&i| Some(foes[i].e) != not)
        .filter_map(|i| {
            let (t, off) = along(from, to, foes[i].pos);
            (t >= 0.0 && t <= span && off <= width + foes[i].radius * 0.5).then_some((i, t))
        })
        .collect();
    hit.sort_by(|a, b| a.1.total_cmp(&b.1));
    hit.into_iter().map(|(i, _)| i).collect()
}

/// Among the foes within `range` of `at`, the place of the one with the most others within `r`.
pub(crate) fn densest(foes: &[Foe], at: Vec3, range: f32, r: f32) -> Option<Vec3> {
    let near: Vec<&Foe> = foes
        .iter()
        .filter(|f| dist(f.pos, at) <= range)
        .take(60)
        .collect();
    near.iter()
        .map(|f| {
            let n = near.iter().filter(|o| dist(o.pos, f.pos) <= r).count();
            (f.pos, n)
        })
        .max_by_key(|&(_, n)| n)
        .map(|(p, _)| p)
}

/// A random foe within `range` of `at`.
pub(crate) fn random(foes: &[Foe], at: Vec3, range: f32, rng: &mut Rng) -> Option<usize> {
    let near: Vec<usize> = (0..foes.len())
        .filter(|&i| dist(foes[i].pos, at) <= range)
        .collect();
    (!near.is_empty()).then(|| near[rng.index(near.len())])
}
