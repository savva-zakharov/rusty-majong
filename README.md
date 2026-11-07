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

There are four 144-tile layouts. Pick one when you start a new board (see below), or start on one
with `cargo run --release -- --layout castle`:

| Layout | Shape |
| --- | --- |
| Turtle | The traditional layout: a shell of rows, a 6×6, 4×4 and 2×2 stack, a capstone, and head and tail tiles. This is the default. |
| Ziggurat | A wide stepped pyramid with staggered rows and wings on both sides. |
| Pyramid | Four layers, each offset half a tile on the one below. Few moves are open at the start. |
| Castle | Four corner towers, walls, a central keep with a spire, and two gates. |

Each new board is dealt with a short animation: the tiles appear one at a time, left to right
and top to bottom, finishing each layer before starting the one above. Their symbols stay hidden
until every tile is down, then all appear together. The board takes no clicks or hints during the
deal, and the clock starts when it ends. The speed (`DEAL_TPS`, tiles per second) and the pause
before the symbols appear (`DEAL_REVEAL_DELAY`) are at the top of `src/game.rs`.

Every layout is tested: 144 tiles, no overlaps, no floating tiles, and deals that can always be
solved.

| Key | Action |
| --- | --- |
| `H` | hint |
| `U` / `Z` / `Backspace` | undo (also undoes shuffles) |
| `S` | shuffle the remaining tiles into a solvable arrangement |
| `N` | new board: opens the new-board dialog, where you pick the layout |
| `T` | next tile set |
| `P` | next colour scheme (palette) |
| `F` | dither-dim blocked tiles |
| `R` | reload the current tile set's art and the list of sets |
| `C` | toggle CRT scanlines |
| `M` | mute / unmute all sounds (the `[M]UTE` button turns red while muted) |
| `Esc` | quit (desktop only) |
| right click | deselect |

Every key in the table, except the extra undo keys, is also a button on the bar at the bottom of the
screen: `[H]INT`, `[L]AYOUT` and so on. Click or tap one to use it, which makes every option
reachable on a phone or tablet without a keyboard. Each button's hit area is the full height of
the bar and reaches halfway to its neighbours, to make it easy to tap. The web version leaves out
`QUIT`.

`[N]EW` asks first. The **NEW BOARD?** dialog shows the layout's name, with `<` and `>` either
side to step through the layouts, and a small top-down map of it: one block per tile, coloured by
height, with higher layers brighter (dark green, then green, amber, ivory and red). The name types out like the command line, with the same
click and space sounds, each time the dialog opens or the layout changes. `[Y]ES` deals
a new board on that layout; `[N]O` closes the dialog and keeps the current board. On a keyboard,
`←` / `→` (or `A` / `D`) change the layout, `Y`, `Enter` or `Space` confirm, and `N`, `Esc` or
`Backspace` cancel. While it's open, the board behind it ignores clicks.

The other pop-ups work the same way: `[N] NEW BOARD` when you clear the board (it opens the dialog above), and `[S]HUFFLE`,
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
| Normal | tile colour (palette column 7) | tile colour dark | the tile body colour |
| Hover (free tile) | amber | dark amber | the tile body colour |
| Selected | red | dark red | the tile body colour |

Each colour comes from the current palette (see Colour schemes below). White takes a colour from
the palette's top row, and grey takes the darkened colour under it. The tile body colour depends
on the palette: black in the dark ones, ivory in IVORY. So the same three-tone tile gives black
tiles on a dark screen and ivory tiles in light mode.

### faces.png: palette colours

Draw faces in the colours of the default PHOSPHOR palette (`assets/palettes/phosphor.png`). Every
pixel snaps to the closest of them. Leave the background transparent, since faces are drawn on top of the tile.
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

## Command line

Messages on the command line (the `>` prompt) wait their turn in a queue. Each one types out
completely, and the next starts 1 second after it finishes, so quick clicks don't cut each other
off. If you select two tiles quickly, the first tile's name is typed in full before the second
starts. At most 4 messages wait in the queue; beyond that the oldest waiting one is dropped, so
the line never falls far behind. An exact repeat of the last message is skipped. The gap
(`MSG_GAP`) and the queue length (`MSG_QUEUE_MAX`) are at the top of `src/game.rs`.

## Sounds

The command line makes typing sounds as it types out a message (so does the layout name in the
new-board dialog):

- **Spaces** play one of `assets/sounds/space1.wav` to `space3.wav`, picked at random.
- **Letters** play one of `click1.wav` to `click6.wav`, but only every second letter, so it isn't
  overwhelming.
- **After a space**, the next letter takes twice as long to appear, so words come out with a
  small pause between them.
- **Dealing** clicks as the tiles are placed, at the same interval as letters (every second
  tile), using the same click sounds. It stays quiet while the command line or the new-board
  dialog's layout name is typing, so the two never play over each other.
- **A successful match** plays one of `enter1.wav` to `enter3.wav`, picked at random, once its
  MATCH message has finished typing, like pressing Enter at the end of a line.

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

`P` cycles through colour schemes (palettes). A palette is an **8 × 2 pixel PNG** in
`assets/palettes/`, so you can edit it in any paint program. The top row holds 8 bright colours
and the bottom row their darkened versions:

| Column | Top row (bright) | Bottom row (dark) |
| --- | --- | --- |
| 0 | light: built-in faces, light text | light shade |
| 1 | primary: text | primary dark: dashed rules |
| 2 | red: selection, errors, ink | red dark |
| 3 | highlight: hover, hints, prompt | highlight dark |
| 4 | blue: ink | blue dark |
| 5 | tile body (the black parts of `tile.png`) | tile body in shadow |
| 6 | background, also black ink in face art | background lattice dots |
| 7 | tile colour: white parts of `tile.png` | tile colour dark: grey parts of `tile.png` |

The screen only ever uses these 16 colours. Switching palettes recolours everything at once: the
HUD, the tiles, the built-in art and your PNGs. Shadows from raised tiles darken a pixel by moving
it from the top row to the bottom row of its column. The background is the exception: its bottom
row is the lattice, not a shade.

`assets/palettes.txt` lists the palettes in order, one name per line (the file name without
`.png`). The first is the default. As with tile sets, a list file is used because a browser can't
list a folder. To add a palette, copy one of the PNGs, recolour it and add its name to the list.
Pressing `R` reloads the palettes too, so you can see your edits right away.

| Palette | Background | Tile body | Primary | Highlight | Selection |
| --- | --- | --- | --- | --- | --- |
| PHOSPHOR (default) | black | black | green | amber | red |
| IVORY (light mode) | pale table | ivory | jade green | orange | crimson |
| AMBER | black | black | amber | pale cream | orange-red |
| CGA | black | black | cyan | yellow | magenta |
| SUNSET | black | black | orange | yellow | pink |
| ICE | black | black | sky blue | white | rose |

Some things to keep in mind:

- Black in face art, such as the bird's eye, is drawn in the background colour (column 6), so it
  always matches the table.
- The tile colour (column 7) is separate from the primary text colour (column 1), so tiles and
  text can be different colours.
- A palette that's missing or not exactly 8 × 2 is skipped, with a message in the terminal. If
  none load, the game uses a built-in copy of PHOSPHOR.
- PHOSPHOR's colours are also built into the game, because `faces.png` is matched against them.
  Editing `phosphor.png` changes how PHOSPHOR looks, but not how your face art is read. A test
  checks that `phosphor.png` and the built-in copy match.

Start with a particular set and scheme with `--tiles NAME` and `--colours NAME` (desktop only, for
example `cargo run -- --tiles wireframe --colours amber`). The choice isn't saved between runs.

## How it's built

- Everything is drawn into a 560×315 palette-indexed framebuffer (93×39 text cells with a hand-made
  5×7 font), then scaled up with nearest-neighbour filtering to fill the window.
- The palette has 16 slots, loaded from an 8 × 2 PNG in `assets/palettes/`. They cover ink colours,
  a background, a tile body and their darkened versions. Colour schemes swap these colours.
- The projection is 2:1 pixel isometric. Each tile is extruded in 2px-by-1px steps (ivory, then
  green, then a dark base), and higher layers sit one tile-thickness up and to the right.
- Every board is dealt by playing the game backwards, so it can always be solved (see
  `board::deal`).

Debug: `--shot out.png [--layout NAME] [--demo | --demo-end] [--dim] [--new-dialog] [--deal-at SECONDS]` renders one frame to a PNG
and exits; it shows the dealt board unless `--deal-at` freezes the deal animation that far in. `--sound-check` prints how many typing sounds were found and exits.
