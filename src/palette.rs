//! A small, loud palette. Everything on screen is one of 16 palette slots.
//!
//! Palettes ("colour schemes") are 8x2 pixel PNGs in `assets/palettes/`, listed
//! in `assets/palettes.txt`, so they can be edited in any paint program. The
//! top row holds 8 bright colours and the bottom row their darkened versions:
//!
//! | col | top row (bright)              | bottom row (dark)                   |
//! |-----|-------------------------------|-------------------------------------|
//! | 0   | light: built-in faces, text   | light shade                         |
//! | 1   | primary: text, highlights     | primary dark: dashed rules          |
//! | 2   | red: selection, errors, ink   | red dark                            |
//! | 3   | highlight: hover, prompt      | highlight dark                      |
//! | 4   | blue: ink                     | blue dark                           |
//! | 5   | tile body (black tile tone)   | tile body in shadow                 |
//! | 6   | background                    | background lattice dots             |
//! | 7   | tile: white parts of tile.png | tile dark: grey parts of tile.png   |
//!
//! Black ink in face art (e.g. the bird's eye) uses the background colour
//! (column 6, top), so it always matches the table the tiles sit on.
//!
//! Switching palettes swaps every colour at once: all art is stored as slots.
//! Tile sprites are three-level (white / grey / black): white pixels take a
//! tint's top-row colour (column 7 normally, highlight on hover, red when
//! selected), grey pixels the colour under it, black pixels the tile body. Cast shadows move a pixel from the top row to the bottom row.

pub const IVORY: u8 = 0;
pub const GREEN: u8 = 1;
pub const RED: u8 = 2;
pub const AMBER: u8 = 3;
pub const BLUE: u8 = 4;
pub const BODY: u8 = 5;
pub const BG: u8 = 6;
pub const TILE: u8 = 7;
pub const IVORY_SH: u8 = 8;
pub const DKGREEN: u8 = 9;
pub const DKRED: u8 = 10;
pub const DKAMBER: u8 = 11;
pub const DKBLUE: u8 = 12;
#[allow(dead_code)] // reached via shade_of(BODY)
pub const BODY_SH: u8 = 13;
pub const DIM: u8 = 14;
pub const TILE_SH: u8 = 15;
/// Black ink in face art: drawn in the background colour.
pub const BLACK: u8 = BG;

/// Palette slots: the 16 pixels of the palette PNG (8 columns x 2 rows).
pub const SLOTS: usize = 16;
/// Width of a palette PNG; its height is 2.
pub const COLS: usize = 8;
pub type Rgb = [u8; 3];

/// Grey level of the middle tone in three-level sprites.
pub const GREY: u8 = 127;

/// The darkened slot for a pixel under a cast shadow (bottom row of its column).
/// Background stays background: its bottom row is the lattice, not a shade.
pub fn shade_of(c: u8) -> u8 {
    if (c as usize) < COLS && c != BG {
        c + COLS as u8
    } else {
        c
    }
}

/// Slots that face art (faces.png) can use. BLACK is the background slot, used
/// as ink; lattice, tile body and tile colour are scheme roles, not art colours.
pub const INK: [u8; 11] = [IVORY, GREEN, RED, AMBER, BLUE, BLACK, IVORY_SH, DKGREEN, DKRED, DKAMBER, DKBLUE];

/// Software multiply of a colour by a grey level (0..=255).
pub const fn mul(c: Rgb, g: u8) -> Rgb {
    let g = g as u16;
    [
        ((c[0] as u16 * g + 127) / 255) as u8,
        ((c[1] as u16 * g + 127) / 255) as u8,
        ((c[2] as u16 * g + 127) / 255) as u8,
    ]
}

#[derive(Clone, Debug, PartialEq)]
pub struct Scheme {
    pub name: String,
    pub rgb: [Rgb; SLOTS],
}

const K: Rgb = [0, 0, 0];
const P_LIGHT: Rgb = [0xF2, 0xE8, 0xCF];
const P_PRIMARY: Rgb = [0x2E, 0xD8, 0x6A];
const P_RED: Rgb = [0xEE, 0x25, 0x3A];
const P_HIGHLIGHT: Rgb = [0xFF, 0xB0, 0x00];
const P_BLUE: Rgb = [0x2A, 0x5C, 0xF0];

/// Built-in PHOSPHOR palette (identical to assets/palettes/phosphor.png). It is
/// the fallback when no palette file loads, and the reference faces.png is
/// snapped to, so face art keeps its slots however the palette files change.
pub const PALETTE: [Rgb; SLOTS] = [
    P_LIGHT,
    P_PRIMARY,
    P_RED,
    P_HIGHLIGHT,
    P_BLUE,
    K, // tile body
    K, // background
    P_PRIMARY, // tile
    [0xBF, 0xAA, 0x80],
    mul(P_PRIMARY, GREY),
    mul(P_RED, GREY),
    mul(P_HIGHLIGHT, GREY),
    mul(P_BLUE, GREY),
    K, // tile body in shadow
    [0x10, 0x2C, 0x1C],
    mul(P_PRIMARY, GREY), // tile dark
];

pub fn builtin() -> Scheme {
    Scheme { name: "PHOSPHOR".into(), rgb: PALETTE }
}

/// Palette from the RGBA bytes of an 8x2 image (row-major, top row first).
/// Alpha is ignored: palette colours are always opaque.
pub fn from_rgba(name: &str, rgba: &[u8]) -> Scheme {
    let mut rgb = [K; SLOTS];
    for (slot, px) in rgba.chunks_exact(4).take(SLOTS).enumerate() {
        rgb[slot] = [px[0], px[1], px[2]];
    }
    Scheme { name: name.to_ascii_uppercase(), rgb }
}

/// Names listed in palettes.txt: one per line, `#` comments, letters, digits,
/// `-` and `_` only; duplicates ignored.
pub fn parse_list(list: &str) -> Vec<String> {
    let mut names: Vec<String> = vec![];
    for line in list.lines() {
        let name = line.split('#').next().unwrap_or("").trim();
        if name.is_empty() {
            continue;
        }
        if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_') {
            eprintln!("rusty-majong: palettes.txt: ignoring {name:?} (use letters, digits, - and _)");
            continue;
        }
        if !names.iter().any(|n| n.eq_ignore_ascii_case(name)) {
            names.push(name.to_string());
        }
    }
    names
}

/// Load the palettes listed in `root/palettes.txt` from `root/palettes/<name>.png`
/// (file system on desktop, HTTP on the web). Unreadable or wrong-sized files
/// are skipped with a message; if none load, the built-in PHOSPHOR is used.
/// `fresh` skips the browser's cache (reloads on the web).
pub async fn load(root: &str, fresh: bool) -> Vec<Scheme> {
    use crate::sprites::fresh_path;
    let root = root.trim_end_matches('/');
    let list = macroquad::file::load_string(&fresh_path(&format!("{root}/palettes.txt"), fresh)).await.unwrap_or_default();
    let mut schemes = vec![];
    for name in parse_list(&list) {
        let path = format!("{root}/palettes/{name}.png");
        let Ok(bytes) = macroquad::file::load_file(&fresh_path(&path, fresh)).await else {
            eprintln!("rusty-majong: palette {path} not found");
            continue;
        };
        match decode(&bytes) {
            Ok(rgba) => schemes.push(from_rgba(&name, &rgba)),
            Err(e) => eprintln!("rusty-majong: palette {path}: {e}"),
        }
    }
    if schemes.is_empty() {
        schemes.push(builtin());
    }
    schemes
}

/// Decode an 8x2 palette PNG to RGBA.
pub fn decode(bytes: &[u8]) -> Result<Vec<u8>, String> {
    use macroquad::prelude::{Image, ImageFormat};
    let img = Image::from_file_with_format(bytes, Some(ImageFormat::Png)).map_err(|e| format!("{e:?}"))?;
    if (img.width as usize, img.height as usize) != (COLS, 2) {
        return Err(format!("is {}x{}, expected {COLS}x2", img.width, img.height));
    }
    Ok(img.bytes)
}

/// A tint for three-level sprites: (slot for white, slot for grey).
#[derive(Clone, Copy)]
pub struct Tint {
    pub full: u8,
    pub half: u8,
}

pub const TINT_NORMAL: Tint = Tint { full: TILE, half: TILE_SH };
pub const TINT_HOVER: Tint = Tint { full: AMBER, half: DKAMBER };
pub const TINT_SELECT: Tint = Tint { full: RED, half: DKRED };

#[cfg(test)]
mod tests {
    use super::*;

    fn shipped() -> Vec<Scheme> {
        let list = std::fs::read_to_string("assets/palettes.txt").expect("assets/palettes.txt");
        parse_list(&list)
            .iter()
            .map(|n| {
                let bytes = std::fs::read(format!("assets/palettes/{n}.png")).expect(n);
                from_rgba(n, &decode(&bytes).unwrap_or_else(|e| panic!("{n}: {e}")))
            })
            .collect()
    }

    #[test]
    fn shipped_palettes_load_and_phosphor_matches_builtin() {
        let schemes = shipped();
        assert!(schemes.len() >= 2);
        assert_eq!(schemes[0], builtin(), "phosphor.png must equal the built-in PALETTE");
        assert_eq!(BLACK, BG, "black ink is drawn in the background colour");
    }

    #[test]
    fn tints_use_a_column_top_and_bottom() {
        for t in [TINT_NORMAL, TINT_HOVER, TINT_SELECT] {
            assert_eq!(t.half as usize, t.full as usize + COLS);
            assert_eq!(shade_of(t.full), t.half);
        }
        assert_eq!(shade_of(BODY), BODY_SH);
        assert_eq!(shade_of(BG), BG);
        assert_eq!(shade_of(DIM), DIM);
    }

    #[test]
    fn list_parsing() {
        assert_eq!(parse_list("a\n# c\n\n b # t\n../x\nA\n"), ["a", "b"]);
    }
}
