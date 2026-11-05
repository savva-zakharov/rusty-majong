//! Palette-indexed framebuffer with a few pixel primitives.

use crate::font;
use crate::palette::*;
use crate::sprites::{Sprite, Tone, ToneSprite, TRANSPARENT};

pub struct Fb {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

/// Sprite colour codes: '#'/'p' = ink, 'c' = alt ink, letters = fixed palette.
fn code(ch: char, ink: u8, alt: u8) -> Option<u8> {
    Some(match ch {
        '#' | 'p' => ink,
        'c' => alt,
        'k' => BLACK,
        'w' => IVORY,
        's' => IVORY_SH,
        'g' => GREEN,
        'd' => DKGREEN,
        'r' => RED,
        'b' => BLUE,
        'a' => AMBER,
        _ => return None,
    })
}

impl Fb {
    pub fn new(w: i32, h: i32) -> Self {
        Fb { w, h, px: vec![BLACK; (w * h) as usize] }
    }

    pub fn clear(&mut self, c: u8) {
        self.px.fill(c);
    }

    #[inline]
    pub fn pset(&mut self, x: i32, y: i32, c: u8) {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize] = c;
        }
    }

    #[inline]
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            BLACK
        }
    }

    pub fn rect(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        let x0 = x.max(0);
        let y0 = y.max(0);
        let x1 = (x + w).min(self.w);
        let y1 = (y + h).min(self.h);
        for yy in y0..y1 {
            let row = (yy * self.w) as usize;
            self.px[row + x0 as usize..row + x1.max(x0) as usize].fill(c);
        }
    }

    pub fn hline(&mut self, x: i32, y: i32, w: i32, c: u8) {
        self.rect(x, y, w, 1, c);
    }

    pub fn outline(&mut self, x: i32, y: i32, w: i32, h: i32, c: u8) {
        self.rect(x, y, w, 1, c);
        self.rect(x, y + h - 1, w, 1, c);
        self.rect(x, y, 1, h, c);
        self.rect(x + w - 1, y, 1, h, c);
    }

    /// 50% checkerboard dither over the opaque pixels of a tile sprite.
    pub fn dither_mask(&mut self, x: i32, y: i32, s: &ToneSprite, c: u8) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                if (x + sx + y + sy) & 1 == 0 && s.opaque(sx, sy) {
                    self.pset(x + sx, y + sy, c);
                }
            }
        }
    }

    /// Draw a palette sprite, skipping transparent pixels.
    pub fn blit(&mut self, x: i32, y: i32, s: &Sprite) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let c = s.px[(sy * s.w + sx) as usize];
                if c != TRANSPARENT {
                    self.pset(x + sx, y + sy, c);
                }
            }
        }
    }

    /// Draw a three-level sprite multiplied by `tint`:
    /// white -> tint.full, grey -> tint.half (tint x 127/255), black -> black.
    pub fn blit_tinted(&mut self, x: i32, y: i32, s: &ToneSprite, tint: Tint) {
        for sy in 0..s.h {
            for sx in 0..s.w {
                let c = match s.px[(sy * s.w + sx) as usize] {
                    Tone::White => tint.full,
                    Tone::Grey => tint.half,
                    Tone::Black => BLACK,
                    Tone::Clear => continue,
                };
                self.pset(x + sx, y + sy, c);
            }
        }
    }

    /// Darken a region by shifting each pixel one step down the palette ramp.
    pub fn shade(&mut self, x: i32, y: i32, w: i32, h: i32) {
        for yy in y..y + h {
            for xx in x..x + w {
                let c = match self.get(xx, yy) {
                    IVORY => IVORY_SH,
                    GREEN => DKGREEN,
                    AMBER => DKAMBER,
                    RED => DKRED,
                    c => c,
                };
                self.pset(xx, yy, c);
            }
        }
    }

    pub fn sprite(&mut self, x: i32, y: i32, rows: &[&str], ink: u8, alt: u8) {
        for (j, row) in rows.iter().enumerate() {
            for (i, ch) in row.chars().enumerate() {
                if let Some(c) = code(ch, ink, alt) {
                    self.pset(x + i as i32, y + j as i32, c);
                }
            }
        }
    }

    pub fn sprite_c(&mut self, cx: i32, cy: i32, rows: &[&str], ink: u8, alt: u8) {
        let w = rows.first().map(|r| r.len()).unwrap_or(0) as i32;
        let h = rows.len() as i32;
        self.sprite(cx - w / 2, cy - h / 2, rows, ink, alt);
    }

    pub const CW: i32 = 6; // character cell width

    pub fn text_w(s: &str) -> i32 {
        s.chars().count() as i32 * Self::CW
    }

    pub fn text(&mut self, x: i32, y: i32, s: &str, c: u8) -> i32 {
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = font::glyph(ch.to_ascii_uppercase()) {
                self.sprite(cx, y, &g, c, c);
            }
            cx += Self::CW;
        }
        cx
    }

    /// Text where anything inside [brackets] is drawn in the key colour.
    pub fn text_keys(&mut self, x: i32, y: i32, s: &str, c: u8, kc: u8) -> i32 {
        let mut cx = x;
        let mut inside = false;
        for ch in s.chars() {
            let col = match ch {
                '[' => {
                    inside = true;
                    DKGREEN
                }
                ']' => {
                    inside = false;
                    DKGREEN
                }
                _ if inside => kc,
                _ => c,
            };
            if let Some(g) = font::glyph(ch.to_ascii_uppercase()) {
                self.sprite(cx, y, &g, col, col);
            }
            cx += Self::CW;
        }
        cx
    }

    pub fn text_center(&mut self, cx: i32, y: i32, s: &str, c: u8, kc: u8) {
        let w = Self::text_w(s) - 1;
        self.text_keys(cx - w / 2, y, s, c, kc);
    }

    pub fn tiny(&mut self, x: i32, y: i32, s: &str, c: u8) {
        let mut cx = x;
        for ch in s.chars() {
            if let Some(g) = font::tiny(ch) {
                self.sprite(cx, y, &g, c, c);
            }
            cx += 4;
        }
    }

    pub fn write_rgba(&self, out: &mut [u8]) {
        for (i, &p) in self.px.iter().enumerate() {
            let rgb = PALETTE[p as usize];
            let o = i * 4;
            out[o] = rgb[0];
            out[o + 1] = rgb[1];
            out[o + 2] = rgb[2];
            out[o + 3] = 255;
        }
    }
}
