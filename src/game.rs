//! Game state, input handling and rendering into the framebuffer.

use crate::art::{self, Face, FH, FW};
use crate::board::{self, Nb, Pos};
use crate::fb::Fb;
use crate::palette::*;
use macroquad::input::{is_key_pressed, is_mouse_button_pressed, KeyCode, MouseButton};
use macroquad::rand::gen_range;

pub const W: i32 = 480;
pub const H: i32 = 270;

const HX: i32 = 12; // half-tile step in pixels
const HY: i32 = 15;
const DEPTH: i32 = 4; // tile thickness in 2:1 iso steps
const LX: i32 = 2 * DEPTH; // layer offset = one tile thickness
const LY: i32 = DEPTH;
const BOARD_TOP: i32 = 16;
const BOARD_BOT: i32 = 242;

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
    dim: bool,
    elapsed: f32,
    t: f32,
    msg: String,
    msg_t: f32,
    origin: (i32, i32),
    state: State,
    moves: usize,
}

impl Game {
    pub fn new() -> Self {
        let pos = board::layout();
        let nb = board::neighbors(&pos);
        let mut order: Vec<usize> = (0..pos.len()).collect();
        // Painter's order: lower layers first, then far (right/top) to near (left/bottom).
        order.sort_by_key(|&i| (pos[i].z, pos[i].gy - pos[i].gx, pos[i].gy));

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

        let n = pos.len();
        let mut g = Game {
            pos,
            nb,
            order,
            faces: vec![Face::Dots(1); n],
            alive: vec![true; n],
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
            dim: false,
            elapsed: 0.0,
            t: 0.0,
            msg: String::new(),
            msg_t: 0.0,
            origin: (ox, oy),
            state: State::Playing,
            moves: 0,
        };
        g.new_game();
        g
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
        self.say("NEW BOARD DEALT. MATCH FREE TILES IN PAIRS.");
    }

    fn say(&mut self, s: &str) {
        self.msg = s.to_string();
        self.msg_t = 0.0;
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
        for &i in self.order.iter().rev() {
            if !self.alive[i] {
                continue;
            }
            let (sx, sy) = self.screen_pos(i);
            for t in 0..=DEPTH {
                let rx = sx - 2 * t;
                let ry = sy + t;
                if mx >= rx && mx < rx + FW && my >= ry && my < ry + FH {
                    return Some(i);
                }
            }
        }
        None
    }

    /// Returns false when the player wants to quit.
    pub fn update(&mut self, dt: f32, mouse: (f32, f32)) -> bool {
        self.t += dt;
        self.msg_t += dt;
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

        if is_key_pressed(KeyCode::Escape) {
            return false;
        }
        if is_key_pressed(KeyCode::N) {
            self.new_game();
        }
        if is_key_pressed(KeyCode::H) {
            self.show_hint();
        }
        if is_key_pressed(KeyCode::U) || is_key_pressed(KeyCode::Z) || is_key_pressed(KeyCode::Backspace) {
            self.undo();
        }
        if is_key_pressed(KeyCode::S) {
            self.reshuffle();
        }
        if is_key_pressed(KeyCode::F) {
            self.dim = !self.dim;
            self.say(if self.dim { "BLOCKED TILES DIMMED." } else { "BLOCKED TILES SHOWN NORMALLY." });
        }
        if is_key_pressed(KeyCode::C) {
            self.crt = !self.crt;
            self.say(if self.crt { "CRT SCANLINES ON." } else { "CRT SCANLINES OFF." });
        }
        if is_mouse_button_pressed(MouseButton::Left) && self.state != State::Won {
            self.click(self.hover);
        }
        if is_mouse_button_pressed(MouseButton::Right) {
            self.selected = None;
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
    }

    // ------------------------------------------------------------ render

    pub fn render(&self, fb: &mut Fb) {
        fb.clear(BLACK);

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
            draw_tile(fb, sx, sy, self.faces[i], sel, self.dim && !free, hover, hinted, bad);
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
        fb.text(x + 6, 4, "V0.1 ZIGGURAT", DKGREEN);
        let stats = format!(
            "TILES {:3} | MOVES {:2} | TIME {}",
            self.remaining(),
            self.moves,
            fmt_time(self.elapsed)
        );
        fb.text(W - 4 - Fb::text_w(&stats) + 1, 4, &stats, GREEN);
        dashed(fb, 13);

        // Prompt line with typewriter reveal and blinking block cursor.
        let shown = ((self.msg_t * 70.0) as usize).min(self.msg.chars().count());
        let line: String = self.msg.chars().take(shown).collect();
        let x = fb.text(4, 245, ">", AMBER);
        let x = fb.text_keys(x + 6, 245, &line, GREEN, AMBER);
        if ((self.t * 2.5) as i32) % 2 == 0 {
            fb.text(x, 245, "█", GREEN);
        }

        dashed(fb, 255);
        fb.text_keys(
            4,
            260,
            "[H]INT [U]NDO [S]HUFFLE [N]EW [F] DIM BLOCKED [C]RT [ESC] QUIT",
            DKGREEN,
            AMBER,
        );
    }
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
    fb.rect(x, y, w, h, BLACK);
    fb.outline(x, y, w, h, GREEN);
    fb.outline(x + 2, y + 2, w - 4, h - 4, GREEN);
    fb.text_center(W / 2, y + 8, title, AMBER, AMBER);
    for (k, l) in lines.iter().enumerate() {
        fb.text_center(W / 2, y + 22 + k as i32 * 10, l, GREEN, AMBER);
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_tile(fb: &mut Fb, x: i32, y: i32, face: Face, sel: bool, dim: bool, hover: bool, hint: bool, bad: bool) {
    // Body: stack offset copies of the face rect along the 2:1 iso depth axis.
    // Near the face it's ivory, then the green "back" of the tile, then a dark base.
    for t in (1..=DEPTH).rev() {
        let c = match t {
            1 => IVORY_SH,
            2 | 3 => GREEN,
            _ => DKGREEN,
        };
        fb.rect(x - 2 * t, y + t, FW, FH, c);
    }
    // 1px owned edge on the right/bottom so neighbouring faces never merge.
    fb.rect(x, y, FW + 1, FH + 1, IVORY_SH);
    fb.rect(x, y, FW, FH, if sel { AMBER } else { IVORY });
    art::draw_face(fb, x, y, face);
    if dim {
        fb.dither(x, y, FW, FH, BLACK);
    }
    if hover {
        fb.outline(x, y, FW, FH, GREEN);
    }
    if sel {
        fb.outline(x, y, FW, FH, RED);
    }
    if hint {
        fb.outline(x, y, FW, FH, AMBER);
        fb.outline(x + 1, y + 1, FW - 2, FH - 2, AMBER);
    }
    if bad {
        fb.outline(x, y, FW, FH, RED);
        fb.outline(x + 1, y + 1, FW - 2, FH - 2, RED);
    }
}
