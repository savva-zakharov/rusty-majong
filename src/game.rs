//! Game state, input handling and rendering into the framebuffer.

use crate::art::{self, Face, FH, FW};
use crate::sprites::{Assets, FACE_OX, FACE_OY, TILE_H, TILE_W};
use crate::board::{self, Nb, Pos};
use crate::fb::Fb;
use crate::palette::{self, *};
use macroquad::input::{is_key_pressed, is_mouse_button_pressed, KeyCode, MouseButton};
use macroquad::audio::{play_sound, PlaySoundParams, Sound};
use macroquad::rand::gen_range;
use std::collections::VecDeque;

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
const CLICK_VOLUME: f32 = 0.3;
/// Beat after a match message finishes typing before its enter sound plays, in seconds.
const ENTER_DELAY: f32 = 0.15;
/// Pause after a message has finished typing before the next queued one starts, in seconds.
const MSG_GAP: f32 = 0.0;
/// Most messages that can wait in the queue; beyond that the oldest waiting one is dropped.
const MSG_QUEUE_MAX: usize = 4;
/// Deal animation: tiles placed per second (144 tiles take about 1.5 s).
const DEAL_TPS: f32 = 96.0;
/// Pause after the last tile lands before the symbols are shown, in seconds.
const DEAL_REVEAL_DELAY: f32 = 0.2;

/// A command-line message waiting its turn.
struct Msg {
    text: String,
    /// Play an enter sound when this message has finished typing (successful match).
    enter: bool,
}

/// Typing sounds: letter clicks and space sounds, one picked at random each time.
#[derive(Default)]
pub struct TypeSounds {
    pub clicks: Vec<Sound>,
    pub spaces: Vec<Sound>,
    /// Played when a pair is matched and removed.
    pub enters: Vec<Sound>,
    /// Played when a selected tile is let go by clicking empty space.
    pub escapes: Vec<Sound>,
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
/// Which typing sound the characters `from..to` of `text` call for: a space
/// sound for a space, a click for every CLICK_EVERY-th letter, at most one
/// (a space wins). `letters` counts the letters typed so far.
fn typing_sound(text: &str, from: usize, to: usize, letters: &mut usize) -> Option<TypeSound> {
    let (mut space, mut click) = (false, false);
    for ch in text.chars().skip(from).take(to.saturating_sub(from)) {
        if ch.is_whitespace() {
            space = true;
        } else {
            click |= *letters % CLICK_EVERY == 0;
            *letters += 1;
        }
    }
    if space {
        Some(TypeSound::Space)
    } else if click {
        Some(TypeSound::Click)
    } else {
        None
    }
}

#[derive(PartialEq, Debug)]
enum TypeSound {
    Click,
    Space,
}

/// A line typed out one character at a time (the command line, the layout name
/// in the new-board dialog).
#[derive(Default)]
struct Typer {
    /// Time since the text started typing.
    t: f32,
    /// Characters placed (and sounded) so far.
    typed: usize,
    /// Letters (non-spaces) placed so far, for CLICK_EVERY.
    letters: usize,
}

impl Typer {
    fn restart(&mut self) {
        *self = Typer::default();
    }

    /// Whether all of `text` has been typed out.
    fn done(&self, text: &str) -> bool {
        self.typed >= text.chars().count()
    }

    /// Advance by `dt`; returns the sound for the characters placed this step.
    fn step(&mut self, text: &str, dt: f32) -> Option<TypeSound> {
        self.t += dt;
        let shown = chars_shown(text, self.t * TYPE_CPS);
        if shown <= self.typed {
            return None;
        }
        let sound = typing_sound(text, self.typed, shown, &mut self.letters);
        self.typed = shown;
        sound
    }
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
    /// New-board dialog: deal the chosen layout.
    NewYes,
    /// New-board dialog: keep the current board.
    NewNo,
    /// New-board dialog: previous / next layout.
    LayoutPrev,
    LayoutNext,
    /// Open the theme dialog (tile set and colour scheme).
    Theme,
    /// Theme dialog: previous / next tile set, previous / next colour scheme.
    SetPrev,
    SetNext,
    SchemePrev,
    SchemeNext,
    /// Theme dialog: keep the choice / leave without changes / reload the art.
    ThemeAccept,
    ThemeExit,
    ThemeReload,
    Dim,
    Crt,
    Mute,
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
        ("[T]HEME", Action::Theme),
        ("[F] DIM", Action::Dim),
        ("[M]UTE", Action::Mute),
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
    text_y: i32,
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
        out.push(Button { label, action, text_x: x, text_y: KEYS_Y, hit: (0, BAR_TOP, 0, H - BAR_TOP) });
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

/// A pop-up dialog: a title, some text lines, and a row of clickable buttons.
struct Dialog {
    title: &'static str,
    lines: Vec<String>,
    buttons: Vec<(&'static str, Action)>,
    /// Layout picker (index of the layout shown): a `< NAME >` row and a
    /// miniature of the layout, between the text lines and the buttons.
    picker: Option<usize>,
    /// Characters of the layout name typed out so far.
    picker_shown: usize,
    /// Theme chooser: a tile preview with arrows round it, and the names.
    theme: Option<ThemeView>,
}

/// What the theme dialog shows: the chosen names, how much of each has typed
/// out, and where each sits in its list.
struct ThemeView {
    set_name: String,
    set_shown: usize,
    set_pos: (usize, usize),
    scheme_name: String,
    scheme_shown: usize,
    scheme_pos: (usize, usize),
    crt: bool,
}

/// The theme being chosen in the theme dialog.
struct ThemeEdit {
    set: usize,
    scheme: usize,
    /// CRT scanlines.
    crt: bool,
    set_typer: Typer,
    scheme_typer: Typer,
}

/// Size of the theme dialog's preview: one tile and a little table around it.
const THEME_BOX_W: i32 = 56;
const THEME_BOX_H: i32 = 48;
/// Height of the theme chooser: up arrow, box, down arrow, gap, three rows
/// (tiles, colours, CRT).
const THEME_H: i32 = 10 + THEME_BOX_H + 12 + 4 + 30;
/// x of the theme rows' labels and values.
const THEME_LABEL_X: i32 = W / 2 - 60;
const THEME_VALUE_X: i32 = W / 2 - 12;
/// The tile shown in the theme preview (the bird: several ink colours).
const THEME_FACE: Face = Face::Bam(1);

const DIALOG_W: i32 = 252;
/// Gap between dialog buttons, in pixels.
const DIALOG_BUTTON_GAP: i32 = 18;
/// Height of the layout picker's miniature, in pixels.
const PREVIEW_H: i32 = 74;
/// Width available to the miniature.
const PREVIEW_W: i32 = DIALOG_W - 24;
/// Distance of the picker's `<` / `>` arrows from the dialog's centre.
const ARROW_DX: i32 = 64;

impl Dialog {
    /// The dialog box: x, y, w, h. Rows: title, lines, a blank, the buttons.
    fn rect(&self) -> (i32, i32, i32, i32) {
        let h = 22 + self.content_h() + 20 + 8;
        ((W - DIALOG_W) / 2, (BOARD_TOP + BOARD_BOT) / 2 - h / 2, DIALOG_W, h)
    }

    /// Height of the text lines plus the picker, if any.
    fn content_h(&self) -> i32 {
        self.lines.len() as i32 * 10
            + if self.picker.is_some() { 14 + PREVIEW_H + 4 } else { 0 }
            + if self.theme.is_some() { THEME_H } else { 0 }
    }

    /// Top of the theme chooser (its up arrow).
    fn theme_y(&self) -> i32 {
        self.picker_y()
    }

    /// y of the theme dialog's first name row (TILES); COLOURS and CRT follow.
    fn theme_rows_y(&self) -> i32 {
        self.theme_y() + 10 + THEME_BOX_H + 16
    }

    /// The theme preview box: x, y, w, h.
    fn theme_box(&self) -> (i32, i32, i32, i32) {
        (W / 2 - THEME_BOX_W / 2, self.theme_y() + 10, THEME_BOX_W, THEME_BOX_H)
    }

    /// y of the picker's `< NAME >` row.
    fn picker_y(&self) -> i32 {
        self.rect().1 + 22 + self.lines.len() as i32 * 10
    }

    fn button_row_y(&self) -> i32 {
        self.rect().1 + 22 + self.content_h() + 10
    }

    /// The buttons centred on their row, each with a padded hit area.
    fn layout_buttons(&self) -> Vec<Button> {
        let widths: Vec<i32> = self.buttons.iter().map(|(l, _)| Fb::text_w(l) - 1).collect();
        let total = widths.iter().sum::<i32>() + DIALOG_BUTTON_GAP * (widths.len() as i32 - 1).max(0);
        let y = self.button_row_y();
        let mut x = W / 2 - total / 2;
        let mut out = vec![];
        for (k, &(label, action)) in self.buttons.iter().enumerate() {
            out.push(Button { label, action, text_x: x, text_y: y, hit: (x - 5, y - 4, widths[k] + 10, 15) });
            x += widths[k] + DIALOG_BUTTON_GAP;
        }
        if self.picker.is_some() {
            // Arrows either side of the layout name; each hit area runs from
            // the dialog's edge to just inside the name, so they're easy to tap.
            let py = self.picker_y();
            let (dx, _, dw, _) = self.rect();
            let (lx, rx) = (W / 2 - ARROW_DX, W / 2 + ARROW_DX);
            out.push(Button { label: "<", action: Action::LayoutPrev, text_x: lx - 2, text_y: py, hit: (dx + 4, py - 4, lx + 10 - (dx + 4), 15) });
            out.push(Button { label: ">", action: Action::LayoutNext, text_x: rx - 2, text_y: py, hit: (rx - 8, py - 4, dx + dw - 4 - (rx - 8), 15) });
        }
        if self.theme.is_some() {
            // Arrows round the preview: left / right change the tile set,
            // up / down the colours. Hit areas fill the space round the box.
            let (dx, _, dw, _) = self.rect();
            let (bx, by, bw, bh) = self.theme_box();
            let cy = by + bh / 2 - 3;
            out.push(Button { label: "^", action: Action::SchemePrev, text_x: W / 2 - 2, text_y: by - 9, hit: (bx, by - 12, bw, 12) });
            out.push(Button { label: "⌄", action: Action::SchemeNext, text_x: W / 2 - 2, text_y: by + bh + 3, hit: (bx, by + bh, bw, 12) });
            out.push(Button { label: "<", action: Action::SetPrev, text_x: bx - 16, text_y: cy, hit: (dx + 4, by, bx - (dx + 4), bh) });
            out.push(Button { label: ">", action: Action::SetNext, text_x: bx + bw + 12, text_y: cy, hit: (bx + bw, by, dx + dw - 4 - (bx + bw), bh) });
            // The CRT row toggles scanlines: its whole width is the button.
            let ry = self.theme_rows_y() + 20;
            out.push(Button { label: "[C]RT", action: Action::Crt, text_x: THEME_LABEL_X, text_y: ry, hit: (THEME_LABEL_X - 5, ry - 3, THEME_VALUE_X + 30 - (THEME_LABEL_X - 5), 12) });
        }
        out
    }
}

/// Draw one button label at (x, y): plain, hovered (highlight box) or pressed
/// (inverse video). `normal` is the label colour when not hovered.
fn draw_button(fb: &mut Fb, label: &str, x: i32, y: i32, normal: u8, hover: bool, pressed: bool) {
    let w = Fb::text_w(label) - 1;
    if pressed {
        fb.rect(x - 2, y - 2, w + 4, 11, AMBER);
        fb.text_keys_br(x, y, label, BG, BG, BG);
    } else if hover {
        fb.rect(x - 2, y - 2, w + 4, 11, DKGREEN);
        fb.text_keys_br(x, y, label, GREEN, AMBER, GREEN);
    } else {
        fb.text_keys(x, y, label, normal, AMBER);
    }
}

fn in_rect(mx: i32, my: i32, (x, y, w, h): (i32, i32, i32, i32)) -> bool {
    mx >= x && mx < x + w && my >= y && my < y + h
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
    /// CRT scanlines (see `scanlines`, which previews the theme dialog's choice).
    crt: bool,
    /// All sounds off (typing and match).
    pub muted: bool,
    assets: Assets,
    /// A tile set the main loop should load and install through `set_assets`
    /// (a theme accepted before its preview had loaded).
    pub asset_request: Option<usize>,
    /// Theme dialog open: the tile set and colours being tried.
    theme: Option<ThemeEdit>,
    /// Tile sets loaded for the theme preview, by index.
    previews: Vec<(usize, Assets)>,
    /// A tile set the main loop should load for the theme preview, and whether
    /// to re-read the set list and palettes first (reload); handed back through
    /// `set_preview`.
    pub preview_request: Option<(usize, bool)>,
    bar: Vec<Button>,
    bar_hover: Option<usize>,
    bar_flash: Option<(usize, f32)>,
    dialog_hover: Option<usize>,
    /// New-board dialog open, showing this layout.
    picker: Option<usize>,
    /// Types out the layout name in the new-board dialog.
    picker_typer: Typer,
    tile_sets: Vec<String>,
    tile_set: usize,
    /// Colour schemes loaded from assets/palettes/ (at least one).
    schemes: Vec<Scheme>,
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
    /// The message on the line plays an enter sound once it has finished typing.
    msg_enter: bool,
    /// Time since the message on the line finished typing.
    msg_done_t: f32,
    /// Messages waiting for the line: each starts MSG_GAP after the previous one
    /// has finished typing.
    queue: VecDeque<Msg>,
    /// Typing sounds; empty = silent.
    sounds: TypeSounds,
    origin: (i32, i32),
    /// Order tiles appear in when a board is dealt: rank of each tile, going
    /// left to right, top to bottom, one layer at a time from the bottom up.
    deal_rank: Vec<usize>,
    /// Time since the current board was dealt (drives the deal animation).
    deal_t: f32,
    /// Tiles the deal animation has already placed (and counted for sounds).
    deal_placed: usize,
    state: State,
    moves: usize,
}

impl Game {
    pub fn new(layout: usize, assets: Assets, tile_sets: Vec<String>, tile_set: usize, schemes: Vec<Scheme>, scheme: usize) -> Self {
        let schemes = if schemes.is_empty() { vec![palette::builtin()] } else { schemes };
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
            muted: false,
            assets,
            asset_request: None,
            theme: None,
            previews: vec![],
            preview_request: None,
            bar: layout_buttons(),
            bar_hover: None,
            bar_flash: None,
            dialog_hover: None,
            picker: None,
            picker_typer: Typer::default(),
            tile_sets,
            tile_set,
            scheme: scheme % schemes.len(),
            schemes,
            dim: false,
            elapsed: 0.0,
            t: 0.0,
            msg: String::new(),
            msg_t: 0.0,
            typed: 0,
            letters_typed: 0,
            msg_enter: false,
            msg_done_t: MSG_GAP, // the (empty) line is free at once
            queue: VecDeque::new(),
            sounds: TypeSounds::default(),
            origin: (0, 0),
            deal_rank: vec![],
            deal_t: 0.0,
            deal_placed: 0,
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

    /// Replace the colour schemes (after a reload), keeping the current one by
    /// name if it still exists.
    pub fn set_schemes(&mut self, schemes: Vec<Scheme>) {
        if schemes.is_empty() {
            return;
        }
        let find = |old: usize| schemes.iter().position(|s| s.name == self.schemes[old].name).unwrap_or(0);
        self.scheme = find(self.scheme);
        if let Some(t) = &self.theme {
            let k = find(t.scheme);
            self.theme.as_mut().unwrap().scheme = k;
        }
        self.schemes = schemes;
    }

    /// Install a tile set loaded for the theme preview. After a reload this also
    /// replaces the list of set names and reports the result.
    pub fn set_preview(&mut self, set: usize, assets: Assets, tile_sets: Vec<String>, reloaded: bool) {
        if reloaded {
            let s = format!("RELOADED. {}", assets.status);
            self.say(&s);
        }
        self.tile_sets = tile_sets;
        self.previews.retain(|(k, _)| *k != set);
        self.previews.push((set, assets));
        if let Some(t) = &mut self.theme {
            t.set = t.set.min(self.tile_sets.len().saturating_sub(1));
        }
    }

    /// The art to preview tile set `set` with, if it has loaded.
    fn preview_assets(&self, set: usize) -> Option<&Assets> {
        match self.previews.iter().find(|(k, _)| *k == set) {
            Some((_, a)) => Some(a),
            None if set == self.tile_set => Some(&self.assets),
            None => None,
        }
    }

    /// RGB for each palette slot in the active colour scheme (the one being
    /// tried while the theme dialog is open, so the whole screen previews it).
    pub fn palette(&self) -> &[Rgb; SLOTS] {
        let k = self.theme.as_ref().map_or(self.scheme, |t| t.scheme);
        &self.schemes[k].rgb
    }

    /// Whether to draw CRT scanlines (the one being tried while the theme dialog is open).
    pub fn scanlines(&self) -> bool {
        self.theme.as_ref().map_or(self.crt, |t| t.crt)
    }

    /// Open the theme dialog on the current tile set and colours.
    pub fn open_theme(&mut self) {
        if self.theme.is_none() {
            self.theme = Some(ThemeEdit {
                set: self.tile_set,
                scheme: self.scheme,
                crt: self.crt,
                set_typer: Typer::default(),
                scheme_typer: Typer::default(),
            });
            self.say("THEME: < > CHANGE THE TILES, ^ ⌄ THE COLOURS.");
        }
    }

    /// Show the theme dialog's names fully typed (screenshots).
    pub fn finish_theme_typing(&mut self) {
        if let Some(t) = &mut self.theme {
            for ty in [&mut t.set_typer, &mut t.scheme_typer] {
                ty.t = 99.0;
                ty.typed = usize::MAX / 2;
            }
        }
    }

    /// Theme dialog: step the tile set by `d` (wrapping), loading it if needed.
    fn theme_step_set(&mut self, d: isize) {
        let n = self.tile_sets.len().max(1) as isize;
        let Some(t) = &mut self.theme else { return };
        t.set = (t.set as isize + d).rem_euclid(n) as usize;
        t.set_typer.restart();
        let set = t.set;
        if self.preview_assets(set).is_none() {
            self.preview_request = Some((set, false));
        }
    }

    /// Theme dialog: step the colour scheme by `d` (wrapping).
    fn theme_step_scheme(&mut self, d: isize) {
        let n = self.schemes.len() as isize;
        if let Some(t) = &mut self.theme {
            t.scheme = (t.scheme as isize + d).rem_euclid(n) as usize;
            t.scheme_typer.restart();
        }
    }

    /// Theme dialog: keep the tile set and colours being tried.
    fn theme_accept(&mut self) {
        let Some(t) = self.theme.take() else { return };
        self.scheme = t.scheme;
        self.crt = t.crt;
        if let Some(k) = self.previews.iter().position(|(i, _)| *i == t.set) {
            self.assets = self.previews.swap_remove(k).1;
            self.tile_set = t.set;
        } else if t.set != self.tile_set {
            self.asset_request = Some(t.set); // still loading: install when it arrives
        }
        self.previews.clear();
        let name = self.tile_sets.get(t.set).cloned().unwrap_or_default();
        let crt = if self.crt { "CRT ON" } else { "CRT OFF" };
        let s = format!("THEME: {} / {} / {}.", name, self.schemes[self.scheme].name, crt);
        self.say(&s);
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
        let mut deal: Vec<usize> = (0..pos.len()).collect();
        deal.sort_by_key(|&i| (pos[i].z, pos[i].gy, pos[i].gx));
        self.deal_rank = vec![0; pos.len()];
        for (rank, &i) in deal.iter().enumerate() {
            self.deal_rank[i] = rank;
        }

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

    /// Open the new-board dialog, starting on the current layout.
    pub fn open_picker(&mut self) {
        if self.picker.is_none() {
            self.picker = Some(self.layout);
            self.picker_typer.restart();
            self.say("NEW BOARD? PICK A LAYOUT WITH < >, THEN [Y]ES OR [N]O.");
        }
    }

    /// How many tiles the deal animation has placed so far.
    fn dealt_count(&self) -> usize {
        (self.deal_t * DEAL_TPS) as usize
    }

    /// True while tiles are still appearing or their symbols are still hidden.
    fn dealing(&self) -> bool {
        self.deal_t < self.pos.len() as f32 / DEAL_TPS + DEAL_REVEAL_DELAY
    }

    /// Jump the deal animation to `t` seconds in (screenshots); a large `t` skips it.
    pub fn set_deal_time(&mut self, t: f32) {
        self.deal_t = t;
        self.deal_placed = self.dealt_count().min(self.pos.len());
    }

    /// True while the command line or the new-board dialog's layout name is
    /// still typing out (and so making its own typing sounds).
    fn text_typing(&self) -> bool {
        let picker = self
            .picker
            .is_some_and(|idx| self.picker_typer.typed < board::LAYOUTS[idx].name.chars().count());
        let theme = self.theme.as_ref().is_some_and(|t| {
            !t.set_typer.done(self.tile_sets.get(t.set).map_or("", |n| n.as_str()))
                || !t.scheme_typer.done(&self.schemes[t.scheme].name)
        });
        !self.msg_finished() || picker || theme
    }

    /// Sound the tiles the deal placed since last frame: a click for every
    /// CLICK_EVERY-th tile, like letters, but only while no text is typing.
    /// Returns whether a click is due (tests).
    fn deal_clicks(&mut self) -> bool {
        let now = self.dealt_count().min(self.pos.len());
        let mut click = false;
        for k in self.deal_placed..now {
            click |= k % CLICK_EVERY == 0;
        }
        self.deal_placed = now;
        let due = click && !self.text_typing();
        if due {
            self.play(&self.sounds.clicks);
        }
        due
    }

    /// Show the new-board dialog's layout name fully typed (screenshots).
    pub fn finish_picker_typing(&mut self) {
        self.picker_typer.t = 99.0;
        self.picker_typer.typed = usize::MAX / 2;
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
        self.deal_t = 0.0;
        self.deal_placed = 0;
        self.state = State::Playing;
        self.refresh_moves();
        let s = format!("{} DEALT. {}", board::LAYOUTS[self.layout].name, self.assets.status);
        self.say(&s);
    }

    /// Queue a message for the command line.
    fn say(&mut self, s: &str) {
        self.queue_msg(s, false);
    }

    /// Queue a message that plays an enter sound once it has finished typing.
    fn say_with_enter(&mut self, s: &str) {
        self.queue_msg(s, true);
    }

    fn queue_msg(&mut self, s: &str, enter: bool) {
        // Repeats are typed again too, so pressing the same button twice
        // answers twice.
        self.queue.push_back(Msg { text: s.to_string(), enter });
        while self.queue.len() > MSG_QUEUE_MAX {
            self.queue.pop_front(); // keep the newest; the line itself always finishes
        }
    }

    fn start_msg(&mut self, m: Msg) {
        self.msg = m.text;
        self.msg_enter = m.enter;
        self.msg_t = 0.0;
        self.typed = 0;
        self.letters_typed = 0;
        self.msg_done_t = 0.0;
    }

    /// Play a random sound from `pool`, unless sound is muted.
    fn play(&self, pool: &[Sound]) {
        if !self.muted {
            play_random(pool);
        }
    }

    fn msg_finished(&self) -> bool {
        self.typed >= self.msg.chars().count()
    }

    /// Advance the command line: type the current message (with its sounds),
    /// play its enter sound after it finishes, and MSG_GAP after it finished
    /// start the next queued message.
    fn step_messages(&mut self, dt: f32) {
        self.msg_t += dt;
        self.type_clicks();
        if !self.msg_finished() {
            return;
        }
        self.msg_done_t += dt;
        if self.msg_enter && self.msg_done_t >= ENTER_DELAY {
            self.play(&self.sounds.enters);
            self.msg_enter = false;
        }
        if self.msg_done_t >= MSG_GAP {
            if let Some(m) = self.queue.pop_front() {
                self.start_msg(m);
            }
        }
    }

    /// Show the newest queued message at once, fully typed (screenshots).
    fn show_latest_now(&mut self) {
        if let Some(m) = self.queue.pop_back() {
            self.queue.clear();
            self.start_msg(m);
        }
        self.msg_t = 99.0;
        self.typed = self.msg.chars().count();
        self.msg_enter = false;
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
        let sound = typing_sound(&self.msg, self.typed, shown, &mut self.letters_typed);
        self.typed = shown;
        self.play_typing(sound);
    }

    fn play_typing(&self, sound: Option<TypeSound>) {
        match sound {
            Some(TypeSound::Space) => self.play(&self.sounds.spaces),
            Some(TypeSound::Click) => self.play(&self.sounds.clicks),
            None => {}
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
        self.step_messages(dt);
        if let Some(idx) = self.picker {
            let sound = self.picker_typer.step(board::LAYOUTS[idx].name, dt);
            self.play_typing(sound);
        }
        if let Some(t) = &mut self.theme {
            // Both names type at once; at most one sound per frame.
            let set_name = self.tile_sets.get(t.set).map_or("", |n| n.as_str());
            let a = t.set_typer.step(set_name, dt);
            let b = t.scheme_typer.step(&self.schemes[t.scheme].name, dt);
            self.play_typing(a.or(b));
        }
        let was_dealing = self.dealing();
        self.deal_t += dt;
        self.deal_clicks();
        if self.state == State::Playing && !was_dealing {
            self.elapsed += dt; // the clock starts once the board is dealt
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

        let keys: &[(&[KeyCode], Action)] = if self.picker.is_some() {
            // The new-board dialog takes the keyboard while it is open.
            &[
                (&[KeyCode::Left, KeyCode::A], Action::LayoutPrev),
                (&[KeyCode::Right, KeyCode::D], Action::LayoutNext),
                (&[KeyCode::Y, KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space], Action::NewYes),
                (&[KeyCode::N, KeyCode::Escape, KeyCode::Backspace], Action::NewNo),
                (&[KeyCode::M], Action::Mute),
            ]
        } else if self.theme.is_some() {
            // The theme dialog takes the keyboard while it is open.
            &[
                (&[KeyCode::Left], Action::SetPrev),
                (&[KeyCode::Right], Action::SetNext),
                (&[KeyCode::Up], Action::SchemePrev),
                (&[KeyCode::Down], Action::SchemeNext),
                (&[KeyCode::A, KeyCode::Y, KeyCode::Enter, KeyCode::KpEnter, KeyCode::Space], Action::ThemeAccept),
                (&[KeyCode::E, KeyCode::N, KeyCode::Escape, KeyCode::Backspace], Action::ThemeExit),
                (&[KeyCode::R], Action::ThemeReload),
                (&[KeyCode::C], Action::Crt),
                (&[KeyCode::M], Action::Mute),
            ]
        } else {
            &[
                (&[KeyCode::H], Action::Hint),
                (&[KeyCode::U, KeyCode::Z, KeyCode::Backspace], Action::Undo),
                (&[KeyCode::S], Action::Shuffle),
                (&[KeyCode::N], Action::New),
                (&[KeyCode::T], Action::Theme),
                (&[KeyCode::F], Action::Dim),
                (&[KeyCode::M], Action::Mute),
                (&[KeyCode::Escape], Action::Quit),
            ]
        };
        for &(codes, action) in keys {
            if codes.iter().any(|&k| is_key_pressed(k)) && !self.run(action) {
                return false;
            }
        }

        // Pop-up dialog: its buttons run their actions; clicks elsewhere in the
        // box are swallowed so they don't reach the tiles underneath.
        self.dialog_hover = None;
        if let Some(d) = self.dialog() {
            if in_rect(mx, my, d.rect()) {
                self.hover = None;
                let buttons = d.layout_buttons();
                self.dialog_hover = buttons.iter().position(|b| in_rect(mx, my, b.hit));
                if is_mouse_button_pressed(MouseButton::Left) {
                    return match self.dialog_hover {
                        Some(k) => self.run(buttons[k].action),
                        None => true,
                    };
                }
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

        if self.modal() || self.dealing() {
            // The board is behind the new-board or theme dialog, or still being dealt.
            self.hover = None;
            return true;
        }
        if is_mouse_button_pressed(MouseButton::Left) && self.state != State::Won {
            self.click(self.hover);
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            self.deselect();
        }
        true
    }

    /// The pop-up for the current state, if any.
    fn dialog(&self) -> Option<Dialog> {
        if let Some(idx) = self.picker {
            return Some(Dialog {
                title: "-- NEW BOARD? --",
                lines: vec![],
                buttons: vec![("[Y]ES", Action::NewYes), ("[N]O", Action::NewNo)],
                picker: Some(idx),
                picker_shown: self.picker_typer.typed,
                theme: None,
            });
        }
        if let Some(t) = &self.theme {
            return Some(Dialog {
                title: "-- THEME --",
                lines: vec![],
                buttons: vec![("[E]XIT", Action::ThemeExit), ("[A]CCEPT", Action::ThemeAccept), ("[R]ELOAD", Action::ThemeReload)],
                picker: None,
                picker_shown: 0,
                theme: Some(ThemeView {
                    set_name: self.tile_sets.get(t.set).cloned().unwrap_or_default(),
                    set_shown: t.set_typer.typed,
                    set_pos: (t.set + 1, self.tile_sets.len()),
                    scheme_name: self.schemes[t.scheme].name.clone(),
                    scheme_shown: t.scheme_typer.typed,
                    scheme_pos: (t.scheme + 1, self.schemes.len()),
                    crt: t.crt,
                }),
            });
        }
        match self.state {
            State::Won => {
                let mut buttons = vec![("[N] NEW BOARD", Action::New)];
                if !cfg!(target_arch = "wasm32") {
                    buttons.push(("[ESC] QUIT", Action::Quit));
                }
                Some(Dialog {
                    title: "** ALL TILES CLEARED **",
                    lines: vec![format!("TIME {}", fmt_time(self.elapsed))],
                    buttons,
                    picker: None,
                    picker_shown: 0,
                    theme: None,
                })
            }
            State::Stuck => Some(Dialog {
                title: "-- NO MOVES LEFT --",
                lines: vec!["THE BOARD IS LOCKED.".into()],
                buttons: vec![("[S]HUFFLE", Action::Shuffle), ("[U]NDO", Action::Undo), ("[N]EW", Action::New)],
                picker: None,
                picker_shown: 0,
                theme: None,
            }),
            State::Playing => None,
        }
    }

    /// Perform an action from a hotkey or a bar button. Returns false to quit.
    /// True while the new-board or theme dialog is open.
    fn modal(&self) -> bool {
        self.picker.is_some() || self.theme.is_some()
    }

    fn run(&mut self, action: Action) -> bool {
        // While a dialog is open, the bar's game actions wait (sound and quit
        // still work).
        let game_action = matches!(action, Action::Hint | Action::Undo | Action::Shuffle | Action::New | Action::Theme | Action::Dim);
        if self.modal() && game_action {
            return true;
        }
        match action {
            Action::Hint => self.show_hint(),
            Action::Undo => self.undo(),
            Action::Shuffle => self.reshuffle(),
            Action::New => self.open_picker(),
            Action::NewYes => {
                if let Some(idx) = self.picker.take() {
                    self.set_layout(idx);
                }
            }
            Action::NewNo => {
                if self.picker.take().is_some() {
                    self.say("KEPT THE CURRENT BOARD.");
                }
            }
            Action::LayoutPrev | Action::LayoutNext => {
                if let Some(idx) = self.picker {
                    let n = board::LAYOUTS.len();
                    self.picker = Some(if action == Action::LayoutNext { (idx + 1) % n } else { (idx + n - 1) % n });
                    self.picker_typer.restart();
                }
            }
            Action::Theme => self.open_theme(),
            Action::SetPrev => self.theme_step_set(-1),
            Action::SetNext => self.theme_step_set(1),
            Action::SchemePrev => self.theme_step_scheme(-1),
            Action::SchemeNext => self.theme_step_scheme(1),
            Action::ThemeAccept => self.theme_accept(),
            Action::ThemeExit => {
                if self.theme.take().is_some() {
                    self.previews.clear();
                    self.say("THEME UNCHANGED.");
                }
            }
            Action::ThemeReload => {
                if let Some(t) = &self.theme {
                    // Re-read the set list, the palettes and the tried set's art.
                    self.previews.clear();
                    self.preview_request = Some((t.set, true));
                }
            }
            Action::Dim => {
                self.dim = !self.dim;
                self.say(if self.dim { "BLOCKED TILES DIMMED." } else { "BLOCKED TILES SHOWN NORMALLY." });
            }
            Action::Crt => {
                if let Some(t) = &mut self.theme {
                    t.crt = !t.crt; // previewed now, kept on accept
                } else {
                    self.crt = !self.crt;
                }
            }
            Action::Mute => {
                self.muted = !self.muted;
                self.say(if self.muted { "SOUND OFF." } else { "SOUND ON." });
            }
            Action::Quit => return cfg!(target_arch = "wasm32"), // no-op in the browser
        }
        true
    }

    /// Let go of the selected tile (empty-space or right click), with an
    /// escape sound if there was one. Returns whether a tile was selected.
    fn deselect(&mut self) -> bool {
        let had = self.selected.take().is_some();
        if had {
            self.play(&self.sounds.escapes);
        }
        had
    }

    fn click(&mut self, target: Option<usize>) {
        let Some(i) = target else {
            self.deselect();
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
            self.say_with_enter("ALL TILES CLEARED. WELL PLAYED.");
        } else if self.moves == 0 {
            self.state = State::Stuck;
            self.say_with_enter("NO MOVES LEFT. [S]HUFFLE, [U]NDO OR [N]EW.");
        } else {
            let s = format!("MATCH. {} TILES LEFT, {} MOVES OPEN.", left, self.moves);
            self.say_with_enter(&s);
        }
    }

    fn show_hint(&mut self) {
        if self.state != State::Playing || self.dealing() {
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
        }
        self.show_latest_now();
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
        let (dealt, dealing) = (self.dealt_count(), self.dealing());
        for &i in &self.order {
            if !self.alive[i] || self.deal_rank[i] >= dealt {
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
                let shown = |j: usize| self.alive[j] && self.deal_rank[j] < dealt;
                if !nb.right.iter().any(|&j| shown(j)) {
                    fb.shade(sx + FW + 1, sy - 2, 3, FH);
                }
                if !nb.up.iter().any(|&j| shown(j)) {
                    fb.shade(sx + 1, sy - 2, FW, 2);
                }
            }
            let face = (!dealing).then_some(self.faces[i]); // symbols hidden while dealing
            draw_tile(fb, &self.assets, sx, sy, face, sel, self.dim && !free, hover, hinted, bad);
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

        if let Some(d) = self.dialog() {
            draw_dialog(fb, &d, self.dialog_hover);
            if let Some(t) = &self.theme {
                self.draw_theme_preview(fb, &d, t.set);
            }
        }
    }

    /// The theme preview: a patch of table with one tile on it, in the tile set
    /// being tried (the colours are already the ones being tried).
    fn draw_theme_preview(&self, fb: &mut Fb, d: &Dialog, set: usize) {
        let (bx, by, bw, bh) = d.theme_box();
        fb.rect(bx, by, bw, bh, BG);
        // The table's dot lattice, lined up with the one behind the board.
        for (r, y) in (BOARD_TOP + 2..BOARD_BOT).step_by(5).enumerate() {
            if y <= by || y >= by + bh - 1 {
                continue;
            }
            let off = if r % 2 == 1 { 5 } else { 0 };
            for x in (off..W).step_by(10).filter(|&x| x > bx && x < bx + bw - 1) {
                fb.pset(x, y, DIM);
            }
        }
        fb.outline(bx, by, bw, bh, DKGREEN);
        match self.preview_assets(set) {
            Some(assets) => {
                let (tx, ty) = (bx + (bw - TILE_W) / 2, by + (bh - TILE_H) / 2);
                draw_tile(fb, assets, tx + FACE_OX, ty + FACE_OY, Some(THEME_FACE), false, false, false, false, false);
            }
            None => fb.text_center(W / 2, by + bh / 2 - 3, "...", DKGREEN, DKGREEN),
        }
    }

    fn render_hud(&self, fb: &mut Fb) {
        let x = fb.text(4, 4, "RUSTY-MAJONG", AMBER);
        let name = format!(
            "V0.3 {} / {} / {}",
            board::LAYOUTS[self.layout].name,
            self.tile_set_name(),
            self.schemes[self.scheme].name
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
            let pressed = self.bar_flash.map_or(false, |(f, _)| f == k);
            // MUTE shows in red while sound is off.
            let normal = if b.action == Action::Mute && self.muted { RED } else { DKGREEN };
            draw_button(fb, b.label, b.text_x, b.text_y, normal, self.bar_hover == Some(k), pressed);
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

fn draw_dialog(fb: &mut Fb, d: &Dialog, hover: Option<usize>) {
    let (x, y, w, h) = d.rect();
    fb.rect(x + 6, y + 4, w, h, DKGREEN); // drop shadow
    fb.rect(x, y, w, h, BG);
    fb.outline(x, y, w, h, GREEN);
    fb.outline(x + 2, y + 2, w - 4, h - 4, GREEN);
    fb.text_center(W / 2, y + 8, d.title, AMBER, AMBER);
    for (k, l) in d.lines.iter().enumerate() {
        fb.text_center(W / 2, y + 22 + k as i32 * 10, l, GREEN, AMBER);
    }
    if let Some(idx) = d.picker {
        let py = d.picker_y();
        let name = board::LAYOUTS[idx].name;
        let nx = W / 2 - (Fb::text_w(name) - 1) / 2;
        let typed: String = name.chars().take(d.picker_shown).collect();
        let end = fb.text(nx, py, &typed, AMBER);
        if d.picker_shown < name.chars().count() {
            fb.text(end, py, "█", AMBER);
        }
        let count = format!("{}/{}", idx + 1, board::LAYOUTS.len());
        fb.text(x + w - 8 - Fb::text_w(&count) + 1, y + 8, &count, DKGREEN);
        draw_layout_preview(fb, idx, W / 2, py + 14 + PREVIEW_H / 2);
    }
    if let Some(t) = &d.theme {
        let ny = d.theme_rows_y();
        let (lx, nx) = (THEME_LABEL_X, THEME_VALUE_X);
        let rows = [("TILES", &t.set_name, t.set_shown, t.set_pos), ("COLOURS", &t.scheme_name, t.scheme_shown, t.scheme_pos)];
        for (k, (label, name, shown, (i, n))) in rows.into_iter().enumerate() {
            let y = ny + k as i32 * 10;
            fb.text(lx, y, label, DKGREEN);
            let typed: String = name.chars().take(shown).collect();
            let end = fb.text(nx, y, &typed, AMBER);
            if shown < name.chars().count() {
                fb.text(end, y, "█", AMBER);
            }
            let count = format!("{i}/{n}");
            fb.text(x + w - 8 - Fb::text_w(&count) + 1, y, &count, DKGREEN);
        }
        // CRT row: the [C]RT label is a button (drawn with the others); its value here.
        fb.text(nx, ny + 20, if t.crt { "ON" } else { "OFF" }, AMBER);
    }
    for (k, b) in d.layout_buttons().iter().enumerate() {
        let normal = if b.action == Action::Crt { DKGREEN } else { GREEN };
        draw_button(fb, b.label, b.text_x, b.text_y, normal, hover == Some(k), false);
    }
}

/// Grid extent of a layout: (min gx, max gx, min gy, max gy), tiles being 2x2.
fn grid_bounds(pos: &[Pos]) -> (i32, i32, i32, i32) {
    let (mut minx, mut maxx, mut miny, mut maxy) = (i32::MAX, i32::MIN, i32::MAX, i32::MIN);
    for p in pos {
        minx = minx.min(p.gx);
        maxx = maxx.max(p.gx + 2);
        miny = miny.min(p.gy);
        maxy = maxy.max(p.gy + 2);
    }
    (minx, maxx, miny, maxy)
}

/// Pixels per half-tile in layout previews: the largest at which every layout
/// fits the preview area (tiles a little taller than wide).
fn preview_unit() -> i32 {
    let spans: Vec<(i32, i32)> = board::LAYOUTS
        .iter()
        .map(|l| {
            let (x0, x1, y0, y1) = grid_bounds(&(l.build)());
            (x1 - x0, y1 - y0)
        })
        .collect();
    (2..=6)
        .rev()
        .find(|&u| spans.iter().all(|&(w, h)| w * u <= PREVIEW_W && h * (u + u / 3) <= PREVIEW_H))
        .unwrap_or(2)
}

/// A small abstract map of a layout, centred on (cx, cy), seen straight from
/// above: one block per tile, coloured by height (higher layers brighter and
/// drawn on top), each with a background-coloured rim so tile edges read.
fn draw_layout_preview(fb: &mut Fb, idx: usize, cx: i32, cy: i32) {
    const LAYER_INK: [u8; 5] = [DKGREEN, GREEN, AMBER, IVORY, RED];
    let pos = (board::LAYOUTS[idx].build)();
    let (minx, maxx, miny, maxy) = grid_bounds(&pos);
    // One scale for every layout (the largest that fits them all), so tiles
    // and layouts keep their relative sizes when stepping through them.
    let ux = preview_unit();
    let uy = ux + ux / 3;
    let (ox, oy) = (cx - (maxx - minx) * ux / 2, cy - (maxy - miny) * uy / 2);
    let mut order: Vec<usize> = (0..pos.len()).collect();
    order.sort_by_key(|&i| pos[i].z); // lower layers first, so higher ones cover them
    for i in order {
        let p = pos[i];
        let (x, y) = (ox + (p.gx - minx) * ux, oy + (p.gy - miny) * uy);
        let c = LAYER_INK[(p.z as usize).min(LAYER_INK.len() - 1)];
        fb.rect(x - 1, y - 1, 2 * ux + 1, 2 * uy + 1, BG);
        fb.rect(x, y, 2 * ux - 1, 2 * uy - 1, c);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tile(
    fb: &mut Fb,
    assets: &Assets,
    x: i32,
    y: i32,
    face: Option<Face>,
    sel: bool,
    dim: bool,
    hover: bool,
    hint: bool,
    bad: bool,
) {
    // (x, y) is the face's top-left; tile.png is anchored so its face lands there.
    // The three-level tile sprite is multiplied by a tint per state.
    // `bad` is already blink-gated by the caller, so a blocked click flashes the whole tile red.
    // `hint` is blink-gated by the caller too, so hinted tiles flash the whole sprite.
    let tint = if bad || sel {
        TINT_SELECT
    } else if hover || hint {
        TINT_HOVER
    } else {
        TINT_NORMAL
    };
    let (tx, ty) = (x - FACE_OX, y - FACE_OY);
    fb.blit_tinted(tx, ty, &assets.tile, tint);
    if let Some(face) = face {
        fb.blit(x, y, assets.face(face));
    }
    if dim {
        // Dissolve the whole tile silhouette (sides included), not just the face.
        fb.dither_mask(tx, ty, &assets.tile, BG);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dialog_buttons_fit_inside_the_box_without_overlapping() {
        let mut g = Game::new(0, Assets::builtin(), vec!["DEFAULT".into()], 0, vec![palette::builtin()], 0);
        for state in [State::Won, State::Stuck] {
            g.state = state;
            let d = g.dialog().expect("dialog");
            let (x, y, w, h) = d.rect();
            let bs = d.layout_buttons();
            assert_eq!(bs.len(), d.buttons.len());
            for b in &bs {
                let (bx, by, bw, bh) = b.hit;
                assert!(bx >= x && by >= y && bx + bw <= x + w && by + bh <= y + h, "{} sticks out", b.label);
            }
            for pair in bs.windows(2) {
                assert!(pair[0].hit.0 + pair[0].hit.2 <= pair[1].hit.0, "{} overlaps {}", pair[0].label, pair[1].label);
            }
        }
        g.state = State::Playing;
        assert!(g.dialog().is_none());
    }

    fn test_game() -> Game {
        Game::new(0, Assets::builtin(), vec!["DEFAULT".into()], 0, vec![palette::builtin()], 0)
    }

    const DT: f32 = 1.0 / 60.0;

    #[test]
    fn queued_messages_type_fully_one_after_another_with_a_gap() {
        let mut g = test_game();
        g.say("FIRST MESSAGE");
        g.say("SECOND");
        // Run until FIRST is on the line.
        for _ in 0..600 {
            g.step_messages(DT);
            if g.msg == "FIRST MESSAGE" {
                break;
            }
        }
        assert_eq!(g.msg, "FIRST MESSAGE");
        let first = g.msg.clone();
        let len = first.chars().count();
        // When the typewriter's own timeline has FIRST fully typed (works for any
        // MSG_GAP, including 0, where SECOND starts in the same frame).
        let mut typed_out = 0.0;
        while chars_shown(&first, typed_out * TYPE_CPS) < len {
            typed_out += DT;
        }
        // Step until SECOND takes the line; FIRST's msg_t then is when it switched.
        let mut switched_at = 0.0;
        while g.msg == first {
            switched_at = g.msg_t + DT;
            g.step_messages(DT);
            assert!(switched_at < 10.0, "SECOND never started");
        }
        assert_eq!(g.msg, "SECOND");
        assert_eq!(chars_shown(&first, switched_at * TYPE_CPS), len, "SECOND started before FIRST was typed out");
        let gap = switched_at - typed_out;
        assert!(gap >= MSG_GAP - 2.0 * DT && gap <= MSG_GAP + 2.0 * DT, "gap was {gap}s, MSG_GAP is {MSG_GAP}s");
    }

    #[test]
    fn queue_is_capped() {
        let mut g = test_game();
        for i in 0..10 {
            g.say(&format!("MSG {i}"));
        }
        assert_eq!(g.queue.len(), MSG_QUEUE_MAX);
        assert_eq!(g.queue.back().map(|m| m.text.as_str()), Some("MSG 9"));
    }

    #[test]
    fn repeated_messages_type_out_again() {
        let mut g = test_game();
        g.show_latest_now();
        g.run(Action::Undo); // NOTHING TO UNDO.
        g.show_latest_now();
        assert_eq!(g.msg, "NOTHING TO UNDO.");
        g.run(Action::Undo);
        assert_eq!(g.queue.len(), 1, "the same message is queued again");
        g.step_messages(DT);
        assert_eq!(g.msg, "NOTHING TO UNDO.");
        assert!(!g.msg_finished(), "and types out from the start");
    }

    #[test]
    fn enter_sound_waits_for_the_match_message() {
        let mut g = test_game();
        let (a, b) = g.find_moves()[0];
        g.remove_pair(a, b);
        // The deal message types first; MATCH waits its turn, carrying the enter sound.
        let mut saw_match = false;
        for _ in 0..1200 {
            g.step_messages(DT);
            if g.msg.starts_with("MATCH") {
                saw_match = true;
                if !g.msg_finished() {
                    assert!(g.msg_enter, "enter sound fired before MATCH finished typing");
                }
                if !g.msg_enter {
                    break;
                }
            }
        }
        assert!(saw_match, "MATCH message never reached the line");
        assert!(!g.msg_enter, "enter sound never fired");
    }

    #[test]
    fn letter_after_a_space_takes_twice_as_long() {
        // "AB CD": A@1, B@2, ' '@3, C@5 (doubled), D@6
        let at = |steps: f32| chars_shown("AB CD", steps);
        assert_eq!([at(0.5), at(1.0), at(2.0), at(3.0), at(4.9), at(5.0), at(6.0)], [0, 1, 2, 3, 3, 4, 5]);
        assert_eq!(at(100.0), 5);
    }

    #[test]
    fn new_board_dialog_picks_a_layout_or_keeps_the_board() {
        let mut g = test_game();
        let faces = g.faces.clone();
        // NO keeps the board as it was.
        g.run(Action::New);
        assert_eq!(g.picker, Some(0));
        assert!(g.dialog().is_some_and(|d| d.picker == Some(0)));
        g.run(Action::NewNo);
        assert_eq!(g.picker, None);
        assert_eq!(g.faces, faces);
        // Arrows wrap both ways; YES deals the chosen layout.
        g.run(Action::New);
        g.run(Action::LayoutPrev);
        assert_eq!(g.picker, Some(board::LAYOUTS.len() - 1));
        g.run(Action::LayoutNext);
        g.run(Action::LayoutNext);
        g.run(Action::NewYes);
        assert_eq!((g.picker, g.layout), (None, 1));
        assert_eq!(g.remaining(), 144);
        // The arrows and buttons sit inside the dialog without overlapping.
        g.run(Action::New);
        let d = g.dialog().unwrap();
        let (x, y, w, h) = d.rect();
        let bs = d.layout_buttons();
        assert_eq!(bs.len(), 4);
        for (k, b) in bs.iter().enumerate() {
            let (bx, by, bw, bh) = b.hit;
            assert!(bx >= x && by >= y && bx + bw <= x + w && by + bh <= y + h, "{} sticks out", b.label);
            for c in &bs[k + 1..] {
                let (cx, cy, cw, ch) = c.hit;
                assert!(bx + bw <= cx || cx + cw <= bx || by + bh <= cy || cy + ch <= by, "{} overlaps {}", b.label, c.label);
            }
        }
        assert!(y >= BOARD_TOP && y + h <= BOARD_BOT, "dialog taller than the board area");
    }

    #[test]
    fn layout_name_types_out_and_restarts_on_change() {
        let mut g = test_game();
        g.run(Action::New);
        let name = board::LAYOUTS[0].name;
        assert_eq!(g.dialog().unwrap().picker_shown, 0);
        let mut sounds = 0;
        for _ in 0..120 {
            g.step_messages(DT); // keep the command line moving too
            if g.picker_typer.step(name, DT).is_some() {
                sounds += 1;
            }
        }
        assert_eq!(g.dialog().unwrap().picker_shown, name.chars().count());
        assert!(sounds > 0, "typing the name made no sound");
        g.run(Action::LayoutNext);
        assert_eq!(g.dialog().unwrap().picker_shown, 0, "a new layout name types from the start");
    }

    #[test]
    fn typing_sound_clicks_every_other_letter_and_spaces_win() {
        let mut letters = 0;
        assert_eq!(typing_sound("AB", 0, 1, &mut letters), Some(TypeSound::Click));
        assert_eq!(typing_sound("AB", 1, 2, &mut letters), None);
        assert_eq!(typing_sound("A B", 0, 3, &mut letters), Some(TypeSound::Space));
    }

    #[test]
    fn deal_places_tiles_row_by_row_then_layer_by_layer() {
        let mut g = test_game();
        assert!(g.dealing());
        // Rank order: by layer, then row (gy), then column (gx).
        let mut by_rank: Vec<usize> = (0..g.pos.len()).collect();
        by_rank.sort_by_key(|&i| g.deal_rank[i]);
        for w in by_rank.windows(2) {
            let (a, b) = (g.pos[w[0]], g.pos[w[1]]);
            assert!((a.z, a.gy, a.gx) < (b.z, b.gy, b.gx));
        }
        // Clicks and hints wait for the deal; then the clock starts.
        g.show_hint();
        assert!(g.hint.is_none());
        g.set_deal_time(99.0);
        assert!(!g.dealing());
        assert_eq!(g.dealt_count() >= g.pos.len(), true);
        g.show_hint();
        assert!(g.hint.is_some());
        // A new board deals again from the start.
        g.new_game();
        assert!(g.dealing() && g.dealt_count() == 0);
    }

    #[test]
    fn deal_clicks_only_while_no_text_is_typing() {
        let mut g = test_game();
        // The DEALT message is typing: the deal stays quiet.
        g.step_messages(DT);
        assert!(g.text_typing());
        g.deal_t = 2.0 / DEAL_TPS;
        assert!(!g.deal_clicks());
        // Once the line is fully typed, every CLICK_EVERY-th tile clicks.
        g.show_latest_now();
        assert!(!g.text_typing());
        let mut clicks = 0;
        for k in 3..=12 {
            g.deal_t = k as f32 / DEAL_TPS + 1e-4;
            if g.deal_clicks() {
                clicks += 1;
            }
        }
        assert_eq!(clicks, 10 / CLICK_EVERY);
        // The layout name typing in the new-board dialog also silences it.
        g.run(Action::New);
        g.show_latest_now();
        assert!(g.text_typing());
    }

    #[test]
    fn theme_dialog_previews_then_accepts_or_cancels() {
        let mut g = test_game();
        g.tile_sets = vec!["DEFAULT".into(), "OTHER".into()];
        g.schemes = vec![palette::builtin(), Scheme { name: "TEST".into(), rgb: [[1, 2, 3]; SLOTS] }];
        g.set_deal_time(99.0);
        // Cancel: colours preview live on the whole screen, then go back.
        g.run(Action::Theme);
        g.run(Action::SchemeNext);
        assert_eq!(g.palette()[0], [1, 2, 3]);
        g.run(Action::Crt);
        assert!(!g.scanlines(), "scanlines preview while trying");
        g.run(Action::ThemeExit);
        assert!(g.theme.is_none());
        assert_eq!((g.scheme, g.palette()[0]), (0, palette::PALETTE[0]));
        assert!(g.scanlines(), "exit keeps the scanlines as they were");
        // A set that isn't loaded yet asks the main loop for it.
        g.run(Action::Theme);
        g.run(Action::SetNext);
        assert_eq!(g.preview_request, Some((1, false)));
        g.set_preview(1, Assets::builtin(), g.tile_sets.clone(), false);
        assert!(g.preview_assets(1).is_some());
        // Wrapping both ways, then accept installs the tried set and colours.
        g.run(Action::SchemePrev);
        g.run(Action::ThemeAccept);
        assert!(g.theme.is_none());
        assert_eq!((g.tile_set, g.scheme), (1, 1));
        // Accept keeps a scanline change.
        g.run(Action::Theme);
        g.run(Action::Crt);
        g.run(Action::ThemeAccept);
        assert!(!g.scanlines());
        // Game actions wait while the dialog is open.
        g.run(Action::Theme);
        g.run(Action::New);
        assert!(g.picker.is_none());
        // The arrows and buttons sit inside the dialog without overlapping.
        let d = g.dialog().unwrap();
        let (x, y, w, h) = d.rect();
        let bs = d.layout_buttons();
        assert_eq!(bs.len(), 8);
        for (k, b) in bs.iter().enumerate() {
            let (bx, by, bw, bh) = b.hit;
            assert!(bx >= x && by >= y && bx + bw <= x + w && by + bh <= y + h, "{} sticks out", b.label);
            for c in &bs[k + 1..] {
                let (cx, cy, cw, ch) = c.hit;
                assert!(bx + bw <= cx || cx + cw <= bx || by + bh <= cy || cy + ch <= by, "{} overlaps {}", b.label, c.label);
            }
        }
    }

    #[test]
    fn clicking_empty_space_deselects() {
        let mut g = test_game();
        g.set_deal_time(99.0);
        let (a, _) = g.find_moves()[0];
        g.click(Some(a));
        assert_eq!(g.selected, Some(a));
        g.click(None);
        assert_eq!(g.selected, None);
        assert!(!g.deselect(), "nothing left to deselect: no second escape sound");
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
