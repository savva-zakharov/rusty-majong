# rusty-majong

Mahjong solitaire in Rust: pixel-art tiles on a black, terminal-style screen.

```
cargo run --release
```

Click two free tiles that match to remove them. A tile is free when nothing is on top of it and
its left or right side is open. Flowers match any flower, and seasons match any season.

| Key | Action |
| --- | --- |
| `H` | hint |
| `U` / `Z` / `Backspace` | undo (also undoes shuffles) |
| `S` | shuffle the remaining tiles into a solvable arrangement |
| `N` | new board |
| `F` | dither-dim blocked tiles |
| `C` | toggle CRT scanlines |
| `Esc` | quit |
| right click | deselect |

## How it's built

- Everything is drawn into a 480×270 palette-indexed framebuffer (80×33 text cells with a hand-made
  5×7 font), then scaled up by a whole number with nearest-neighbour filtering.
- The palette has nine colours, set in `src/palette.rs`: black, ivory, phosphor green, red, blue
  and amber, plus a few shades.
- The projection is 2:1 pixel isometric. Each tile is extruded in 2px-by-1px steps (ivory, then
  green, then a dark base), and higher layers sit one tile-thickness up and to the right.
- Every board is dealt by playing the game backwards, so it can always be solved (see
  `board::deal`).

Debug: `--shot out.png [--demo | --demo-end]` renders one frame to a PNG and exits.
