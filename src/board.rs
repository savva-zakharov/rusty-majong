//! Layout, free-tile rules and a dealer that only produces solvable boards.

use crate::art::Face;
use macroquad::rand::RandGenerator;

/// Position in half-tile units; a tile covers 2x2 cells. `z` is the layer.
#[derive(Clone, Copy, Debug)]
pub struct Pos {
    pub gx: i32,
    pub gy: i32,
    pub z: i32,
}

pub struct Nb {
    pub z: i32,
    pub above: Vec<usize>,
    pub left: Vec<usize>,
    pub right: Vec<usize>,
    pub up: Vec<usize>, // same layer, directly above on screen (used for shadows)
}

pub struct Layout {
    pub name: &'static str,
    pub build: fn() -> Vec<Pos>,
}

/// All layouts, 144 tiles each. The new-board dialog picks between them.
pub const LAYOUTS: &[Layout] = &[
    Layout { name: "TURTLE", build: turtle },
    Layout { name: "ZIGGURAT", build: ziggurat },
    Layout { name: "PYRAMID", build: pyramid },
    Layout { name: "CASTLE", build: castle },
];

/// Columns/rows here are whole tiles; positions are stored in half-tile units.
fn tile(v: &mut Vec<Pos>, col: f32, row: f32, z: i32) {
    v.push(Pos { gx: (col * 2.0) as i32, gy: (row * 2.0) as i32, z });
}

/// The traditional "Turtle": a shell of rows, a 6x6 / 4x4 / 2x2 stack,
/// a single capstone, and head/tail end tiles.
fn turtle() -> Vec<Pos> {
    let mut v = Vec::with_capacity(144);
    // Layer 0: (row, first col, count) — 84 tiles + 3 end tiles.
    let rows = [(0, 1, 12), (1, 3, 8), (2, 2, 10), (3, 1, 12), (4, 1, 12), (5, 2, 10), (6, 3, 8), (7, 1, 12)];
    for (r, c0, n) in rows {
        for c in c0..c0 + n {
            tile(&mut v, c as f32, r as f32, 0);
        }
    }
    tile(&mut v, 0.0, 3.5, 0); // head
    tile(&mut v, 13.0, 3.5, 0); // tail
    tile(&mut v, 14.0, 3.5, 0);
    for (z, lo, n) in [(1, 4, 6), (2, 5, 4), (3, 6, 2)] {
        for r in 0..n {
            for c in 0..n {
                tile(&mut v, (lo + c) as f32, (lo - 3 + r) as f32, z);
            }
        }
    }
    tile(&mut v, 6.5, 3.5, 4); // capstone
    v
}

/// Half-stepped pyramid: each layer sits offset by half a tile on the one below.
fn pyramid() -> Vec<Pos> {
    let mut v = Vec::with_capacity(144);
    // (z, first col, first row, cols, rows): 60 + 45 + 32 + 7
    for (z, c0, r0, w, h) in [(0, 0.0, 0.0, 10, 6), (1, 0.5, 0.5, 9, 5), (2, 1.0, 1.0, 8, 4), (3, 1.5, 2.5, 7, 1)] {
        for r in 0..h {
            for c in 0..w {
                tile(&mut v, c0 + c as f32, r0 + r as f32, z);
            }
        }
    }
    v
}

/// Walled castle: four corner towers, walls, a central keep and two gates.
fn castle() -> Vec<Pos> {
    let mut v = Vec::with_capacity(144);
    for r in 0..6 {
        for c in 0..12 {
            tile(&mut v, c as f32, r as f32, 0); // courtyard floor, 72
        }
    }
    let towers = [(0, 0), (10, 0), (0, 4), (10, 4)];
    for z in 1..=2 {
        for (tc, tr) in towers {
            for (dc, dr) in [(0, 0), (1, 0), (0, 1), (1, 1)] {
                tile(&mut v, (tc + dc) as f32, (tr + dr) as f32, z); // 2 x 16
            }
        }
    }
    for (tc, tr) in towers {
        tile(&mut v, tc as f32 + 0.5, tr as f32 + 0.5, 3); // tower caps, 4
    }
    for c in 2..10 {
        tile(&mut v, c as f32, 0.0, 1); // north wall
        tile(&mut v, c as f32, 5.0, 1); // south wall, 16
    }
    for r in 2..4 {
        tile(&mut v, 0.0, r as f32, 1); // west wall
        tile(&mut v, 11.0, r as f32, 1); // east wall, 4
    }
    for r in 2..4 {
        for c in 4..8 {
            tile(&mut v, c as f32, r as f32, 1); // keep, 8
        }
        for c in 5..7 {
            tile(&mut v, c as f32, r as f32, 2); // keep, 4
        }
    }
    tile(&mut v, 5.5, 2.5, 3); // keep spire
    tile(&mut v, 5.5, 2.5, 4);
    tile(&mut v, -1.0, 2.5, 0); // gates
    tile(&mut v, 12.0, 2.5, 0);
    v
}

/// "Ziggurat": a stepped pyramid with wings, 144 tiles.
fn ziggurat() -> Vec<Pos> {
    let mut v = Vec::with_capacity(144);
    let mut add = |gx, gy, z| v.push(Pos { gx, gy, z });
    for j in 0..6 {
        for i in 0..12 {
            add(2 * i, 2 * j, 0); // 72
        }
    }
    for j in 0..5 {
        for i in 0..8 {
            add(4 + 2 * i, 1 + 2 * j, 1); // 40, rows staggered half a tile
        }
    }
    for j in 0..3 {
        for i in 0..6 {
            add(6 + 2 * i, 3 + 2 * j, 2); // 18
        }
    }
    for j in 0..2 {
        for i in 0..4 {
            add(8 + 2 * i, 4 + 2 * j, 3); // 8
        }
    }
    add(10, 5, 4);
    add(12, 5, 4); // 2
    add(-2, 5, 0);
    add(-4, 5, 0);
    add(24, 5, 0);
    add(26, 5, 0); // 4 wings
    v
}

pub fn neighbors(p: &[Pos]) -> Vec<Nb> {
    (0..p.len())
        .map(|i| {
            let a = p[i];
            let mut nb = Nb { z: a.z, above: vec![], left: vec![], right: vec![], up: vec![] };
            for (j, b) in p.iter().enumerate() {
                if i == j {
                    continue;
                }
                let oy = (a.gy - b.gy).abs() < 2;
                let ox = (a.gx - b.gx).abs() < 2;
                if b.z > a.z && ox && oy {
                    nb.above.push(j);
                }
                if b.z == a.z && (a.gx - b.gx).abs() <= 2 && (1..=2).contains(&(a.gy - b.gy)) {
                    nb.up.push(j);
                }
                if b.z == a.z && oy {
                    let d = a.gx - b.gx;
                    if (1..=2).contains(&d) {
                        nb.left.push(j);
                    }
                    if (-2..=-1).contains(&d) {
                        nb.right.push(j);
                    }
                }
            }
            nb
        })
        .collect()
}

pub fn is_free(nb: &[Nb], alive: &[bool], i: usize) -> bool {
    let n = &nb[i];
    alive[i]
        && !n.above.iter().any(|&j| alive[j])
        && (!n.left.iter().any(|&j| alive[j]) || !n.right.iter().any(|&j| alive[j]))
}

pub fn shuffle<T>(rng: &RandGenerator, v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = rng.gen_range(0, i as u32 + 1) as usize;
        v.swap(i, j);
    }
}

/// Fill `slots` with `pairs` by playing the game backwards: repeatedly pick two
/// currently-free slots and give them a matching pair. Removing pairs in the
/// order they were placed is then always a valid solution. All randomness
/// comes from `rng`, so a generator seeded the same way deals the same board.
pub fn deal(rng: &RandGenerator, nb: &[Nb], slots: &[usize], pairs: &[(Face, Face)]) -> Option<Vec<(usize, Face)>> {
    let n = nb.len();
    let mut pairs = pairs.to_vec();
    for _attempt in 0..2000 {
        shuffle(rng, &mut pairs);
        let mut alive = vec![false; n];
        for &s in slots {
            alive[s] = true;
        }
        let mut out = Vec::with_capacity(slots.len());
        let mut ok = true;
        for &(a, b) in &pairs {
            let mut free: Vec<usize> =
                slots.iter().copied().filter(|&i| is_free(nb, &alive, i)).collect();
            if free.len() < 2 {
                ok = false;
                break;
            }
            let i1 = pick_weighted(rng, nb, &free);
            free.retain(|&x| x != i1);
            let i2 = pick_weighted(rng, nb, &free);
            alive[i1] = false;
            alive[i2] = false;
            out.push((i1, a));
            out.push((i2, b));
        }
        if ok {
            return Some(out);
        }
    }
    None
}

/// Prefer high tiles so tall stacks don't end up as an unpairable leftover.
fn pick_weighted(rng: &RandGenerator, nb: &[Nb], free: &[usize]) -> usize {
    let w = |i: usize| ((nb[i].z + 1) * (nb[i].z + 1)) as u32;
    let total: u32 = free.iter().map(|&i| w(i)).sum();
    let mut r = rng.gen_range(0, total);
    for &i in free {
        if r < w(i) {
            return i;
        }
        r -= w(i);
    }
    free[free.len() - 1]
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::art::full_set_pairs;

    #[test]
    fn layouts_have_144_unique_supported_tiles() {
        for l in LAYOUTS {
            let p = (l.build)();
            assert_eq!(p.len(), 144, "{}", l.name);
            for i in 0..p.len() {
                for j in i + 1..p.len() {
                    let (a, b) = (p[i], p[j]);
                    let overlap = a.z == b.z && (a.gx - b.gx).abs() < 2 && (a.gy - b.gy).abs() < 2;
                    assert!(!overlap, "{}: tiles {i} and {j} overlap", l.name);
                }
                // Nothing floats: a raised tile rests on at least one tile directly below.
                let a = p[i];
                if a.z > 0 {
                    let rests = p.iter().any(|b| b.z == a.z - 1 && (a.gx - b.gx).abs() < 2 && (a.gy - b.gy).abs() < 2);
                    assert!(rests, "{}: tile {i} {a:?} floats", l.name);
                }
            }
        }
    }

    #[test]
    fn deals_are_solvable_in_placement_order() {
        for l in LAYOUTS {
            let p = (l.build)();
            let nb = neighbors(&p);
            let slots: Vec<usize> = (0..p.len()).collect();
            for seed in 0..30 {
                let rng = RandGenerator::new();
                rng.srand(seed);
                let d = deal(&rng, &nb, &slots, &full_set_pairs()).unwrap_or_else(|| panic!("{}: deal failed", l.name));
                let mut alive = vec![true; p.len()];
                for pair in d.chunks(2) {
                    let (a, b) = (pair[0], pair[1]);
                    assert!(is_free(&nb, &alive, a.0) && is_free(&nb, &alive, b.0));
                    assert!(a.1.matches(b.1));
                    alive[a.0] = false;
                    alive[b.0] = false;
                }
                assert!(alive.iter().all(|&x| !x));
            }
        }
    }
}
