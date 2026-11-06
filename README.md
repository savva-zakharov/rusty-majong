# rusty-majong

Mahjong solitaire in Rust: pixel-art tiles on a black, terminal-style screen.

```
cargo run --release
```

## Web version

The same code also builds for the browser (WebAssembly):

```bash
bash web/build.sh
```

```bash
npx --yes serve@14 dist --listen tcp://0.0.0.0:8080 --no-clipboard
```

This serves on every network interface, so other devices on your network can open
`http://<this-machine's-LAN-IP>:8080` (`ipconfig` shows the IP). On this machine,
http://localhost:8080 also works. Windows Firewall has to allow inbound connections for Node.js
on the network you're on. To serve to this machine only, use `--listen tcp://127.0.0.1:8080`.

`web/build.sh` puts `index.html`, `gl.js` (miniquad's loader,
copied into `web/`), `rusty-majong.wasm` and your `assets/*.png` into `dist/`. Any static file
server works. In the browser the art loads over HTTP from `dist/assets/`; after changing it, run
the build again, or copy the PNGs into `dist/assets/` and press `R`. `Esc` does nothing on the
web.

### Embedding in another site

To put the game into another website's folder:

```bash
bash web/export.sh ../pano-gallery/mahjong
```

This builds the game and copies `gl.js`, `rusty-majong.wasm` and `assets/` into that folder. The
folder's `index.html` belongs to the host site, so the script leaves it alone. If there isn't one
yet, it copies `web/index.html` as a starting point. Run it again after changing the game or its
art. The page must be served from an address ending in `/`, because the game loads its files by
relative paths.

## Playing

Click two free tiles that match to remove them. A tile is free when nothing is on top of it and
its left or right side is open. Flowers match any flower, and seasons match any season.

There are four 144-tile layouts. Press `L` to switch, or start on one with
`cargo run --release -- --layout castle`:

| Layout | Shape |
| --- | --- |
| Turtle | The traditional layout: a shell of rows, a 6×6, 4×4 and 2×2 stack, a capstone, and head and tail tiles. This is the default. |
| Ziggurat | A wide stepped pyramid with staggered rows and wings on both sides. |
| Pyramid | Four layers, each offset half a tile on the one below. Few moves are open at the start. |
| Castle | Four corner towers, walls, a central keep with a spire, and two gates. |

Every layout is tested: 144 tiles, no overlaps, no floating tiles, and deals that can always be
solved.

| Key | Action |
| --- | --- |
| `H` | hint |
| `U` / `Z` / `Backspace` | undo (also undoes shuffles) |
| `S` | shuffle the remaining tiles into a solvable arrangement |
| `N` | new board |
| `L` | next layout (deals a new board) |
| `T` | next tile set |
| `P` | next colour scheme (palette) |
| `F` | dither-dim blocked tiles |
| `R` | reload the current tile set's art and the list of sets |
| `C` | toggle CRT scanlines |
| `Esc` | quit (desktop only) |
| right click | deselect |

Every key in the table, except the extra undo keys, is also a button on the bar at the bottom of the
screen: `[H]INT`, `[L]AYOUT` and so on. Click or tap one to use it, which makes every option
reachable on a phone or tablet without a keyboard. Each button's hit area is the full height of
the bar and reaches halfway to its neighbours, to make it easy to tap. The web version leaves out
`QUIT`.

The pop-ups work the same way: `[N] NEW BOARD` when you clear the board, and `[S]HUFFLE`,
`[U]NDO` and `[N]EW` when no moves are left. Click or tap a button to use it. Clicks elsewhere
inside a pop-up are ignored, so they can't hit the tiles underneath.

## Custom tile art

Each tile is drawn from two PNGs in `assets/`: the tile body, and the picture drawn on top of it.

| File | Size | Contents |
| --- | --- | --- |
| `assets/tile.png` | 32 × 34 | A blank tile, including its sides. The face's top-left corner is at (8, 0). |
| `assets/faces.png` | 207 × 145 | A 9 × 5 grid of 23 × 29 cells, one picture per cell, with no gaps. |

Rows in `faces.png`:

1. dots 1–9
2. bamboo 1–9
3. characters 1–9
4. winds E S W N, then dragons red, green, white (the last 2 cells are unused)
5. flowers 1–4, then seasons 1–4 (the last cell is unused)

### tile.png: three tones, coloured by multiplication

Draw the tile in only three colours: white `(255,255,255)`, grey `(127,127,127)` and black
`(0,0,0)`, plus transparency. Other colours snap to the closest of the three by brightness. The
game colours the tile by multiplying it with a tint for each state:

| State | White becomes | Grey becomes | Black becomes |
| --- | --- | --- | --- |
| Normal | green | dark green | the tile body colour |
| Hover (free tile) | amber | dark amber | the tile body colour |
| Selected | red | dark red | the tile body colour |

Each dark colour is its tint × 127/255, and it's part of the palette, so the screen still only
uses palette colours. The tints are `TINT_NORMAL`, `TINT_HOVER` and `TINT_SELECT` in
`src/palette.rs`. The tile body colour depends on the colour scheme: black in the dark schemes,
ivory in IVORY. So the same three-tone tile gives black tiles on a dark screen and ivory tiles in
light mode.

### faces.png: palette colours

Draw faces in the colours of the default PHOSPHOR scheme in `src/palette.rs`. Every pixel snaps to
the closest of them. Leave the background transparent, since faces are drawn on top of the tile.
Other colour schemes recolour the faces automatically (see below).

### Both files

- Pixels with alpha below 128 are transparent.
- Clicks are tested against the opaque pixels of `tile.png`, so the tile shape is also the hit area.
- Keep the sides within 8px left and 4px down of the face. Layers are stacked at that offset.
- A file that's missing or the wrong size falls back to the built-in art. The message line says
  which happened.

Press `R` in the game to reload the PNGs while you edit them. To write the built-in art out as
placeholders (existing files are kept unless you add `--force`):

```
cargo run -- --export-assets assets
```

## Sounds

The command line makes typing sounds as it types out a message:

- **Spaces** play one of `assets/sounds/space1.wav` to `space3.wav`, picked at random.
- **Letters** play one of `click1.wav` to `click6.wav`, but only every second letter, so it isn't
  overwhelming.
- **After a space**, the next letter takes twice as long to appear, so words come out with a
  small pause between them.
- **A successful match** plays one of `enter1.wav` to `enter3.wav`, picked at random, once the
  MATCH message has finished typing, like pressing Enter at the end of a line. If another
  message replaces it first, the sound waits for that one to finish instead.

Each name is also tried capitalised (`Click1.wav`), because web servers are case-sensitive.
There's at most one sound per frame, so fast typing doesn't stack up overlapping sounds. The
settings are at the top of `src/game.rs`:

| Setting | Default | What it does |
| --- | --- | --- |
| `TYPE_CPS` | 70 | Typing speed, in steps per second |
| `AFTER_SPACE_STEPS` | 2 | Steps the letter after a space takes (other characters take 1) |
| `CLICK_EVERY` | 2 | Click on every Nth letter |
| `CLICK_VOLUME` | 0.5 | Volume of all sounds (typing and match) |
| `ENTER_DELAY` | 0.15 | Seconds between the end of the message and the enter sound |

The sounds load in the background, so the board appears at once and typing is silent for the
moment they take to arrive. Browsers only allow sound after the first tap, click or key press on
the page. The web build loads quad-snd's `audio.js` (copied into `web/`) for sound. Any file
that's missing is skipped, and with none at all the game is simply silent.

## Tile sets

`T` cycles through tile sets. The list is:

1. **DEFAULT**: `assets/tile.png` and `assets/faces.png`.
2. One set per line of `assets/sets.txt`, in order. Each line is a folder name under
   `assets/sets/`, using only letters, digits, `-` and `_`. `#` starts a comment.
3. **BUILT-IN**: the art drawn by the code.

A set folder can hold `tile.png`, `faces.png` or both, following the rules above. Any file a set
leaves out comes from `assets/`, then from the built-in art. So a set with just a new `tile.png`
reuses your faces.

The game uses a list file instead of scanning `assets/sets/` because a browser can't list a folder
over HTTP. Pressing `R` re-reads `sets.txt`, so a new set shows up without restarting.

Two example sets are included. Both have only a `tile.png`:

| Set | Look |
| --- | --- |
| `wireframe` | A black body outlined like a vector drawing: white face edge, grey iso back edges |
| `chunky` | A thick 2px face border on a black face, with a bright top edge on the sides |

## Colour schemes

`P` cycles through colour schemes. A scheme is a palette swap: it gives new colours to the same 14
palette slots, so the HUD, the tiles, the built-in art and your PNGs all recolour together. The
background and the tile body are palette slots too, which is what makes a light mode possible.

| Scheme | Background | Tile body | Primary | Highlight (hover) | Selection |
| --- | --- | --- | --- | --- | --- |
| PHOSPHOR (default) | black | black | green | amber | red |
| IVORY (light mode) | pale table | ivory | jade green | orange | crimson |
| AMBER | black | black | amber | pale cream | orange-red |
| CGA | black | black | cyan | yellow | magenta |
| SUNSET | black | black | orange | yellow | pink |
| ICE | black | black | sky blue | white | rose |

IVORY looks like ordinary tiles: ivory faces, green backs, dark inks, on a light table. The `C`
scanlines still work in light mode, but you may prefer them off there.

Schemes are defined in `SCHEMES` in `src/palette.rs`. Each scheme sets its light, light-shade,
primary, red, blue, highlight, lattice, background and tile body colours. The game works out the
dark tile tints (colour × 127/255) and the shadowed tile body itself, and a test checks every
scheme. The `BLACK` slot is always black, because it's the black ink in face art.

Start with a particular set and scheme with `--tiles NAME` and `--colours NAME` (desktop only, for
example `cargo run -- --tiles wireframe --colours amber`). The choice isn't saved between runs.

## How it's built

- Everything is drawn into a 560×315 palette-indexed framebuffer (93×39 text cells with a hand-made
  5×7 font), then scaled up with nearest-neighbour filtering to fill the window.
- The palette has 14 slots, set in `src/palette.rs`. They cover ink colours, a background, a tile
  body and a few shades. Colour schemes swap these colours.
- The projection is 2:1 pixel isometric. Each tile is extruded in 2px-by-1px steps (ivory, then
  green, then a dark base), and higher layers sit one tile-thickness up and to the right.
- Every board is dealt by playing the game backwards, so it can always be solved (see
  `board::deal`).

Debug: `--shot out.png [--layout NAME] [--demo | --demo-end] [--dim]` renders one frame to a PNG
and exits. `--sound-check` prints how many typing sounds were found and exits.
