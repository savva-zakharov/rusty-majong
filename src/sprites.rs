//! PNG sprite loading. Two images drive the look of every tile:
//!
//! * `assets/tile.png`  — the blank tile body (face + iso sides), 32x34, drawn in
//!   only white / grey (127) / black. It is coloured in software by multiplying
//!   with a tint per state (normal / hover / selected), see `palette::Tint`.
//! * `assets/faces.png` — a 9x5 grid of 23x29 face pictures in palette colours.
//!
//! Pixels with alpha < 128 are transparent. Tile pixels snap to the nearest of
//! the three tones by brightness; face pixels snap to the nearest palette colour.
//! Missing or wrong-sized files fall back to the built-in art, which is also what
//! `--export-assets` writes out as placeholders.

use crate::art::{self, Face, FH, FW};
use crate::fb::Fb;
use crate::palette::*;
use macroquad::prelude::{Image, ImageFormat};
use std::path::Path;

pub const TRANSPARENT: u8 = 255;

pub const TILE_W: i32 = 32;
pub const TILE_H: i32 = 34;
/// Where the face's top-left sits inside tile.png.
pub const FACE_OX: i32 = 8;
pub const FACE_OY: i32 = 0;
pub const SIDE_STEPS: i32 = 4; // 2:1 iso steps of side depth in the built-in body

pub const SHEET_COLS: i32 = 9;
pub const SHEET_ROWS: i32 = 5;

/// One pixel of a three-level tile sprite.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tone {
    Clear,
    Black,
    Grey,
    White,
}

impl Tone {
    fn from_rgba(p: &[u8]) -> Tone {
        if p[3] < 128 {
            return Tone::Clear;
        }
        let l = (299 * p[0] as u32 + 587 * p[1] as u32 + 114 * p[2] as u32 + 500) / 1000;
        match l {
            0..=63 => Tone::Black,
            64..=191 => Tone::Grey,
            _ => Tone::White,
        }
    }

    fn rgba(self) -> [u8; 4] {
        match self {
            Tone::Clear => [0, 0, 0, 0],
            Tone::Black => [0, 0, 0, 255],
            Tone::Grey => [GREY, GREY, GREY, 255],
            Tone::White => [255, 255, 255, 255],
        }
    }
}

pub struct ToneSprite {
    pub w: i32,
    pub h: i32,
    pub px: Vec<Tone>,
}

impl ToneSprite {
    pub fn opaque(&self, x: i32, y: i32) -> bool {
        x >= 0 && y >= 0 && x < self.w && y < self.h && self.px[(y * self.w + x) as usize] != Tone::Clear
    }
}

pub struct Sprite {
    pub w: i32,
    pub h: i32,
    pub px: Vec<u8>,
}

impl Sprite {
    pub fn get(&self, x: i32, y: i32) -> u8 {
        if x >= 0 && y >= 0 && x < self.w && y < self.h {
            self.px[(y * self.w + x) as usize]
        } else {
            TRANSPARENT
        }
    }

    fn crop(fb: &Fb, x: i32, y: i32, w: i32, h: i32) -> Sprite {
        let mut px = Vec::with_capacity((w * h) as usize);
        for yy in y..y + h {
            for xx in x..x + w {
                px.push(fb.px[(yy * fb.w + xx) as usize]);
            }
        }
        Sprite { w, h, px }
    }
}

/// Grid cell of each face in faces.png.
/// Row 0 dots 1-9, row 1 bamboo 1-9, row 2 characters 1-9,
/// row 3 winds E S W N + dragons red green white, row 4 flowers 1-4 + seasons 1-4.
pub fn face_cell(f: Face) -> (i32, i32) {
    match f {
        Face::Dots(n) => (n as i32 - 1, 0),
        Face::Bam(n) => (n as i32 - 1, 1),
        Face::Chr(n) => (n as i32 - 1, 2),
        Face::Wind(n) => (n as i32 - 1, 3),
        Face::Dragon(n) => (n as i32 + 3, 3),
        Face::Flower(n) => (n as i32 - 1, 4),
        Face::Season(n) => (n as i32 + 3, 4),
    }
}

pub fn all_faces() -> Vec<Face> {
    let mut v = vec![];
    for n in 1..=9 {
        v.extend([Face::Dots(n), Face::Bam(n), Face::Chr(n)]);
    }
    for n in 1..=4 {
        v.extend([Face::Wind(n), Face::Flower(n), Face::Season(n)]);
    }
    for n in 1..=3 {
        v.push(Face::Dragon(n));
    }
    v
}

pub struct Assets {
    pub tile: ToneSprite,
    faces: Vec<Sprite>, // indexed by row * SHEET_COLS + col
    pub status: String,
}

impl Assets {
    pub fn face(&self, f: Face) -> &Sprite {
        let (c, r) = face_cell(f);
        &self.faces[(r * SHEET_COLS + c) as usize]
    }

    pub fn builtin() -> Assets {
        Assets { tile: builtin_tile(), faces: split_sheet(&builtin_sheet()), status: "BUILT-IN ART".into() }
    }

    /// Build assets from raw PNG bytes (`None` = file absent), falling back to
    /// built-in art per file. Shared by the native and web loaders.
    pub fn from_png_bytes(tile: Option<Vec<u8>>, faces: Option<Vec<u8>>) -> Assets {
        let mut a = Assets::builtin();
        let mut loaded = vec![];
        let mut errors = vec![];

        match tile.map(|b| decode_rgba(&b, "TILE.PNG", TILE_W, TILE_H)).transpose() {
            Ok(Some(rgba)) => {
                let px = rgba.chunks_exact(4).map(Tone::from_rgba).collect();
                a.tile = ToneSprite { w: TILE_W, h: TILE_H, px };
                loaded.push("TILE.PNG");
            }
            Ok(None) => {}
            Err(e) => errors.push(e),
        }
        let (sw, sh) = (SHEET_COLS * FW, SHEET_ROWS * FH);
        match faces.map(|b| decode_rgba(&b, "FACES.PNG", sw, sh)).transpose() {
            Ok(Some(rgba)) => {
                let px = rgba
                    .chunks_exact(4)
                    .map(|p| if p[3] < 128 { TRANSPARENT } else { nearest(p[0], p[1], p[2]) })
                    .collect();
                a.faces = split_sheet(&Sprite { w: sw, h: sh, px });
                loaded.push("FACES.PNG");
            }
            Ok(None) => {}
            Err(e) => errors.push(e),
        }

        for e in &errors {
            eprintln!("rusty-majong: {e}");
        }
        a.status = if let Some(e) = errors.first() {
            format!("ERR: {e}")
        } else if loaded.is_empty() {
            "NO ASSETS FOUND, USING BUILT-IN ART.".into()
        } else {
            format!("LOADED {}.", loaded.join(" + "))
        };
        a
    }

    /// Load a tile set with macroquad's file loader (the file system on desktop,
    /// an HTTP fetch relative to the page on the web). Each file comes from the
    /// set's folder, else from the root `assets/` folder, else the built-in art.
    pub async fn load_set(root: &str, set: &TileSet) -> Assets {
        let Some(dir) = &set.dir else {
            let mut a = Assets::builtin();
            a.status = "TILES: BUILT-IN.".into();
            return a;
        };
        let fetch = |d: &str, name: &str| format!("{}/{name}", d.trim_end_matches('/'));
        let mut files = vec![];
        for name in ["tile.png", "faces.png"] {
            let mut bytes = macroquad::file::load_file(&fetch(dir, name)).await.ok();
            if bytes.is_none() && dir != root {
                bytes = macroquad::file::load_file(&fetch(root, name)).await.ok();
            }
            files.push(bytes);
        }
        let faces = files.pop().flatten();
        let tile = files.pop().flatten();
        let mut a = Assets::from_png_bytes(tile, faces);
        a.status = format!("TILES: {}. {}", set.name, a.status);
        a
    }

    /// Synchronous file-system loader (tests, tools).
    #[allow(dead_code)]
    pub fn load_dir(dir: &Path) -> Assets {
        let read = |name: &str| std::fs::read(dir.join(name)).ok();
        Assets::from_png_bytes(read("tile.png"), read("faces.png"))
    }
}

/// A selectable tile set. `dir: None` means the built-in art.
#[derive(Clone, Debug, PartialEq)]
pub struct TileSet {
    pub name: String,
    pub dir: Option<String>,
}

/// Tile sets available under `root`: DEFAULT (the root folder itself), then
/// one per line of `root/sets.txt` (folder names under `root/sets/`; blank
/// lines and `#` comments ignored), then BUILT-IN. A list file is used instead
/// of scanning folders because the web build can't list directories.
pub fn parse_set_list(root: &str, list: &str) -> Vec<TileSet> {
    let root = root.trim_end_matches('/');
    let mut sets = vec![TileSet { name: "DEFAULT".into(), dir: Some(root.into()) }];
    for line in list.lines() {
        let name = line.split('#').next().unwrap_or("").trim();
        let safe = !name.is_empty() && name.chars().all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
        if !safe {
            if !name.is_empty() {
                eprintln!("rusty-majong: sets.txt: ignoring {name:?} (use letters, digits, - and _)");
            }
            continue;
        }
        let upper = name.to_ascii_uppercase();
        if sets.iter().any(|s| s.name == upper) {
            continue;
        }
        sets.push(TileSet { name: upper, dir: Some(format!("{root}/sets/{name}")) });
    }
    sets.push(TileSet { name: "BUILT-IN".into(), dir: None });
    sets
}

pub async fn list_sets(root: &str) -> Vec<TileSet> {
    let path = format!("{}/sets.txt", root.trim_end_matches('/'));
    let list = macroquad::file::load_string(&path).await.unwrap_or_default();
    parse_set_list(root, &list)
}

/// Decode PNG bytes to RGBA, checking the size.
fn decode_rgba(bytes: &[u8], name: &str, w: i32, h: i32) -> Result<Vec<u8>, String> {
    let img = Image::from_file_with_format(bytes, Some(ImageFormat::Png)).map_err(|e| format!("{name}: {e:?}"))?;
    if img.width as i32 != w || img.height as i32 != h {
        return Err(format!("{name} IS {}X{}, EXPECTED {w}X{h}", img.width, img.height));
    }
    Ok(img.bytes)
}

fn nearest(r: u8, g: u8, b: u8) -> u8 {
    let d = |c: &[u8; 3]| {
        let dr = c[0] as i32 - r as i32;
        let dg = c[1] as i32 - g as i32;
        let db = c[2] as i32 - b as i32;
        dr * dr + dg * dg + db * db
    };
    (0..PALETTE.len()).min_by_key(|&i| d(&PALETTE[i])).unwrap() as u8
}

fn split_sheet(sheet: &Sprite) -> Vec<Sprite> {
    let mut v = vec![];
    for r in 0..SHEET_ROWS {
        for c in 0..SHEET_COLS {
            let mut px = Vec::with_capacity((FW * FH) as usize);
            for y in 0..FH {
                for x in 0..FW {
                    px.push(sheet.get(c * FW + x, r * FH + y));
                }
            }
            v.push(Sprite { w: FW, h: FH, px });
        }
    }
    v
}

/// Placeholder tile in the three tones: white outline on a black face,
/// grey 2:1 iso sides. Tinted green / amber / red at draw time.
fn builtin_tile() -> ToneSprite {
    let mut px = vec![Tone::Clear; (TILE_W * TILE_H) as usize];
    let mut rect = |x: i32, y: i32, w: i32, h: i32, t: Tone| {
        for yy in y.max(0)..(y + h).min(TILE_H) {
            for xx in x.max(0)..(x + w).min(TILE_W) {
                px[(yy * TILE_W + xx) as usize] = t;
            }
        }
    };
    let (x, y) = (FACE_OX, FACE_OY);
    for t in (1..=SIDE_STEPS).rev() {
        rect(x - 2 * t, y + t, FW + 1, FH + 1, Tone::Grey);
    }
    rect(x, y, FW + 1, FH + 1, Tone::Grey); // 1px rim so neighbouring faces never merge
    rect(x, y, FW, FH, Tone::White); // face outline
    rect(x + 1, y + 1, FW - 2, FH - 2, Tone::Black); // face
    ToneSprite { w: TILE_W, h: TILE_H, px }
}

fn builtin_sheet() -> Sprite {
    let (w, h) = (SHEET_COLS * FW, SHEET_ROWS * FH);
    let mut fb = Fb::new(w, h);
    fb.clear(TRANSPARENT);
    for f in all_faces() {
        let (c, r) = face_cell(f);
        art::draw_face(&mut fb, c * FW, r * FH, f);
    }
    Sprite::crop(&fb, 0, 0, w, h)
}

fn save_rgba(w: i32, h: i32, rgba: impl Fn(i32, i32) -> [u8; 4], path: &Path) {
    let mut img = Image::gen_image_color(w as u16, h as u16, macroquad::prelude::BLANK);
    for y in 0..h {
        for x in 0..w {
            // export_png flips rows bottom-up, so write them pre-flipped.
            let o = (((h - 1 - y) * w + x) * 4) as usize;
            img.bytes[o..o + 4].copy_from_slice(&rgba(x, y));
        }
    }
    img.export_png(&path.to_string_lossy());
}

/// Write the built-in art as editable placeholders (tile.png, faces.png).
/// Existing files are left alone unless `force` is set. Returns what was written.
pub fn export_builtin(dir: &Path, force: bool) -> std::io::Result<Vec<String>> {
    std::fs::create_dir_all(dir)?;
    let mut out = vec![];
    let tile_path = dir.join("tile.png");
    if force || !tile_path.exists() {
        let t = builtin_tile();
        save_rgba(t.w, t.h, |x, y| t.px[(y * t.w + x) as usize].rgba(), &tile_path);
        out.push(tile_path.display().to_string());
    }
    let faces_path = dir.join("faces.png");
    if force || !faces_path.exists() {
        let s = builtin_sheet();
        let px = |x, y| match s.get(x, y) {
            TRANSPARENT => [0, 0, 0, 0],
            p => {
                let c = PALETTE[p as usize];
                [c[0], c[1], c[2], 255]
            }
        };
        save_rgba(s.w, s.h, px, &faces_path);
        out.push(faces_path.display().to_string());
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grey_times_tint_is_in_palette_for_every_scheme() {
        for s in SCHEMES {
            assert_eq!(s.rgb[BLACK as usize], [0, 0, 0], "{}: background must stay black", s.name);
            for t in [TINT_NORMAL, TINT_HOVER, TINT_SELECT] {
                assert_eq!(s.rgb[t.half as usize], mul(s.rgb[t.full as usize], GREY), "{}", s.name);
            }
        }
    }

    #[test]
    fn set_list_parsing() {
        let sets = parse_set_list("assets/", "wireframe\n# comment\n\n chunky # trailing\n../evil\nWIREFRAME\n");
        let names: Vec<&str> = sets.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, ["DEFAULT", "WIREFRAME", "CHUNKY", "BUILT-IN"]);
        assert_eq!(sets[0].dir.as_deref(), Some("assets"));
        assert_eq!(sets[1].dir.as_deref(), Some("assets/sets/wireframe"));
        assert_eq!(sets[3].dir, None);
    }

    #[test]
    fn tones_snap_by_brightness() {
        assert_eq!(Tone::from_rgba(&[255, 255, 255, 255]), Tone::White);
        assert_eq!(Tone::from_rgba(&[127, 127, 127, 255]), Tone::Grey);
        assert_eq!(Tone::from_rgba(&[0, 0, 0, 255]), Tone::Black);
        assert_eq!(Tone::from_rgba(&[255, 255, 255, 0]), Tone::Clear);
    }

    #[test]
    fn every_face_has_a_unique_cell() {
        let mut seen = std::collections::HashSet::new();
        for f in all_faces() {
            let (c, r) = face_cell(f);
            assert!(c >= 0 && c < SHEET_COLS && r >= 0 && r < SHEET_ROWS);
            assert!(seen.insert((c, r)), "{f:?} shares a cell");
        }
        assert_eq!(seen.len(), 42);
    }

    #[test]
    fn png_round_trip_is_lossless() {
        let dir = std::env::temp_dir().join(format!("rusty-majong-test-{}", std::process::id()));
        assert_eq!(export_builtin(&dir, false).unwrap().len(), 2);
        assert!(export_builtin(&dir, false).unwrap().is_empty(), "must not overwrite");
        let a = Assets::load_dir(&dir);
        let b = Assets::builtin();
        assert!(a.status.starts_with("LOADED"), "{}", a.status);
        assert_eq!(a.tile.px, b.tile.px);
        for f in all_faces() {
            assert_eq!(a.face(f).px, b.face(f).px, "{f:?}");
        }
        let _ = std::fs::remove_dir_all(dir);
    }
}
