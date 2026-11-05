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
| `Esc` | quit |
| right click | deselect |

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

| State | White becomes | Grey becomes | Black |
| --- | --- | --- | --- |
| Normal | green | dark green | black |
| Hover (free tile) | amber | dark amber | black |
| Selected | red | dark red | black |

Each dark colour is its tint × 127/255, and it's part of the palette, so the screen still only
uses palette colours. The tints are `TINT_NORMAL`, `TINT_HOVER` and `TINT_SELECT` in
`src/palette.rs`.

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

`P` cycles through colour schemes. A scheme is a palette swap: it gives new colours to the same 11
palette slots, so the HUD, the tiles, the built-in art and your PNGs all recolour together. The
background is always black.

| Scheme | Primary | Highlight (hover) | Selection |
| --- | --- | --- | --- |
| PHOSPHOR (default) | green | amber | red |
| AMBER | amber | pale cream | orange-red |
| CGA | cyan | yellow | magenta |
| SUNSET | orange | yellow | pink |
| ICE | sky blue | white | rose |

Schemes are defined in `SCHEMES` in `src/palette.rs`. To add one, add a line with its light,
light-shade, primary, red, blue, highlight and lattice colours. The game works out the dark tile
tints itself (colour × 127/255), and a test checks every scheme.

Start with a particular set and scheme with `--tiles NAME` and `--colours NAME` (desktop only, for
example `cargo run -- --tiles wireframe --colours amber`). The choice isn't saved between runs.

## How it's built

- Everything is drawn into a 560×315 palette-indexed framebuffer (93×39 text cells with a hand-made
  5×7 font), then scaled up with nearest-neighbour filtering to fill the window.
- The palette has eleven slots, set in `src/palette.rs`. In the default scheme they're black, ivory,
  phosphor green, red, blue and amber, plus a few shades. Colour schemes swap these colours.
- The projection is 2:1 pixel isometric. Each tile is extruded in 2px-by-1px steps (ivory, then
  green, then a dark base), and higher layers sit one tile-thickness up and to the right.
- Every board is dealt by playing the game backwards, so it can always be solved (see
  `board::deal`).

Debug: `--shot out.png [--layout NAME] [--demo | --demo-end] [--dim]` renders one frame to a PNG
and exits.
