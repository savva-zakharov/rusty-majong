mod art;
mod board;
mod fb;
mod font;
mod game;
mod palette;
mod sprites;

use game::{H, W};
use macroquad::prelude::*;

fn conf() -> Conf {
    Conf {
        window_title: "RUSTY-MAJONG".to_owned(),
        window_width: W * 3,
        window_height: H * 3,
        window_resizable: true,
        // In the browser, render at the device's real pixel density so phones get
        // crisp pixels instead of an upscaled blur.
        high_dpi: cfg!(target_arch = "wasm32"),
        ..Default::default()
    }
}

#[macroquad::main(conf)]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    // `--export-assets [dir] [--force]` writes the built-in art as placeholder PNGs.
    // Existing files are kept unless --force is given.
    if let Some(i) = args.iter().position(|a| a == "--export-assets") {
        let dir = args
            .get(i + 1)
            .filter(|a| !a.starts_with("--"))
            .map(std::path::PathBuf::from)
            .unwrap_or_else(|| "assets".into());
        let force = args.iter().any(|a| a == "--force");
        match sprites::export_builtin(&dir, force) {
            Ok(w) if w.is_empty() => println!("nothing written: files exist (use --force to overwrite)"),
            Ok(w) => println!("wrote {}", w.join(", ")),
            Err(e) => eprintln!("export failed: {e}"),
        }
        return;
    }
    // `--shot out.png [--demo]` renders one frame to a PNG (3x, with scanlines) and exits.
    let shot = args.iter().position(|a| a == "--shot").and_then(|i| args.get(i + 1).cloned());
    let demo = args.iter().any(|a| a == "--demo");
    let demo_end = args.iter().any(|a| a == "--demo-end");

    let seed = if shot.is_some() {
        0xC0FFEE
    } else {
        // Works on desktop and in the browser (std::time::SystemTime panics on wasm).
        (macroquad::miniquad::date::now() * 1000.0) as u64
    };
    macroquad::rand::srand(seed);

    // `--layout NAME` picks the starting layout (default: the first, Turtle).
    let layout = args
        .iter()
        .position(|a| a == "--layout")
        .and_then(|i| args.get(i + 1))
        .and_then(|n| game::Game::layout_index(n))
        .unwrap_or(0);
    // `--tiles NAME` / `--colours NAME` pick the starting tile set / colour scheme.
    let arg = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
    let root = game::assets_dir();
    let sets = sprites::list_sets(&root).await;
    let set = arg("--tiles")
        .and_then(|n| sets.iter().position(|s| s.name.eq_ignore_ascii_case(&n)))
        .unwrap_or(0);
    let scheme = arg("--colours").and_then(|n| palette::scheme_index(&n)).unwrap_or(0);
    let assets = sprites::Assets::load_set(&root, &sets[set]).await;
    let names = sets.iter().map(|s| s.name.clone()).collect();
    let mut game = game::Game::new(layout, assets, names, set, scheme);
    if demo || demo_end {
        game.demo_setup(if demo_end { 72 } else { 9 });
    }
    if args.iter().any(|a| a == "--dim") {
        game.set_dim(true);
    }
    let mut fb = fb::Fb::new(W, H);
    let mut image = Image::gen_image_color(W as u16, H as u16, BLACK);
    let tex = Texture2D::from_image(&image);
    tex.set_filter(FilterMode::Nearest);

    loop {
        let dt = get_frame_time().min(0.05);
        // Fill the window: largest scale that fits, fractional when needed (whole
        // multiples only would leave big borders on phones and odd window sizes).
        let s = (screen_width() / W as f32).min(screen_height() / H as f32);
        let ox = ((screen_width() - W as f32 * s) / 2.0).floor();
        let oy = ((screen_height() - H as f32 * s) / 2.0).floor();
        let (mx, my) = mouse_position();

        if !game.update(dt, ((mx - ox) / s, (my - oy) / s)) {
            break;
        }
        if let Some(want) = game.asset_request {
            // Re-read sets.txt too, so newly added sets show up without a restart.
            let sets = sprites::list_sets(&root).await;
            let idx = want.min(sets.len() - 1);
            let assets = sprites::Assets::load_set(&root, &sets[idx]).await;
            game.set_assets(assets, idx, sets.iter().map(|s| s.name.clone()).collect());
        }
        game.render(&mut fb);
        fb.write_rgba(&mut image.bytes, game.palette());
        tex.update(&image);

        // Letterbox bars use the scheme's background colour too.
        let bg = game.palette()[palette::BG as usize];
        clear_background(Color::from_rgba(bg[0], bg[1], bg[2], 255));
        draw_texture_ex(
            &tex,
            ox,
            oy,
            WHITE,
            DrawTextureParams { dest_size: Some(vec2(W as f32 * s, H as f32 * s)), ..Default::default() },
        );
        if game.crt && s >= 2.0 {
            let lh = (s / 3.0).max(1.0).floor();
            // Round each line to whole pixels so fractional scales don't shimmer.
            let col = Color::new(0.0, 0.0, 0.0, 0.38);
            for r in 0..H {
                draw_rectangle(ox, (oy + (r + 1) as f32 * s).round() - lh, W as f32 * s, lh, col);
            }
        }

        if let Some(path) = &shot {
            save_shot(&fb, game.palette(), path, 3);
            break;
        }
        next_frame().await
    }
}

fn save_shot(fb: &fb::Fb, pal: &[palette::Rgb; palette::SLOTS], path: &str, k: i32) {
    let (w, h) = (fb.w * k, fb.h * k);
    let mut img = Image::gen_image_color(w as u16, h as u16, BLACK);
    for y in 0..h {
        let scan = (h - 1 - y) % k == k - 1;
        for x in 0..w {
            // export_png writes rows bottom-up, so sample the framebuffer flipped.
            let rgb = pal[fb.get(x / k, (h - 1 - y) / k) as usize];
            let f = if scan { 0.62 } else { 1.0 };
            let o = ((y * w + x) * 4) as usize;
            img.bytes[o] = (rgb[0] as f32 * f) as u8;
            img.bytes[o + 1] = (rgb[1] as f32 * f) as u8;
            img.bytes[o + 2] = (rgb[2] as f32 * f) as u8;
            img.bytes[o + 3] = 255;
        }
    }
    img.export_png(path);
}
