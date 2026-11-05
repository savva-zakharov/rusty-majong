//! A small, loud palette. Everything on screen is one of these indices.
//!
//! Colour schemes are palette swaps: every scheme gives its own RGB for the
//! same 11 slots, so all art (built-in, tile.png, faces.png) recolours at once.
//! Slot names describe the default PHOSPHOR scheme; in other schemes read them
//! by role (GREEN = primary, AMBER = highlight, RED = selection/error, ...).
//!
//! Tile sprites are three-level (white / grey / black) and are coloured by
//! multiplying with a tint: white x tint = tint, grey (127) x tint = the tint's
//! dark slot, black stays black. Dark slots are computed with that exact
//! multiplication per scheme, so the result is always a palette colour.

pub const BLACK: u8 = 0; // always black: background, tile bodies
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

pub const SLOTS: usize = 11;
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

/// Build a scheme from its base colours; the dark slots are derived by `mul`.
#[allow(clippy::too_many_arguments)]
const fn scheme(name: &'static str, light: Rgb, light_sh: Rgb, primary: Rgb, red: Rgb, blue: Rgb, highlight: Rgb, dim: Rgb) -> Scheme {
    Scheme {
        name,
        rgb: [
            [0, 0, 0],
            light,
            light_sh,
            primary,
            mul(primary, GREY),
            red,
            blue,
            highlight,
            dim,
            mul(red, GREY),
            mul(highlight, GREY),
        ],
    }
}

/// `P` cycles through these in-game. The first is the default.
pub const SCHEMES: &[Scheme] = &[
    //       name        light               light shade         primary             red                 blue                highlight           lattice
    scheme("PHOSPHOR", [0xF2, 0xE8, 0xCF], [0xBF, 0xAA, 0x80], [0x2E, 0xD8, 0x6A], [0xEE, 0x25, 0x3A], [0x2A, 0x5C, 0xF0], [0xFF, 0xB0, 0x00], [0x10, 0x2C, 0x1C]),
    scheme("AMBER", [0xFF, 0xF1, 0xD0], [0xB8, 0x8A, 0x3A], [0xFF, 0xB0, 0x00], [0xFF, 0x4D, 0x2E], [0xFF, 0xD9, 0x8A], [0xFF, 0xF1, 0xD0], [0x2A, 0x1C, 0x00]),
    scheme("CGA", [0xFF, 0xFF, 0xFF], [0xAA, 0xAA, 0xAA], [0x55, 0xFF, 0xFF], [0xFF, 0x55, 0xFF], [0x55, 0x55, 0xFF], [0xFF, 0xFF, 0x55], [0x00, 0x2A, 0x2A]),
    scheme("SUNSET", [0xF7, 0xF7, 0xF2], [0xB5, 0xA8, 0x9A], [0xFF, 0x6B, 0x35], [0xEE, 0x42, 0x66], [0x3B, 0xCE, 0xAC], [0xFF, 0xD2, 0x3F], [0x2B, 0x0F, 0x0A]),
    scheme("ICE", [0xE8, 0xF4, 0xFF], [0x8F, 0xA8, 0xC0], [0x5C, 0xC8, 0xFF], [0xFF, 0x5C, 0x8A], [0xB0, 0x8C, 0xFF], [0xFF, 0xFF, 0xFF], [0x0A, 0x1A, 0x2A]),
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
