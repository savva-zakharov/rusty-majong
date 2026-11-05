//! A small, loud palette. Everything on screen is one of these indices.
//!
//! Tile sprites are three-level (white / grey / black) and are coloured by
//! multiplying with a tint: white x tint = tint, grey (127) x tint = the tint's
//! dark entry below, black stays black. The dark entries are computed with that
//! exact multiplication so the result is always a palette colour.

pub const BLACK: u8 = 0;
pub const IVORY: u8 = 1; // tile face (built-in faces), light text
pub const IVORY_SH: u8 = 2; // shaded ivory
pub const GREEN: u8 = 3; // phosphor green: text + normal tile tint
pub const DKGREEN: u8 = 4; // GREEN x grey
pub const RED: u8 = 5; // selection tint
pub const BLUE: u8 = 6;
pub const AMBER: u8 = 7; // hover tint / highlights
pub const DIM: u8 = 8; // background lattice
pub const DKRED: u8 = 9; // RED x grey
pub const DKAMBER: u8 = 10; // AMBER x grey

/// Grey level of the middle tone in three-level sprites.
pub const GREY: u8 = 127;

const GREEN_RGB: [u8; 3] = [0x2E, 0xD8, 0x6A];
const RED_RGB: [u8; 3] = [0xEE, 0x25, 0x3A];
const AMBER_RGB: [u8; 3] = [0xFF, 0xB0, 0x00];

/// Software multiply of a colour by a grey level (0..=255).
pub const fn mul(c: [u8; 3], g: u8) -> [u8; 3] {
    let g = g as u16;
    [
        ((c[0] as u16 * g + 127) / 255) as u8,
        ((c[1] as u16 * g + 127) / 255) as u8,
        ((c[2] as u16 * g + 127) / 255) as u8,
    ]
}

pub const PALETTE: [[u8; 3]; 11] = [
    [0x00, 0x00, 0x00],
    [0xF2, 0xE8, 0xCF],
    [0xBF, 0xAA, 0x80],
    GREEN_RGB,
    mul(GREEN_RGB, GREY),
    RED_RGB,
    [0x2A, 0x5C, 0xF0],
    AMBER_RGB,
    [0x10, 0x2C, 0x1C],
    mul(RED_RGB, GREY),
    mul(AMBER_RGB, GREY),
];

/// A tint for three-level sprites: (colour for white, colour for grey).
#[derive(Clone, Copy)]
pub struct Tint {
    pub full: u8,
    pub half: u8,
}

pub const TINT_NORMAL: Tint = Tint { full: GREEN, half: DKGREEN };
pub const TINT_HOVER: Tint = Tint { full: AMBER, half: DKAMBER };
pub const TINT_SELECT: Tint = Tint { full: RED, half: DKRED };
