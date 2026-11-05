//! Layout, free-tile rules and a dealer that only produces solvable boards.

use crate::art::Face;
use macroquad::rand::gen_range;

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

/// "Ziggurat": a stepped pyramid with wings, 144 tiles.
pub fn layout() -> Vec<Pos> {
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

pub fn shuffle<T>(v: &mut [T]) {
    for i in (1..v.len()).rev() {
        let j = gen_range(0, i as u32 + 1) as usize;
        v.swap(i, j);
    }
}

/// Fill `slots` with `pairs` by playing the game backwards: repeatedly pick two
/// currently-free slots and give them a matching pair. Removing pairs in the
/// order they were placed is then always a valid solution.
pub fn deal(nb: &[Nb], slots: &[usize], pairs: &[(Face, Face)]) -> Option<Vec<(usize, Face)>> {
    let n = nb.len();
    let mut pairs = pairs.to_vec();
    for _attempt in 0..2000 {
        shuffle(&mut pairs);
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
            let i1 = pick_weighted(nb, &free);
            free.retain(|&x| x != i1);
            let i2 = pick_weighted(nb, &free);
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
fn pick_weighted(nb: &[Nb], free: &[usize]) -> usize {
    let w = |i: usize| ((nb[i].z + 1) * (nb[i].z + 1)) as u32;
    let total: u32 = free.iter().map(|&i| w(i)).sum();
    let mut r = gen_range(0, total);
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
    fn layout_has_144_unique_tiles() {
        let p = layout();
        assert_eq!(p.len(), 144);
        for i in 0..p.len() {
            for j in i + 1..p.len() {
                let (a, b) = (p[i], p[j]);
                let overlap = a.z == b.z && (a.gx - b.gx).abs() < 2 && (a.gy - b.gy).abs() < 2;
                assert!(!overlap, "tiles {i} and {j} overlap");
            }
        }
    }

    #[test]
    fn deals_are_solvable_in_placement_order() {
        let p = layout();
        let nb = neighbors(&p);
        let slots: Vec<usize> = (0..p.len()).collect();
        for seed in 0..50 {
            macroquad::rand::srand(seed);
            let d = deal(&nb, &slots, &full_set_pairs()).expect("deal failed");
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
