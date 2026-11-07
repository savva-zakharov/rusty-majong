//! Tile faces, hand-pixelled. A face is 23x29 pixels.

use crate::fb::Fb;
use crate::palette::*;

pub const FW: i32 = 23;
pub const FH: i32 = 29;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Face {
    Dots(u8),
    Bam(u8),
    Chr(u8),
    Wind(u8),   // 1 E, 2 S, 3 W, 4 N
    Dragon(u8), // 1 red, 2 green, 3 white
    Flower(u8),
    Season(u8),
    /// The four 1-of-bamboo tiles, each with its own bird picture (1..=4).
    /// They match each other, like flowers.
    Bird(u8),
}

impl Face {
    /// Tiles match when their keys are equal (any flower matches any flower, etc).
    pub fn key(self) -> (u8, u8) {
        match self {
            Face::Dots(n) => (0, n),
            Face::Bam(n) => (1, n),
            Face::Bird(_) => (1, 1),
            Face::Chr(n) => (2, n),
            Face::Wind(n) => (3, n),
            Face::Dragon(n) => (4, n),
            Face::Flower(_) => (5, 0),
            Face::Season(_) => (6, 0),
        }
    }

    pub fn matches(self, o: Face) -> bool {
        self.key() == o.key()
    }

    pub fn name(self) -> String {
        const WINDS: [&str; 4] = ["EAST", "SOUTH", "WEST", "NORTH"];
        const DRAGONS: [&str; 3] = ["RED", "GREEN", "WHITE"];
        const FLOWERS: [&str; 4] = ["PLUM", "ORCHID", "CHRYSANTHEMUM", "BAMBOO"];
        const SEASONS: [&str; 4] = ["SPRING", "SUMMER", "AUTUMN", "WINTER"];
        match self {
            Face::Dots(n) => format!("{n} OF DOTS"),
            Face::Bam(n) => format!("{n} OF BAMBOO"),
            Face::Bird(_) => "1 OF BAMBOO".into(),
            Face::Chr(n) => format!("{n} OF CHARACTERS"),
            Face::Wind(n) => format!("{} WIND", WINDS[n as usize - 1]),
            Face::Dragon(n) => format!("{} DRAGON", DRAGONS[n as usize - 1]),
            Face::Flower(n) => format!("FLOWER: {}", FLOWERS[n as usize - 1]),
            Face::Season(n) => format!("SEASON: {}", SEASONS[n as usize - 1]),
        }
    }

    /// Main ink colour, used for removal particles.
    pub fn ink(self) -> u8 {
        match self {
            Face::Dots(_) => BLUE,
            Face::Bam(_) | Face::Bird(_) => GREEN,
            Face::Chr(_) => RED,
            Face::Wind(_) => BLUE,
            Face::Dragon(1) => RED,
            Face::Dragon(2) => GREEN,
            Face::Dragon(_) => BLUE,
            Face::Flower(_) => RED,
            Face::Season(_) => AMBER,
        }
    }
}

pub fn full_set_pairs() -> Vec<(Face, Face)> {
    let mut v = Vec::with_capacity(72);
    for n in 1..=9 {
        for f in [Face::Dots(n), Face::Bam(n), Face::Chr(n)] {
            if f == Face::Bam(1) {
                continue; // the four birds below
            }
            v.push((f, f));
            v.push((f, f));
        }
    }
    v.push((Face::Bird(1), Face::Bird(2)));
    v.push((Face::Bird(3), Face::Bird(4)));
    for n in 1..=4 {
        v.push((Face::Wind(n), Face::Wind(n)));
        v.push((Face::Wind(n), Face::Wind(n)));
    }
    for n in 1..=3 {
        v.push((Face::Dragon(n), Face::Dragon(n)));
        v.push((Face::Dragon(n), Face::Dragon(n)));
    }
    v.push((Face::Flower(1), Face::Flower(2)));
    v.push((Face::Flower(3), Face::Flower(4)));
    v.push((Face::Season(1), Face::Season(2)));
    v.push((Face::Season(3), Face::Season(4)));
    v
}

// ---------------------------------------------------------------- sprites

const DOT: [&str; 5] = [".###.", "#...#", "#.#.#", "#...#", ".###."];

const BIG_DOT: [&str; 13] = [
    "....ggggg....",
    "..gg.....gg..",
    ".g..rrrrr..g.",
    ".g.r.....r.g.",
    "g.r..bbb..r.g",
    "g.r.b...b.r.g",
    "g.r.b.r.b.r.g",
    "g.r.b...b.r.g",
    "g.r..bbb..r.g",
    ".g.r.....r.g.",
    ".g..rrrrr..g.",
    "..gg.....gg..",
    "....ggggg....",
];

const STICK: [&str; 9] = ["###", ".#.", ".#.", ".#.", "###", ".#.", ".#.", ".#.", "###"];

const BIRD: [&str; 17] = [
    ".......rrr.....",
    "......rrrrr....",
    "......rkrrraa..",
    "......rrrr.....",
    ".......gg......",
    "......gggg.....",
    ".....gggggg....",
    "....ggbbbggg...",
    "...ggbbbbbgg...",
    "..ggbbbbbbgg...",
    "..gbbrrrbbg....",
    "...gbbrbbg.....",
    "....gbbbg......",
    ".....ggg.......",
    "....r.g.r......",
    "...r..g..r.....",
    "..rr..g..rr....",
];

const NUMERALS: [[&str; 7]; 9] = [
    [".........", ".........", ".........", "#########", ".........", ".........", "........."],
    [".........", ".#######.", ".........", ".........", ".........", "#########", "........."],
    [".#######.", ".........", "..#####..", ".........", ".........", "#########", "........."],
    ["#########", "#..#.#..#", "#..#.#..#", "#.#..#..#", "##....###", "#.......#", "#########"],
    ["#########", "...#.....", "...#.....", ".#######.", "...#...#.", "..#....#.", "#########"],
    ["....#....", ".....#...", "#########", ".........", "..#...#..", ".#.....#.", "#.......#"],
    ["..#......", "..#...##.", "..####...", "###......", "..#......", "..#.....#", "...######"],
    ["...#.#...", "...#.#...", "...#..#..", "..#...#..", "..#....#.", ".#.....#.", "#.......#"],
    ["...#.....", "...#.....", "#######..", "...#..#..", "...#..#..", "..#...#.#", "##....###"],
];

const WAN: [&str; 9] = [
    "#########",
    "...#.....",
    "...#.....",
    "...######",
    "...#....#",
    "..#.....#",
    "..#.....#",
    ".#......#",
    "#.....##.",
];

const WINDS: [[&str; 11]; 4] = [
    [
        ".....#.....",
        "###########",
        ".....#.....",
        ".#########.",
        ".#...#...#.",
        ".#########.",
        ".#...#...#.",
        ".#########.",
        "....###....",
        "..##.#.##..",
        "##...#...##",
    ],
    [
        ".....#.....",
        "###########",
        ".....#.....",
        ".#########.",
        ".#..#.#..#.",
        ".#.#####.#.",
        ".#...#...#.",
        ".#.#####.#.",
        ".#...#...#.",
        ".#...#...#.",
        ".#......##.",
    ],
    [
        "###########",
        "...#...#...",
        ".#########.",
        ".#.#...#.#.",
        ".#.#...#.#.",
        ".#.#...#.#.",
        ".##.....##.",
        ".#.......#.",
        ".#.......#.",
        ".#########.",
        ".#.......#.",
    ],
    [
        "...#...#...",
        "...#...#...",
        "...#...#..#",
        "...#...#.#.",
        "####...##..",
        "...#...#...",
        "...#...#...",
        "...#...#...",
        "..##...#...",
        "...#...#..#",
        "...#....##.",
    ],
];

const ZHONG: [&str; 11] = [
    ".....#.....",
    ".....#.....",
    "###########",
    "#....#....#",
    "#....#....#",
    "#....#....#",
    "###########",
    ".....#.....",
    ".....#.....",
    ".....#.....",
    ".....#.....",
];

const FA: [&str; 11] = [
    "...#...#...",
    "...#....#..",
    "###########",
    "...#.......",
    "...######..",
    "..#.#...#..",
    "..#..#.#...",
    ".#....#....",
    ".#...#.#...",
    "#...#...##.",
    "...#.....##",
];

const FLOWER: [&str; 15] = [
    "..pp...pp..",
    ".pppp.pppp.",
    ".ppppppppp.",
    "..ppcccpp..",
    "pppcccccppp",
    "pppcccccppp",
    ".ppcccccpp.",
    "..ppppppp..",
    ".pppp.pppp.",
    "..pp.g.pp..",
    ".....g.....",
    "...ggg.....",
    "..g..g.gg..",
    ".....gg....",
    ".....g.....",
];

const SPROUT: [&str; 11] = [
    ".gg.....gg.",
    "gggg...gggg",
    "ggggg.ggggg",
    ".gggg.gggg.",
    "...ggggg...",
    ".....g.....",
    ".....g.....",
    ".....g.....",
    "....ddd....",
    "..ddddddd..",
    "ddddddddddd",
];

const SUN: [&str; 11] = [
    ".....a.....",
    ".a...a...a.",
    "..a.....a..",
    "....aaa....",
    "...aaaaa...",
    "aa.aarar.aa",
    "...aaaaa...",
    "....aaa....",
    "..a.....a..",
    ".a...a...a.",
    ".....a.....",
];

const LEAF: [&str; 12] = [
    "........rrr",
    "......rrrrr",
    ".....rrrrar",
    "....rrrrarr",
    "...rrrrarrr",
    "..rrrrarrr.",
    "..rrrarrrr.",
    ".rrrarrrr..",
    ".rrarrrr...",
    ".rarrrr....",
    ".arrr......",
    "a..........",
];

const SNOW: [&str; 11] = [
    ".....b.....",
    "...b.b.b...",
    "....bbb....",
    ".b...b...b.",
    "..b..b..b..",
    "bbbbbbbbbbb",
    "..b..b..b..",
    ".b...b...b.",
    "....bbb....",
    "...b.b.b...",
    ".....b.....",
];

fn dot_layout(n: u8) -> &'static [(i32, i32, u8)] {
    match n {
        2 => &[(0, -6, GREEN), (0, 6, BLUE)],
        3 => &[(-6, -8, BLUE), (0, 0, RED), (6, 8, GREEN)],
        4 => &[(-4, -5, BLUE), (4, -5, GREEN), (-4, 5, GREEN), (4, 5, BLUE)],
        5 => &[(-6, -8, BLUE), (6, -8, GREEN), (0, 0, RED), (-6, 8, GREEN), (6, 8, BLUE)],
        6 => &[
            (-4, -8, GREEN), (4, -8, GREEN),
            (-4, 1, RED), (4, 1, RED),
            (-4, 8, RED), (4, 8, RED),
        ],
        7 => &[
            (-6, -9, GREEN), (0, -6, GREEN), (6, -3, GREEN),
            (-4, 3, RED), (4, 3, RED), (-4, 9, RED), (4, 9, RED),
        ],
        8 => &[
            (-4, -9, BLUE), (4, -9, BLUE), (-4, -3, BLUE), (4, -3, BLUE),
            (-4, 3, BLUE), (4, 3, BLUE), (-4, 9, BLUE), (4, 9, BLUE),
        ],
        _ => &[
            (-6, -7, BLUE), (0, -7, BLUE), (6, -7, BLUE),
            (-6, 0, RED), (0, 0, RED), (6, 0, RED),
            (-6, 7, GREEN), (0, 7, GREEN), (6, 7, GREEN),
        ],
    }
}

fn bam_layout(n: u8) -> &'static [(i32, i32, u8)] {
    match n {
        2 => &[(0, -5, GREEN), (0, 5, BLUE)],
        3 => &[(0, -5, GREEN), (-4, 5, BLUE), (4, 5, BLUE)],
        4 => &[(-4, -5, GREEN), (4, -5, BLUE), (-4, 5, BLUE), (4, 5, GREEN)],
        5 => &[(-6, -5, GREEN), (6, -5, BLUE), (0, 0, RED), (-6, 5, BLUE), (6, 5, GREEN)],
        6 => &[
            (-5, -5, GREEN), (0, -5, GREEN), (5, -5, GREEN),
            (-5, 5, BLUE), (0, 5, BLUE), (5, 5, BLUE),
        ],
        7 => &[
            (0, -9, RED),
            (-5, 0, GREEN), (0, 0, GREEN), (5, 0, GREEN),
            (-5, 9, BLUE), (0, 9, BLUE), (5, 9, BLUE),
        ],
        8 => &[
            (-6, -5, GREEN), (-2, -5, GREEN), (2, -5, GREEN), (6, -5, GREEN),
            (-6, 5, BLUE), (-2, 5, BLUE), (2, 5, BLUE), (6, 5, BLUE),
        ],
        _ => &[
            (-5, -9, GREEN), (0, -9, RED), (5, -9, BLUE),
            (-5, 0, GREEN), (0, 0, RED), (5, 0, BLUE),
            (-5, 9, GREEN), (0, 9, RED), (5, 9, BLUE),
        ],
    }
}

/// Draw the art for `face` onto a face rect whose top-left is (x, y).
pub fn draw_face(fb: &mut Fb, x: i32, y: i32, face: Face) {
    let cx = x + FW / 2;
    let cy = y + FH / 2;
    let digit = |n: u8| char::from(b'0' + n).to_string();
    match face {
        Face::Dots(1) => fb.sprite_c(cx, cy, &BIG_DOT, BLUE, BLUE),
        Face::Dots(n) => {
            for &(dx, dy, c) in dot_layout(n) {
                fb.sprite(cx + dx - 2, cy + dy - 2, &DOT, c, c);
            }
        }
        Face::Bam(1) | Face::Bird(1) => fb.sprite_c(cx, cy, &BIRD, GREEN, GREEN),
        Face::Bird(n) => {
            // Built-in variants: 2 faces the other way, 3 swaps blue and red, 4 both.
            let rows: Vec<String> = BIRD
                .iter()
                .map(|r| {
                    let r: String = if n == 3 { r.to_string() } else { r.chars().rev().collect() };
                    if n == 2 {
                        r
                    } else {
                        r.chars().map(|c| match c { 'b' => 'r', 'r' => 'b', c => c }).collect()
                    }
                })
                .collect();
            let rows: Vec<&str> = rows.iter().map(|r| r.as_str()).collect();
            fb.sprite_c(cx, cy, &rows, GREEN, GREEN);
        }
        Face::Bam(n) => {
            for &(dx, dy, c) in bam_layout(n) {
                fb.sprite(cx + dx - 1, cy + dy - 4, &STICK, c, c);
            }
        }
        Face::Chr(n) => {
            fb.sprite(x + 7, y + 4, &NUMERALS[n as usize - 1], BLUE, BLUE);
            fb.sprite(x + 7, y + 15, &WAN, RED, RED);
            fb.tiny(x + 2, y + 2, &digit(n), RED);
        }
        Face::Wind(n) => {
            fb.sprite_c(cx, cy + 1, &WINDS[n as usize - 1], BLUE, BLUE);
            let l = ["E", "S", "W", "N"][n as usize - 1];
            fb.tiny(x + 2, y + 2, l, RED);
        }
        Face::Dragon(1) => fb.sprite_c(cx, cy, &ZHONG, RED, RED),
        Face::Dragon(2) => fb.sprite_c(cx, cy, &FA, GREEN, GREEN),
        Face::Dragon(_) => {
            fb.outline(x + 4, y + 4, FW - 8, FH - 8, BLUE);
            fb.outline(x + 6, y + 6, FW - 12, FH - 12, BLUE);
            fb.rect(x + 4, y + 4, 3, 3, BLUE);
            fb.rect(x + FW - 7, y + 4, 3, 3, BLUE);
            fb.rect(x + 4, y + FH - 7, 3, 3, BLUE);
            fb.rect(x + FW - 7, y + FH - 7, 3, 3, BLUE);
        }
        Face::Flower(n) => {
            let (petal, centre) = match n {
                1 => (RED, AMBER),
                2 => (BLUE, AMBER),
                3 => (AMBER, RED),
                _ => (GREEN, RED),
            };
            fb.sprite_c(cx, cy + 2, &FLOWER, petal, centre);
            fb.tiny(x + 2, y + 2, &digit(n), GREEN);
        }
        Face::Season(n) => {
            match n {
                1 => fb.sprite_c(cx, cy + 2, &SPROUT, GREEN, GREEN),
                2 => fb.sprite_c(cx, cy + 2, &SUN, AMBER, AMBER),
                3 => fb.sprite_c(cx, cy + 2, &LEAF, RED, RED),
                _ => fb.sprite_c(cx, cy + 2, &SNOW, BLUE, BLUE),
            }
            fb.tiny(x + 2, y + 2, &digit(n), BLUE);
        }
    }
}
