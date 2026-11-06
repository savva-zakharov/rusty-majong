//! Game state, input handling and rendering into the framebuffer.

use crate::art::{self, Face, FH, FW};
use crate::sprites::{Assets, FACE_OX, FACE_OY, TILE_H, TILE_W};
use crate::board::{self, Nb, Pos};
use crate::fb::Fb;
use crate::palette::*;
use macroquad::input::{is_key_pressed, is_mouse_button_pressed, KeyCode, MouseButton};
use macroquad::audio::{play_sound, PlaySoundParams, Sound};
use macroquad::rand::gen_range;

pub const W: i32 = 560;
pub const H: i32 = 315;

const HX: i32 = 12; // half-tile step in pixels
const HY: i32 = 15;
const DEPTH: i32 = 4; // tile thickness in 2:1 iso steps
const LX: i32 = 2 * DEPTH; // layer offset = one tile thickness
const LY: i32 = DEPTH;
const BOARD_TOP: i32 = 16;
/// Typewriter speed of the command line, in typing steps per second. Each
/// character takes one step; the letter after a space takes `AFTER_SPACE_STEPS`.
const TYPE_CPS: f32 = 70.0;
/// Pause between a space and the next letter, in steps (normal letters take 1).
const AFTER_SPACE_STEPS: f32 = 2.0;
/// Only every Nth letter of a message clicks (spaces always make their sound).
const CLICK_EVERY: usize = 2;
/// Volume of the typing sounds.
const CLICK_VOLUME: f32 = 0.5;

/// Typing sounds: letter clicks and space sounds, one picked at random each time.
#[derive(Default)]
pub struct TypeSounds {
    pub clicks: Vec<Sound>,
    pub spaces: Vec<Sound>,
    /// Played when a pair is matched and removed.
    pub enters: Vec<Sound>,
}

/// Play one sound from `pool` at random (nothing if the pool is empty).
fn play_random(pool: &[Sound]) {
    if !pool.is_empty() {
        let k = gen_range(0, pool.len() as u32) as usize;
        play_sound(&pool[k], PlaySoundParams { looped: false, volume: CLICK_VOLUME });
    }
}

/// How many characters of `msg` the typewriter has placed after `steps` typing
/// steps. A character appears when the steps so far reach its own duration:
/// 1 step, or `AFTER_SPACE_STEPS` if the previous character was a space.
fn chars_shown(msg: &str, steps: f32) -> usize {
    let mut t = 0.0;
    let mut prev_space = false;
    for (n, ch) in msg.chars().enumerate() {
        t += if prev_space { AFTER_SPACE_STEPS } else { 1.0 };
        if t > steps {
            return n;
        }
        prev_space = ch.is_whitespace();
    }
    msg.chars().count()
}
const BOARD_BOT: i32 = H - 28;
const PROMPT_Y: i32 = H - 25;
const RULE_Y: i32 = H - 15;
const KEYS_Y: i32 = H - 10;
/// Top of the clickable button bar (just below the dashed rule).
const BAR_TOP: i32 = RULE_Y + 1;

/// Everything the hotkeys do. The bottom bar's buttons trigger the same
/// actions, so touch screens without a keyboard can reach every option.
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
enum Action {
    Hint,
    Undo,
    Shuffle,
    New,
    Layout,
    Tiles,
    Palette,
    Dim,
    Reload,
    Crt,
    Quit,
}

/// Bottom-bar buttons: label (key letters in [brackets]) and action.
/// QUIT is desktop-only: a browser tab has nothing to quit to.
fn buttons() -> Vec<(&'static str, Action)> {
    let mut b = vec![
        ("[H]INT", Action::Hint),
        ("[U]NDO", Action::Undo),
        ("[S]HUFFLE", Action::Shuffle),
        ("[N]EW", Action::New),
        ("[L]AYOUT", Action::Layout),
        ("[T]ILES", Action::Tiles),
        ("[P]ALETTE", Action::Palette),
        ("[F] DIM", Action::Dim),
        ("[R]ELOAD", Action::Reload),
        ("[C]RT", Action::Crt),
    ];
    if !cfg!(target_arch = "wasm32") {
        b.push(("[ESC] QUIT", Action::Quit));
    }
    b
}

/// One laid-out button: label position plus its (generous) hit area.
struct Button {
    label: &'static str,
    action: Action,
    text_x: i32,
    hit: (i32, i32, i32, i32), // x, y, w, h
}

/// Spread the buttons evenly across the bar. Each hit area spans the full bar
/// height and reaches halfway to its neighbours, so there are no dead gaps.
fn layout_buttons() -> Vec<Button> {
    let b = buttons();
    let widths: Vec<i32> = b.iter().map(|(l, _)| Fb::text_w(l) - 1).collect();
    let total: i32 = widths.iter().sum();
    let n = b.len() as i32;
    let gap = if n > 1 { (W - 8 - total) / (n - 1) } else { 0 };
    let mut x = 4;
    let mut out: Vec<Button> = Vec::with_capacity(b.len());
    for (k, (label, action)) in b.into_iter().enumerate() {
        out.push(Button { label, action, text_x: x, hit: (0, BAR_TOP, 0, H - BAR_TOP) });
        x += widths[k] + gap;
    }
    for k in 0..out.len() {
        let left = if k == 0 { 0 } else { (out[k - 1].text_x + widths[k - 1] + out[k].text_x) / 2 };
        let right = if k + 1 == out.len() { W } else { (out[k].text_x + widths[k] + out[k + 1].text_x) / 2 };
        out[k].hit.0 = left;
        out[k].hit.2 = right - left;
    }
    out
}

#[derive(PartialEq, Clone, Copy)]
enum State {
    Playing,
    Won,
    Stuck,
}

enum Undo {
    Pair(usize, usize),
    Shuffle(Vec<Face>),
}

struct Ghost {
    x: i32,
    y: i32,
    t: f32,
}

struct Particle {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    life: f32,
    c: u8,
}

pub struct Game {
    layout: usize,
    pos: Vec<Pos>,
    nb: Vec<Nb>,
    order: Vec<usize>,
    faces: Vec<Face>,
    alive: Vec<bool>,
    selected: Option<usize>,
    hover: Option<usize>,
    hint: Option<(usize, usize)>,
    hint_t: f32,
    bad: Option<usize>,
    bad_t: f32,
    history: Vec<Undo>,
    ghosts: Vec<Ghost>,
    particles: Vec<Particle>,
    pub crt: bool,
    assets: Assets,
    /// Set by `T` (next set) / `R` (reload): the tile set index the main loop
    /// should load asynchronously and hand back through `set_assets`.
    pub asset_request: Option<usize>,
    bar: Vec<Button>,
    bar_hover: Option<usize>,
    bar_flash: Option<(usize, f32)>,
    tile_sets: Vec<String>,
    tile_set: usize,
    scheme: usize,
    dim: bool,
    elapsed: f32,
    t: f32,
    msg: String,
    msg_t: f32,
    /// How many characters of `msg` have been typed out (and sounded) so far.
    typed: usize,
    /// How many non-space characters of `msg` have been typed (for CLICK_EVERY).
    letters_typed: usize,
    /// Typing sounds; empty = silent.
    sounds: TypeSounds,
    origin: (i32, i32),
    state: State,
    moves: usize,
}

impl Game {
    pub fn new(layout: usize, assets: Assets, tile_sets: Vec<String>, tile_set: usize, scheme: usize) -> Self {
        let mut g = Game {
            layout: 0,
            pos: vec![],
            nb: vec![],
            order: vec![],
            faces: vec![],
            alive: vec![],
            selected: None,
            hover: None,
            hint: None,
            hint_t: 0.0,
            bad: None,
            bad_t: 0.0,
            history: vec![],
            ghosts: vec![],
            particles: vec![],
            crt: true,
            assets,
            asset_request: None,
            bar: layout_buttons(),
            bar_hover: None,
            bar_flash: None,
            tile_sets,
            tile_set,
            scheme: scheme % SCHEMES.len(),
            dim: false,
            elapsed: 0.0,
            t: 0.0,
            msg: String::new(),
            msg_t: 0.0,
            typed: 0,
            letters_typed: 0,
            sounds: TypeSounds::default(),
            origin: (0, 0),
            state: State::Playing,
            moves: 0,
        };
        g.set_layout(layout);
        g
    }

    /// Install a loaded tile set. `tile_sets` is the (possibly re-read) list of set names.
    pub fn set_assets(&mut self, assets: Assets, tile_set: usize, tile_sets: Vec<String>) {
        self.assets = assets;
        self.tile_sets = tile_sets;
        self.tile_set = tile_set;
        self.asset_request = None;
        let s = self.assets.status.clone();
        self.say(&s);
    }

    /// RGB for each palette slot in the active colour scheme.
    pub fn palette(&self) -> &[Rgb; SLOTS] {
        &SCHEMES[self.scheme].rgb
    }

    fn tile_set_name(&self) -> &str {
        self.tile_sets.get(self.tile_set).map(|s| s.as_str()).unwrap_or("?")
    }

    /// Switch to layout `idx` (wrapping) and deal a fresh board on it.
    pub fn set_layout(&mut self, idx: usize) {
        self.layout = idx % board::LAYOUTS.len();
        let pos = (board::LAYOUTS[self.layout].build)();
        self.nb = board::neighbors(&pos);
        let mut order: Vec<usize> = (0..pos.len()).collect();
        // Painter's order: lower layers first, then far (right/top) to near (left/bottom).
        order.sort_by_key(|&i| (pos[i].z, pos[i].gy - pos[i].gx, pos[i].gy));
        self.order = order;

        let (mut minx, mut maxx, mut miny, mut maxy) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
        for p in &pos {
            let x = p.gx * HX + p.z * LX;
            let y = p.gy * HY - p.z * LY;
            minx = minx.min(x - 2 * DEPTH);
            maxx = maxx.max(x + FW);
            miny = miny.min(y);
            maxy = maxy.max(y + FH + DEPTH);
        }
        let ox = (W - (maxx - minx)) / 2 - minx;
        let oy = BOARD_TOP + (BOARD_BOT - BOARD_TOP - (maxy - miny)) / 2 - miny;
        self.origin = (ox, oy);

        self.faces = vec![Face::Dots(1); pos.len()];
        self.pos = pos;
        self.hover = None;
        self.new_game();
    }

    /// Index of a layout by (case-insensitive) name.
    pub fn layout_index(name: &str) -> Option<usize> {
        board::LAYOUTS.iter().position(|l| l.name.eq_ignore_ascii_case(name))
    }

    fn new_game(&mut self) {
        let n = self.pos.len();
        let slots: Vec<usize> = (0..n).collect();
        let dealt = loop {
            if let Some(d) = board::deal(&self.nb, &slots, &art::full_set_pairs()) {
                break d;
            }
        };
        for (i, f) in dealt {
            self.faces[i] = f;
        }
        self.alive = vec![true; n];
        self.selected = None;
        self.hint = None;
        self.bad = None;
        self.history.clear();
        self.ghosts.clear();
        self.particles.clear();
        self.elapsed = 0.0;
        self.state = State::Playing;
        self.refresh_moves();
        let s = format!("{} DEALT. {}", board::LAYOUTS[self.layout].name, self.assets.status);
        self.say(&s);
    }

    fn say(&mut self, s: &str) {
        self.msg = s.to_string();
        self.msg_t = 0.0;
        self.typed = 0;
        self.letters_typed = 0;
    }

    pub fn set_sounds(&mut self, sounds: TypeSounds) {
        self.sounds = sounds;
    }

    /// Characters of the current message revealed by the typewriter so far.
    fn shown_chars(&self) -> usize {
        chars_shown(&self.msg, self.msg_t * TYPE_CPS)
    }

    /// Sound the characters the typewriter placed since last frame: a space
    /// sound for a space, a click for every CLICK_EVERY-th letter. At most one
    /// sound per frame (a space wins).
    fn type_clicks(&mut self) {
        let shown = self.shown_chars();
        if shown <= self.typed {
            return;
        }
        let (mut space, mut click) = (false, false);
        for ch in self.msg.chars().skip(self.typed).take(shown - self.typed) {
            if ch.is_whitespace() {
                space = true;
            } else {
                click |= self.letters_typed % CLICK_EVERY == 0;
                self.letters_typed += 1;
            }
        }
        self.typed = shown;
        if space {
            play_random(&self.sounds.spaces);
        } else if click {
            play_random(&self.sounds.clicks);
        }
    }

    fn free(&self, i: usize) -> bool {
        board::is_free(&self.nb, &self.alive, i)
    }

    fn screen_pos(&self, i: usize) -> (i32, i32) {
        let p = self.pos[i];
        (
            self.origin.0 + p.gx * HX + p.z * LX,
            self.origin.1 + p.gy * HY - p.z * LY,
        )
    }

    fn remaining(&self) -> usize {
        self.alive.iter().filter(|&&a| a).count()
    }

    fn find_moves(&self) -> Vec<(usize, usize)> {
        let free: Vec<usize> = (0..self.pos.len()).filter(|&i| self.free(i)).collect();
        let mut out = vec![];
        for (k, &a) in free.iter().enumerate() {
            for &b in &free[k + 1..] {
                if self.faces[a].matches(self.faces[b]) {
                    out.push((a, b));
                }
            }
        }
        out
    }

    fn refresh_moves(&mut self) {
        self.moves = self.find_moves().len();
    }

    fn pick(&self, mx: i32, my: i32) -> Option<usize> {
        // Topmost tile whose sprite has an opaque pixel under the cursor.
        for &i in self.order.iter().rev() {
            if !self.alive[i] {
                continue;
            }
            let (sx, sy) = self.screen_pos(i);
            let (lx, ly) = (mx - (sx - FACE_OX), my - (sy - FACE_OY));
            if lx >= 0 && ly >= 0 && lx < TILE_W && ly < TILE_H && self.assets.tile.opaque(lx, ly) {
                return Some(i);
            }
        }
        None
    }

    /// Returns false when the player wants to quit.
    pub fn update(&mut self, dt: f32, mouse: (f32, f32)) -> bool {
        self.t += dt;
        self.msg_t += dt;
        self.type_clicks();
        if self.state == State::Playing {
            self.elapsed += dt;
        }
        self.hint_t -= dt;
        if self.hint_t <= 0.0 {
            self.hint = None;
        }
        self.bad_t -= dt;
        if self.bad_t <= 0.0 {
            self.bad = None;
        }
        for g in &mut self.ghosts {
            g.t += dt;
        }
        self.ghosts.retain(|g| g.t < 0.35);
        for p in &mut self.particles {
            p.vy += 260.0 * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);

        let (mx, my) = (mouse.0.floor() as i32, mouse.1.floor() as i32);
        self.hover = self.pick(mx, my);

        let keys: [(&[KeyCode], Action); 11] = [
            (&[KeyCode::H], Action::Hint),
            (&[KeyCode::U, KeyCode::Z, KeyCode::Backspace], Action::Undo),
            (&[KeyCode::S], Action::Shuffle),
            (&[KeyCode::N], Action::New),
            (&[KeyCode::L], Action::Layout),
            (&[KeyCode::T], Action::Tiles),
            (&[KeyCode::P], Action::Palette),
            (&[KeyCode::F], Action::Dim),
            (&[KeyCode::R], Action::Reload),
            (&[KeyCode::C], Action::Crt),
            (&[KeyCode::Escape], Action::Quit),
        ];
        for (codes, action) in keys {
            if codes.iter().any(|&k| is_key_pressed(k)) && !self.run(action) {
                return false;
            }
        }

        // Bottom bar: hover highlight, and clicks/taps run the button's action.
        self.bar_hover = self.bar.iter().position(|b| {
            let (x, y, w, h) = b.hit;
            mx >= x && mx < x + w && my >= y && my < y + h
        });
        if let Some((_, t)) = &mut self.bar_flash {
            *t -= dt;
        }
        self.bar_flash = self.bar_flash.filter(|&(_, t)| t > 0.0);
        if let Some(k) = self.bar_hover {
            self.hover = None; // the bar is not part of the board
            if is_mouse_button_pressed(MouseButton::Left) {
                self.bar_flash = Some((k, 0.15));
                let action = self.bar[k].action;
                return self.run(action);
            }
        }

        if is_mouse_button_pressed(MouseButton::Left) && self.state != State::Won {
            self.click(self.hover);
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected = None;
        }
        true
    }

    /// Perform an action from a hotkey or a bar button. Returns false to quit.
    fn run(&mut self, action: Action) -> bool {
        match action {
            Action::Hint => self.show_hint(),
            Action::Undo => self.undo(),
            Action::Shuffle => self.reshuffle(),
            Action::New => self.new_game(),
            Action::Layout => self.set_layout(self.layout + 1),
            Action::Tiles => self.asset_request = Some((self.tile_set + 1) % self.tile_sets.len().max(1)),
            Action::Palette => {
                self.scheme = (self.scheme + 1) % SCHEMES.len();
                let s = format!("COLOURS: {}.", SCHEMES[self.scheme].name);
                self.say(&s);
            }
            Action::Dim => {
                self.dim = !self.dim;
                self.say(if self.dim { "BLOCKED TILES DIMMED." } else { "BLOCKED TILES SHOWN NORMALLY." });
            }
            Action::Reload => self.asset_request = Some(self.tile_set),
            Action::Crt => {
                self.crt = !self.crt;
                self.say(if self.crt { "CRT SCANLINES ON." } else { "CRT SCANLINES OFF." });
            }
            Action::Quit => return cfg!(target_arch = "wasm32"), // no-op in the browser
        }
        true
    }

    fn click(&mut self, target: Option<usize>) {
        let Some(i) = target else {
            self.selected = None;
            return;
        };
        if !self.free(i) {
            self.bad = Some(i);
            self.bad_t = 0.45;
            self.say("ERR: TILE IS BLOCKED.");
            return;
        }
        match self.selected {
            Some(j) if j == i => {
                self.selected = None;
                self.say("DESELECTED.");
            }
            Some(j) if self.faces[i].matches(self.faces[j]) => self.remove_pair(i, j),
            _ => {
                self.selected = Some(i);
                let s = format!("SELECTED {}", self.faces[i].name());
                self.say(&s);
            }
        }
    }

    fn remove_pair(&mut self, a: usize, b: usize) {
        self.alive[a] = false;
        self.alive[b] = false;
        self.history.push(Undo::Pair(a, b));
        self.selected = None;
        self.hint = None;
        play_random(&self.sounds.enters);
        for i in [a, b] {
            let (sx, sy) = self.screen_pos(i);
            self.ghosts.push(Ghost { x: sx, y: sy, t: 0.0 });
            let ink = self.faces[i].ink();
            for _ in 0..26 {
                let c = [AMBER, IVORY, GREEN, ink][gen_range(0, 4) as usize];
                self.particles.push(Particle {
                    x: (sx + gen_range(0, FW)) as f32,
                    y: (sy + gen_range(0, FH)) as f32,
                    vx: gen_range(-70.0, 70.0),
                    vy: gen_range(-130.0, -20.0),
                    life: gen_range(0.35, 0.9),
                    c,
                });
            }
        }
        self.refresh_moves();
        let left = self.remaining();
        if left == 0 {
            self.state = State::Won;
            self.say("ALL TILES CLEARED. WELL PLAYED.");
        } else if self.moves == 0 {
            self.state = State::Stuck;
            self.say("NO MOVES LEFT. [S]HUFFLE, [U]NDO OR [N]EW.");
        } else {
            let s = format!("MATCH. {} TILES LEFT, {} MOVES OPEN.", left, self.moves);
            self.say(&s);
        }
    }

    fn show_hint(&mut self) {
        if self.state != State::Playing {
            return;
        }
        let moves = self.find_moves();
        if moves.is_empty() {
            self.say("NO MOVES. PRESS [S] TO SHUFFLE.");
            return;
        }
        let m = moves[gen_range(0, moves.len() as u32) as usize];
        self.hint = Some(m);
        self.hint_t = 2.5;
        let s = format!("HINT: {}", self.faces[m.0].name());
        self.say(&s);
    }

    fn undo(&mut self) {
        match self.history.pop() {
            Some(Undo::Pair(a, b)) => {
                self.alive[a] = true;
                self.alive[b] = true;
                self.say("UNDO: PAIR RESTORED.");
            }
            Some(Undo::Shuffle(f)) => {
                self.faces = f;
                self.say("UNDO: SHUFFLE REVERTED.");
            }
            None => {
                self.say("NOTHING TO UNDO.");
                return;
            }
        }
        self.selected = None;
        self.hint = None;
        self.state = State::Playing;
        self.refresh_moves();
        if self.moves == 0 {
            self.state = State::Stuck;
        }
    }

    fn reshuffle(&mut self) {
        if self.state == State::Won {
            return;
        }
        let slots: Vec<usize> = (0..self.pos.len()).filter(|&i| self.alive[i]).collect();
        let mut faces: Vec<Face> = slots.iter().map(|&i| self.faces[i]).collect();
        faces.sort_by_key(|f| f.key());
        let pairs: Vec<(Face, Face)> = faces.chunks(2).map(|c| (c[0], c[1])).collect();
        match board::deal(&self.nb, &slots, &pairs) {
            Some(d) => {
                self.history.push(Undo::Shuffle(self.faces.clone()));
                for (i, f) in d {
                    self.faces[i] = f;
                }
                self.selected = None;
                self.hint = None;
                self.state = State::Playing;
                self.refresh_moves();
                self.say("SHUFFLED. REMAINING TILES REARRANGED (SOLVABLE).");
            }
            None => self.say("ERR: NO SOLVABLE ARRANGEMENT EXISTS. [U]NDO OR [N]EW."),
        }
    }

    /// Put the board into a mid-game state for screenshots.
    pub fn demo_setup(&mut self, pairs: usize) {
        for _ in 0..pairs {
            let m = self.find_moves();
            if let Some(&(a, b)) = m.first() {
                self.remove_pair(a, b);
            }
        }
        self.ghosts.clear();
        self.particles.clear();
        self.elapsed = 83.0;
        if self.state != State::Playing {
            return;
        }
        if let Some(&(a, b)) = self.find_moves().last() {
            self.selected = Some(a);
            self.hint = Some((a, b));
            self.hint_t = 99.0;
            let s = format!("SELECTED {}", self.faces[a].name());
            self.say(&s);
            self.msg_t = 99.0;
        }
        // Show a blocked-tile flash too (topmost blocked tile in draw order).
        if let Some(&i) = self.order.iter().rev().find(|&&i| self.alive[i] && !self.free(i)) {
            self.bad = Some(i);
            self.bad_t = 99.0;
        }
    }

    pub fn set_dim(&mut self, on: bool) {
        self.dim = on;
    }

    // ------------------------------------------------------------ render

    pub fn render(&self, fb: &mut Fb) {
        fb.clear(BG);

        // Isometric dot lattice on the "desk".
        for (r, y) in (BOARD_TOP + 2..BOARD_BOT).step_by(5).enumerate() {
            let off = if r % 2 == 1 { 5 } else { 0 };
            for x in (off..W).step_by(10) {
                fb.pset(x, y, DIM);
            }
        }

        let blink = ((self.t * 5.0) as i32) % 2 == 0;
        for &i in &self.order {
            if !self.alive[i] {
                continue;
            }
            let (sx, sy) = self.screen_pos(i);
            let free = self.free(i);
            let sel = self.selected == Some(i);
            let hinted = self.hint.map_or(false, |(a, b)| a == i || b == i) && blink;
            let bad = self.bad == Some(i) && ((self.t * 12.0) as i32) % 2 == 0;
            let hover = self.hover == Some(i) && free && !sel;
            // Raised tiles cast a dithered shadow up-right onto the layer below.
            if self.pos[i].z > 0 {
                let nb = &self.nb[i];
                if !nb.right.iter().any(|&j| self.alive[j]) {
                    fb.shade(sx + FW + 1, sy - 2, 3, FH);
                }
                if !nb.up.iter().any(|&j| self.alive[j]) {
                    fb.shade(sx + 1, sy - 2, FW, 2);
                }
            }
            draw_tile(fb, &self.assets, sx, sy, self.faces[i], sel, self.dim && !free, hover, hinted, bad);
        }

        for g in &self.ghosts {
            let k = (g.t * 40.0) as i32;
            let c = if g.t < 0.12 { AMBER } else if g.t < 0.24 { GREEN } else { DKGREEN };
            if g.t < 0.06 {
                fb.rect(g.x, g.y, FW, FH, IVORY);
            }
            fb.outline(g.x - k, g.y - k, FW + 2 * k, FH + 2 * k, c);
        }
        for p in &self.particles {
            let s = if p.life > 0.5 { 2 } else { 1 };
            fb.rect(p.x as i32, p.y as i32, s, s, p.c);
        }

        self.render_hud(fb);

        match self.state {
            State::Won => {
                let t = format!("TIME {}", fmt_time(self.elapsed));
                dialog(fb, "** ALL TILES CLEARED **", &[&t, "", "[N] NEW BOARD   [ESC] QUIT"]);
            }
            State::Stuck => dialog(
                fb,
                "-- NO MOVES LEFT --",
                &["THE BOARD IS LOCKED.", "", "[S]HUFFLE  [U]NDO  [N]EW"],
            ),
            State::Playing => {}
        }
    }

    fn render_hud(&self, fb: &mut Fb) {
        let x = fb.text(4, 4, "RUSTY-MAJONG", AMBER);
        let name = format!(
            "V0.3 {} / {} / {}",
            board::LAYOUTS[self.layout].name,
            self.tile_set_name(),
            SCHEMES[self.scheme].name
        );
        fb.text(x + 6, 4, &name, DKGREEN);
        let stats = format!(
            "TILES {:3} | MOVES {:2} | TIME {}",
            self.remaining(),
            self.moves,
            fmt_time(self.elapsed)
        );
        fb.text(W - 4 - Fb::text_w(&stats) + 1, 4, &stats, GREEN);
        dashed(fb, 13);

        // Prompt line with typewriter reveal and blinking block cursor.
        let shown = self.shown_chars();
        let line: String = self.msg.chars().take(shown).collect();
        let x = fb.text(4, PROMPT_Y, ">", AMBER);
        let x = fb.text_keys(x + 6, PROMPT_Y, &line, GREEN, AMBER);
        if ((self.t * 2.5) as i32) % 2 == 0 {
            fb.text(x, PROMPT_Y, "█", GREEN);
        }

        dashed(fb, RULE_Y);
        for (k, b) in self.bar.iter().enumerate() {
            let w = Fb::text_w(b.label) - 1;
            let flash = self.bar_flash.map_or(false, |(f, _)| f == k);
            if flash {
                // Pressed: inverse video in the highlight colour.
                fb.rect(b.text_x - 2, KEYS_Y - 2, w + 4, 11, AMBER);
                fb.text_keys_br(b.text_x, KEYS_Y, b.label, BG, BG, BG);
            } else if self.bar_hover == Some(k) {
                fb.rect(b.text_x - 2, KEYS_Y - 2, w + 4, 11, DKGREEN);
                fb.text_keys_br(b.text_x, KEYS_Y, b.label, GREEN, AMBER, GREEN);
            } else {
                fb.text_keys(b.text_x, KEYS_Y, b.label, DKGREEN, AMBER);
            }
        }
    }
}

/// Where to load art from. On the web: `assets/` next to index.html.
/// On desktop: `assets/` in the working directory, else next to the executable.
pub fn assets_dir() -> String {
    if cfg!(target_arch = "wasm32") || std::path::Path::new("assets").is_dir() {
        return "assets".into();
    }
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("assets")))
        .filter(|d| d.is_dir())
        .map(|d| d.to_string_lossy().into_owned())
        .unwrap_or_else(|| "assets".into())
}

fn fmt_time(t: f32) -> String {
    let s = t as u32;
    format!("{:02}:{:02}", s / 60, s % 60)
}

fn dashed(fb: &mut Fb, y: i32) {
    let mut x = 0;
    while x < W {
        fb.hline(x, y, 4, DKGREEN);
        x += 6;
    }
}

fn dialog(fb: &mut Fb, title: &str, lines: &[&str]) {
    let w = 252;
    let h = 22 + lines.len() as i32 * 10 + 8;
    let x = (W - w) / 2;
    let y = (BOARD_TOP + BOARD_BOT) / 2 - h / 2;
    fb.rect(x + 6, y + 4, w, h, DKGREEN); // drop shadow
    fb.rect(x, y, w, h, BG);
    fb.outline(x, y, w, h, GREEN);
    fb.outline(x + 2, y + 2, w - 4, h - 4, GREEN);
    fb.text_center(W / 2, y + 8, title, AMBER, AMBER);
    for (k, l) in lines.iter().enumerate() {
        fb.text_center(W / 2, y + 22 + k as i32 * 10, l, GREEN, AMBER);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tile(
    fb: &mut Fb,
    assets: &Assets,
    x: i32,
    y: i32,
    face: Face,
    sel: bool,
    dim: bool,
    hover: bool,
    hint: bool,
    bad: bool,
) {
    // (x, y) is the face's top-left; tile.png is anchored so its face lands there.
    // The three-level tile sprite is multiplied by a tint per state.
    // `bad` is already blink-gated by the caller, so a blocked click flashes the whole tile red.
    let tint = if bad || sel {
        TINT_SELECT
    } else if hover {
        TINT_HOVER
    } else {
        TINT_NORMAL
    };
    let (tx, ty) = (x - FACE_OX, y - FACE_OY);
    fb.blit_tinted(tx, ty, &assets.tile, tint);
    fb.blit(x, y, assets.face(face));
    if dim {
        // Dissolve the whole tile silhouette (sides included), not just the face.
        fb.dither_mask(tx, ty, &assets.tile, BG);
    }
    if hint {
        fb.outline(x, y, FW, FH, AMBER);
        fb.outline(x + 1, y + 1, FW - 2, FH - 2, AMBER);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letter_after_a_space_takes_twice_as_long() {
        // "AB CD": A@1, B@2, ' '@3, C@5 (doubled), D@6
        let at = |steps: f32| chars_shown("AB CD", steps);
        assert_eq!([at(0.5), at(1.0), at(2.0), at(3.0), at(4.9), at(5.0), at(6.0)], [0, 1, 2, 3, 3, 4, 5]);
        assert_eq!(at(100.0), 5);
    }

    #[test]
    fn bar_buttons_tile_the_bar_without_gaps() {
        let bar = layout_buttons();
        assert_eq!(bar.len(), buttons().len());
        assert_eq!(bar[0].hit.0, 0);
        assert_eq!(bar.last().map(|b| b.hit.0 + b.hit.2), Some(W));
        for pair in bar.windows(2) {
            assert_eq!(pair[0].hit.0 + pair[0].hit.2, pair[1].hit.0, "gap/overlap between {} and {}", pair[0].label, pair[1].label);
        }
        for b in &bar {
            let (x, y, w, h) = b.hit;
            let tw = Fb::text_w(b.label) - 1;
            assert!(b.text_x >= x && b.text_x + tw <= x + w, "{} label outside its hit area", b.label);
            assert!(y == BAR_TOP && y + h == H && KEYS_Y >= y && KEYS_Y + 7 <= H);
        }
    }
}
