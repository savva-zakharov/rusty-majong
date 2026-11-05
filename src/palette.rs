//! A small, loud palette. Everything on screen is one of these indices.

pub const BLACK: u8 = 0;
pub const IVORY: u8 = 1; // tile face
pub const IVORY_SH: u8 = 2; // ivory side band
pub const GREEN: u8 = 3; // phosphor green: text + tile backs
pub const DKGREEN: u8 = 4; // deep green: tile base, rules
pub const RED: u8 = 5;
pub const BLUE: u8 = 6;
pub const AMBER: u8 = 7; // selection / highlights
pub const DIM: u8 = 8; // background lattice

pub const PALETTE: [[u8; 3]; 9] = [
    [0x00, 0x00, 0x00],
    [0xF2, 0xE8, 0xCF],
    [0xBF, 0xAA, 0x80],
    [0x2E, 0xD8, 0x6A],
    [0x0C, 0x5E, 0x34],
    [0xEE, 0x25, 0x3A],
    [0x2A, 0x5C, 0xF0],
    [0xFF, 0xB0, 0x00],
    [0x10, 0x2C, 0x1C],
];
