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
the build again, or copy the PNGs into `dist/assets/` and use `[R]ELOAD` in the theme dialog. `Esc` does nothing on the
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

## Release builds (GitHub Actions)

Pushing a tag builds the game for Windows, Linux and Android and publishes the builds as a
GitHub release named after the tag (`.github/workflows/release.yml`):

```bash
git tag v0.4
```

```bash
git push origin v0.4
```

| File | Contents |
| --- | --- |
| `rusty-majong-<tag>-windows-x64.zip` | `rusty-majong.exe`, `assets/` and this README |
| `rusty-majong-<tag>-linux-x64.tar.gz` | the `rusty-majong` binary, `assets/` and this README (built on Ubuntu 22.04) |
| `rusty-majong-<tag>-android.apk` | the game for Android 5 and later (ARM phones and tablets, x86_64 Chromebooks and emulators), in landscape |

Some things to know:

- Each platform is its own job, so if one fails the others still go into the release. You can see
  each job's log in the repository's Actions tab, and also start a build there by hand ("Run
  workflow"); a hand-started run makes the files as downloads on the run, without a release.
- The `.psd` files are left out of the packages.
- The Linux binary needs ALSA, X11 and OpenGL, which desktop distributions have already.
- The Android build uses macroquad's packager, `cargo quad-apk`, built from source at a pinned
  commit (`QUAD_APK_REV` in the workflow), with the runner's Android SDK and NDK. Its settings
  (app name, package name, SDK versions, orientation) are under `[package.metadata.android]` in
  `Cargo.toml`. The app id is `io.github.savvazakharov.rusty_majong`. The package name must end in
  `rusty-majong`, the binary's name, or the app can't find its native library and closes on
  launch. Android asks you to allow installing it from an unknown source.
- Before the APK goes into a release, it's installed and launched on an Android 10 and an
  Android 15 emulator, and must still be running 25 seconds later
  (`.github/android-smoke-test.sh`). Each test keeps its logcat and a screenshot as downloads
  on the run. If a test fails, the release goes out without the APK.
- Each build's `versionCode` is the workflow run number and its `versionName` is the tag without
  the `v`, so a new release installs over the last one.
- The APK is signed in the workflow. Set two repository secrets so every build is signed with
  the same key and new versions install over old ones: `ANDROID_KEYSTORE` (a `.jks` keystore,
  base64-encoded) and `ANDROID_KEYSTORE_PASSWORD`. Without them, each run signs with a throwaway
  key: the APK installs, but moving to another build means uninstalling the old one first.
- On Android the art and sounds are packed inside the APK, so they can't be edited without
  rebuilding.

## Playing

Click two free tiles that match to remove them. A tile is free when nothing is on top of it and
its left or right side is open. Flowers match any flower, and seasons match any season.

There are four 144-tile layouts. Pick one when you start a new board (see below), or start on one
with `cargo run --release -- --layout castle`.

Every board has a seed, eight digits grouped like a date (`0000-00-00` to `9999-99-99`), shown
after the layout's name in the top line (`TURTLE #2026-11-08`), in the `DEALT` message and on the
win screen. A date makes a natural seed: everyone who plays `TURTLE #2026-11-08` gets the same
daily board. The same layout and
seed always deal the same board, on desktop, the web and Android alike, so a board can be played
again or shared. New boards get a random seed unless you give one in the new-board dialog (see
below), or start with one: `cargo run --release -- --layout castle --seed 2026-11-08` (the dashes
are optional). Shuffles stay
random; the seed sets the deal.

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
| `N` | new board: opens the new-board dialog, where you pick the layout and seed |
| `D` | daily challenge: opens the daily dialog with today's leaderboards |
| `T` | theme: opens the theme dialog, where you pick the tile set and colours |
| `F` | dither-dim blocked tiles |
| `M` | mute / unmute all sounds (the `[M]UTE` button turns red while muted) |
| `Esc` | quit (desktop only) |
| right click | deselect |

Every key in the table, except the extra undo keys, is also a button on the bar at the bottom of the
screen: `[H]INT`, `[T]HEME` and so on. Click or tap one to use it, which makes every option
reachable on a phone or tablet without a keyboard. Each button's hit area is the full height of
the bar and reaches halfway to its neighbours, to make it easy to tap. The web version leaves out
`QUIT`.

`[N]EW` asks first. The **NEW BOARD?** dialog shows the layout's name, with `<` and `>` either
side to step through the layouts, and a small top-down map of it: one block per tile, coloured by
height, with higher layers brighter (UI dark, then UI, amber, blue and red). The name types out like the command line, with the same
click and space sounds, each time the dialog opens or the layout changes. `[Y]ES` deals
a new board on that layout; `[N]O` closes the dialog and keeps the current board. On a keyboard,
`←` / `→` (or `A` / `D`) change the layout, `Y`, `Enter` or `Space` confirm, and `N` or `Esc`
cancel. While it's open, the board behind it ignores clicks.

Below the map is the **SEED** row. It says `RANDOM` until you type digits (up to eight, each
with a click), which fill a `____-__-__` mask, so typing `20261108` gives `2026-11-08`;
`Backspace` deletes one, and on an empty seed closes the dialog. `[T]ODAY` fills in today's date
(UTC, so it's the same daily board everywhere). `[R]EPLAY` fills in the current board's layout and
seed, so `[Y]ES` deals the same board again from the start. Pressing either again goes back to a
random seed. Both have wide hit areas for touch screens, where there's no keyboard to type a seed
with.

`[T]HEME` opens the **THEME** dialog. In the middle is one tile on a small patch of table, drawn
in the tile set and colours you're trying. Arrows sit round it: `<` / `>` (or the `←` / `→` keys)
step through the tile sets, and `^` / `⌄` (or `↑` / `↓`) through the colour schemes. The whole
screen switches to the colours you're trying, so you can see them on the board too. Under the
tile, the names of the set and the colours type out, with typing sounds, whenever they change,
with each one's place in its list on the right. Below them, the `[C]RT` row turns the CRT
scanlines on or off; like the colours, the screen shows the change straight away, and it's kept
only if you accept. An accepted theme is saved, so the game starts with it next time (see
[Saved settings](#saved-settings)).

| Button | Keys | What it does |
| --- | --- | --- |
| `[E]XIT` | `E`, `N`, `Esc`, `Backspace` | close the dialog and keep the tile set, colours and scanlines you had |
| `[A]CCEPT` | `A`, `Y`, `Enter`, `Space` | switch to the tile set, colours and scanlines shown |
| `[R]ELOAD` | `R` | re-read `sets.txt`, the palettes and the PNGs (`tile.png` and `faces.png`) of both the shown set and the board's set, and redraw the board with them straight away |
| `[C]RT` (a row under the names) | `C` | turn CRT scanlines on or off |

Each tile set is loaded the first time you step to it, which shows `...` in the box for a moment
on the web.

While either dialog is open, the board ignores clicks, and the bar's game buttons (`[H]INT`,
`[U]NDO`, `[S]HUFFLE`, `[N]EW`, `[T]HEME`, `[F] DIM`) wait until it closes. `[M]UTE` and
`[ESC] QUIT` still work.

The other pop-ups work the same way: `[N] NEW BOARD` when you clear the board (it opens the dialog above), and `[S]HUFFLE`,
`[U]NDO` and `[N]EW` when no moves are left. Click or tap a button to use it. Clicks elsewhere
inside a pop-up are ignored, so they can't hit the tiles underneath.

### Daily challenge

`[D]AILY` opens the **DAILY CHALLENGE** dialog. Every layout has its own board each day, dealt
from the date (UTC) as its seed, so everyone playing `TURTLE` on 2026-11-08 gets the same tiles.
`<` / `>` pick the layout and show its leaderboard: the top ten times, your own rank under them
if you're further down, and how many have played. `[P]LAY` deals that board.

In the challenge the top line reads `DAILY TURTLE 2026-11-08`, and there are no hints or
shuffles (`[H]INT` and `[S]HUFFLE` are dimmed). Undo works as usual: every deal can be solved,
so undoing is always a way out of a dead end. A new board from `[N]EW` leaves the challenge.

Clearing the daily shows your time to a tenth of a second and asks for a nickname, arcade style:
up to eight letters, digits, `-`, `.` or `_`. Type it, or use `↑` / `↓` (the `^` / `⌄` buttons)
to change the letter under the cursor and `←` / `→` (`<` / `>`) to move. Enter or `[ENTER] SUBMIT`
sends it, Esc or `[ESC] SKIP` doesn't. The leaderboard then shows where you placed. Each name
keeps only its best time per board, so playing again only counts if you're faster. The name
you used last is saved and filled in again next time, even after you close the game. If the time can't be sent (`OFFLINE`), `[ENTER] RETRY`
tries again.

The leaderboard lives on the website that hosts the web version: a Vercel function at
`/api/daily` in the pano-gallery project (`api/daily.js`), storing times in Upstash Redis. The
web version asks its own site; desktop and Android builds send to `daily::DEFAULT_URL` (in
`src/daily.rs`), or to `--leaderboard URL`. The server only takes today's and yesterday's
boards (for anyone finishing just after midnight UTC), rejects impossible times (under 20 s)
and badly formed names, limits each address to 30 submissions an hour, and keeps the order the
tiles were removed in alongside each best time, so suspicious runs can be checked later.

To try it locally without the real server, `node web/dev-server.js` serves the web build from
`dist/` at http://localhost:8080 and runs the same `api/daily.js` with an in-memory store
instead of Redis. Point a desktop build at it with
`cargo run -- --leaderboard http://localhost:8080/api/daily`.

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
4. winds E S W N, then dragons red, green, white, then birds 2 and 3
5. flowers 1–4, then seasons 1–4, then bird 4

The four 1 of bamboo tiles each have their own bird: bird 1 is the first cell of the bamboo row,
and birds 2–4 fill the three cells at the bottom right. All four match each other, like the
flowers. The included `faces.png` has your bird mirrored, recoloured (blue and red swapped), and
both, in those cells as placeholders to repaint. A sheet that leaves them empty shows bird 1
there instead, so older sheets still work.

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
pixel snaps to the closest of their ink colours (columns 1–4 and the background's black, in both
rows); the UI, tile body, tile and lattice colours are left out. Leave the background transparent, since faces are drawn on top of the tile.
Other colour schemes recolour the faces automatically (see below).

### Both files

- Pixels with alpha below 128 are transparent.
- Clicks are tested against the opaque pixels of `tile.png`, so the tile shape is also the hit area.
- Keep the sides within 8px left and 4px down of the face. Layers are stacked at that offset.
- A file that's missing or the wrong size falls back to the built-in art. The message line says
  which happened.

Use `[R]ELOAD` in the theme dialog (`T`, then `R`) to reload the PNGs while you edit them: the
board is redrawn with the new tiles and faces at once, whether you then accept or exit. On the web,
Reload asks the server for fresh copies, so the browser's cache can't hand back old versions. To write the built-in art out as
placeholders (existing files are kept unless you add `--force`):

```
cargo run -- --export-assets assets
```

## Command line

Messages on the command line (the `>` prompt) wait their turn in a queue. Each one types out
completely, and the next starts 1 second after it finishes, so quick clicks don't cut each other
off. If you select two tiles quickly, the first tile's name is typed in full before the second
starts. At most 4 messages wait in the queue; beyond that the oldest waiting one is dropped, so
the line never falls far behind. A repeated message types out again, so pressing the same button twice answers twice. The gap
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
- **Letting go of a selected tile**, by clicking empty space or right-clicking, plays one of
  `escape1.wav` to `escape3.wav`, picked at random. Nothing plays if no tile was selected.

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

Pick a tile set in the theme dialog (`T`). The list is:

1. **DEFAULT**: `assets/tile.png` and `assets/faces.png`.
2. One set per line of `assets/sets.txt`, in order. Each line is a folder name under
   `assets/sets/`, using only letters, digits, `-` and `_`. `#` starts a comment.
3. **BUILT-IN**: the art drawn by the code.

A set folder can hold `tile.png`, `faces.png` or both, following the rules above. Any file a set
leaves out comes from `assets/`, then from the built-in art. So a set with just a new `tile.png`
reuses your faces.

The game uses a list file instead of scanning `assets/sets/` because a browser can't list a folder
over HTTP. `[R]ELOAD` in the theme dialog re-reads `sets.txt`, so a new set shows up without restarting.

Two example sets are included. Both have only a `tile.png`:

| Set | Look |
| --- | --- |
| `wireframe` | A black body outlined like a vector drawing: white face edge, grey iso back edges |
| `chunky` | A thick 2px face border on a black face, with a bright top edge on the sides |

## Colour schemes

Pick a colour scheme (palette) in the theme dialog (`T`). A palette is an **8 × 2 pixel PNG** in
`assets/palettes/`, so you can edit it in any paint program. The top row holds 8 bright colours
and the bottom row their darkened versions:

| Column | Top row (bright) | Bottom row (dark) |
| --- | --- | --- |
| 0 | UI: the HUD's figures, command line, dialog frames and text, buttons | UI dark: the HUD's title line, dashed rules, the button bar, labels, dialog shadows |
| 1 | primary: face ink, removal effects | primary dark |
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
`[R]ELOAD` in the theme dialog reloads the palettes too, so you can see your edits right away.

| Palette | Background | Tile body | Primary | Highlight | Selection |
| --- | --- | --- | --- | --- | --- |
| PHOSPHOR (default) | black | black | green | amber | red |
| IVORY (light mode) | pale table | ivory | jade green | orange | crimson |
| AMBER | black | black | amber | pale cream | orange-red |
| CGA | black | black | cyan | yellow | magenta |
| SUNSET | black | black | orange | yellow | pink |
| ICE | black | black | sky blue | white | rose |

Some things to keep in mind:

- The interface (column 0) has its own colours, separate from the faces' green (column 1), so you
  can give the HUD, command line, bottom bar and dialogs any colour. The included palettes start
  with column 0 a copy of column 1, so they look as they always have; repaint it to change the UI.
  Highlights in the UI (key letters, names being typed, the title) still use the highlight colour
  (column 3), and a muted `[M]UTE` turns red (column 2).

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
example `cargo run -- --tiles wireframe --colours amber`). These override the saved theme for that
run.

## Saved settings

The game remembers your daily challenge nickname and your theme (tile set, colours and CRT
scanlines) between sessions. They're saved whenever they change, as `key=value` lines:

```
name=SAVVA
tiles=DEFAULT
colours=PHOSPHOR
crt=true
```

| Where | File |
| --- | --- |
| Windows | `%APPDATA%\rusty-majong\settings.txt` |
| macOS | `~/Library/Application Support/rusty-majong/settings.txt` |
| Linux | `$XDG_CONFIG_HOME/rusty-majong/settings.txt` (or `~/.config/...`) |
| Android | the app's private storage (cleared with the app's data) |
| Web | the browser's localStorage, per site (`rusty-majong-settings`) |

Delete the file to go back to the defaults. A tile set or colour scheme that no longer exists
falls back to the first one. Screenshots (`--shot`) neither read nor write the settings.

## How it's built

- Everything is drawn into a 560×315 palette-indexed framebuffer (93×39 text cells with a hand-made
  5×7 font), then scaled up with nearest-neighbour filtering to fill the window.
- The palette has 16 slots, loaded from an 8 × 2 PNG in `assets/palettes/`. They cover ink colours,
  a background, a tile body and their darkened versions. Colour schemes swap these colours.
- The projection is 2:1 pixel isometric. Each tile is extruded in 2px-by-1px steps (ivory, then
  green, then a dark base), and higher layers sit one tile-thickness up and to the right.
- Every board is dealt by playing the game backwards, so it can always be solved (see
  `board::deal`).

Debug: `--shot out.png [--layout NAME] [--seed N] [--daily-dialog | --daily-win SECONDS] [--name NAME] [--leaderboard URL] [--demo | --demo-end] [--dim] [--new-dialog | --theme-dialog] [--deal-at SECONDS]` renders one frame to a PNG
and exits; it shows the dealt board unless `--deal-at` freezes the deal animation that far in. `--sound-check` prints how many typing sounds were found and exits.
