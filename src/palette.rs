//! A small, loud palette. Everything on screen is one of these indices.
//!
//! Colour schemes are palette swaps: every scheme gives its own RGB for the
//! same 11 slots, so all art (built-in, tile.png, faces.png) recolours at once.
//! Slot names describe the default PHOSPHOR scheme; in other schemes read them
//! by role (GREEN = primary, AMBER = highlight, RED = selection/error, ...).
//!
//! Tile sprites are three-level (white / grey / black) and are coloured by
//! multiplying with a tint: white x tint = tint, grey (127) x tint = the tint's
//! dark slot. Dark slots are computed with that exact multiplication per
//! scheme, so the result is always a palette colour. Black pixels become the
//! scheme's BODY colour: black in dark schemes, ivory in light ones.

pub const BLACK: u8 = 0; // black ink in face art
pub const IVORY: u8 = 1; // light: tile face (built-in faces), light text
pub const IVORY_SH: u8 = 2; // shaded light
pub const GREEN: u8 = 3; // primary: text + normal tile tint
pub const DKGREEN: u8 = 4; // primary x grey
pub const RED: u8 = 5; // selection tint, errors, ink
pub const BLUE: u8 = 6; // ink
pub const AMBER: u8 = 7; // highlight: hover tint, prompt
pub const DIM: u8 = 8; // background lattice
pub const DKRED: u8 = 9; // RED x grey
pub const DKAMBER: u8 = 10; // AMBER x grey
pub const BG: u8 = 11; // background ("the desk")
pub const BODY: u8 = 12; // black tone of tile sprites: the tile's face/body colour
pub const BODY_SH: u8 = 13; // BODY under a cast shadow

pub const SLOTS: usize = 14;

/// How much a cast shadow darkens the tile body (BODY_SH = BODY x this).
const SHADOW: u8 = 200;
pub type Rgb = [u8; 3];

/// Grey level of the middle tone in three-level sprites.
pub const GREY: u8 = 127;

/// Software multiply of a colour by a grey level (0..=255).
pub const fn mul(c: Rgb, g: u8) -> Rgb {
    let g = g as u16;
    [
        ((c[0] as u16 * g + 127) / 255) as u8,
        ((c[1] as u16 * g + 127) / 255) as u8,
        ((c[2] as u16 * g + 127) / 255) as u8,
    ]
}

pub struct Scheme {
    pub name: &'static str,
    pub rgb: [Rgb; SLOTS],
}

/// Base colours of a scheme; the dark slots are derived from them by `mul`.
struct Base {
    light: Rgb,
    light_sh: Rgb,
    primary: Rgb,
    red: Rgb,
    blue: Rgb,
    highlight: Rgb,
    lattice: Rgb,
    bg: Rgb,
    body: Rgb,
}

const fn scheme(name: &'static str, b: Base) -> Scheme {
    Scheme {
        name,
        rgb: [
            [0, 0, 0],
            b.light,
            b.light_sh,
            b.primary,
            mul(b.primary, GREY),
            b.red,
            b.blue,
            b.highlight,
            b.lattice,
            mul(b.red, GREY),
            mul(b.highlight, GREY),
            b.bg,
            b.body,
            mul(b.body, SHADOW),
        ],
    }
}

const K: Rgb = [0, 0, 0];

/// `P` cycles through these in-game. The first is the default.
pub const SCHEMES: &[Scheme] = &[
    scheme("PHOSPHOR", Base {
        light: [0xF2, 0xE8, 0xCF], light_sh: [0xBF, 0xAA, 0x80], primary: [0x2E, 0xD8, 0x6A],
        red: [0xEE, 0x25, 0x3A], blue: [0x2A, 0x5C, 0xF0], highlight: [0xFF, 0xB0, 0x00],
        lattice: [0x10, 0x2C, 0x1C], bg: K, body: K,
    }),
    // Light mode: ivory tiles with green backs on a pale table, dark inks.
    scheme("IVORY", Base {
        light: [0xFF, 0xFD, 0xF5], light_sh: [0xC9, 0xB6, 0x8E], primary: [0x1F, 0x7A, 0x4A],
        red: [0xC8, 0x10, 0x2E], blue: [0x1F, 0x4F, 0xB8], highlight: [0xE0, 0x7B, 0x00],
        lattice: [0xC9, 0xC0, 0xAB], bg: [0xE3, 0xDD, 0xCD], body: [0xF8, 0xF2, 0xE0],
    }),
    scheme("AMBER", Base {
        light: [0xFF, 0xF1, 0xD0], light_sh: [0xB8, 0x8A, 0x3A], primary: [0xFF, 0xB0, 0x00],
        red: [0xFF, 0x4D, 0x2E], blue: [0xFF, 0xD9, 0x8A], highlight: [0xFF, 0xF1, 0xD0],
        lattice: [0x2A, 0x1C, 0x00], bg: K, body: K,
    }),
    scheme("CGA", Base {
        light: [0xFF, 0xFF, 0xFF], light_sh: [0xAA, 0xAA, 0xAA], primary: [0x55, 0xFF, 0xFF],
        red: [0xFF, 0x55, 0xFF], blue: [0x55, 0x55, 0xFF], highlight: [0xFF, 0xFF, 0x55],
        lattice: [0x00, 0x2A, 0x2A], bg: K, body: K,
    }),
    scheme("SUNSET", Base {
        light: [0xF7, 0xF7, 0xF2], light_sh: [0xB5, 0xA8, 0x9A], primary: [0xFF, 0x6B, 0x35],
        red: [0xEE, 0x42, 0x66], blue: [0x3B, 0xCE, 0xAC], highlight: [0xFF, 0xD2, 0x3F],
        lattice: [0x2B, 0x0F, 0x0A], bg: K, body: K,
    }),
    scheme("ICE", Base {
        light: [0xE8, 0xF4, 0xFF], light_sh: [0x8F, 0xA8, 0xC0], primary: [0x5C, 0xC8, 0xFF],
        red: [0xFF, 0x5C, 0x8A], blue: [0xB0, 0x8C, 0xFF], highlight: [0xFF, 0xFF, 0xFF],
        lattice: [0x0A, 0x1A, 0x2A], bg: K, body: K,
    }),
];

/// The default scheme's colours: what faces.png is snapped to on load and what
/// `--export-assets` writes, so art is always authored against PHOSPHOR.
pub const PALETTE: [Rgb; SLOTS] = SCHEMES[0].rgb;

pub fn scheme_index(name: &str) -> Option<usize> {
    SCHEMES.iter().position(|s| s.name.eq_ignore_ascii_case(name))
}

/// A tint for three-level sprites: (slot for white, slot for grey).
#[derive(Clone, Copy)]
pub struct Tint {
    pub full: u8,
    pub half: u8,
}

pub const TINT_NORMAL: Tint = Tint { full: GREEN, half: DKGREEN };
pub const TINT_HOVER: Tint = Tint { full: AMBER, half: DKAMBER };
pub const TINT_SELECT: Tint = Tint { full: RED, half: DKRED };
