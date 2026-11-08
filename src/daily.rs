//! Daily challenge leaderboard: talking to the server (`/api/daily` on the
//! website, see pano-gallery's api/daily.js) and reading its answers.
//!
//! Every layout has its own board each day, dealt from the date (UTC) as its
//! seed. The server keeps each nickname's best time per board.

/// Where desktop and Android builds send times (`--leaderboard URL` changes
/// it). The web build asks its own site, so it works wherever it's hosted.
pub const DEFAULT_URL: &str = if cfg!(target_arch = "wasm32") { "/api/daily" } else { "https://REPLACE-WITH-SITE/api/daily" };

/// Nickname characters, in the order the arcade-style ^ / ⌄ buttons cycle.
pub const NAME_CHARS: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-._";
pub const NAME_MAX: usize = 8;

/// One leaderboard line.
#[derive(Clone, Debug, PartialEq)]
pub struct Entry {
    pub rank: u32,
    pub ms: u32,
    pub name: String,
}

/// A board's leaderboard, as the server sent it.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Board {
    /// Entries on the board.
    pub count: u32,
    /// The asking name's rank and best time, if it has one.
    pub you: Option<(u32, u32)>,
    /// After a submission: NEW, BETTER or KEPT.
    pub result: Option<String>,
    /// The top ten.
    pub top: Vec<Entry>,
}

/// Read the server's answer (see api/daily.js for the format).
pub fn parse(text: &str) -> Result<Board, String> {
    let mut b = Board::default();
    let mut seen_header = false;
    for line in text.lines() {
        let w: Vec<&str> = line.split_whitespace().collect();
        match w.as_slice() {
            ["DAILY", ..] => seen_header = true,
            ["COUNT", n] => b.count = n.parse().unwrap_or(0),
            ["YOU", r, ms] => b.you = r.parse().ok().zip(ms.parse().ok()),
            ["RESULT", r] => b.result = Some(r.to_string()),
            [r, ms, name] => {
                if let (Ok(rank), Ok(ms)) = (r.parse(), ms.parse()) {
                    b.top.push(Entry { rank, ms, name: name.to_string() });
                }
            }
            _ => {}
        }
    }
    if seen_header {
        Ok(b)
    } else {
        Err("SERVER TROUBLE".into())
    }
}

/// The GET address for a board's leaderboard (with `name`'s rank, if any).
pub fn board_url(base: &str, date: &str, layout: &str, name: &str) -> String {
    let mut u = format!("{base}?date={date}&layout={layout}");
    if !name.is_empty() {
        u += &format!("&name={name}");
    }
    u
}

/// The JSON body that submits a time. `pairs` lists the removed tiles in
/// order (pairs side by side), so the server can check a run later.
pub fn submit_body(date: &str, layout: &str, name: &str, ms: u32, pairs: &[usize]) -> String {
    let pairs: Vec<String> = pairs.iter().map(|i| i.to_string()).collect();
    format!(r#"{{"date":"{date}","layout":"{layout}","name":"{name}","ms":{ms},"pairs":[{}]}}"#, pairs.join(","))
}

/// A time as MM:SS.T (tenths), e.g. `03:21.4`.
pub fn fmt_ms(ms: u32) -> String {
    let tenths = ms / 100;
    format!("{:02}:{:02}.{}", tenths / 600, tenths / 10 % 60, tenths % 10)
}

/// The nickname character after (`step` 1) or before (-1) `c`, wrapping.
pub fn cycle_char(c: Option<char>, step: i32) -> char {
    let chars: Vec<char> = NAME_CHARS.chars().collect();
    let n = chars.len() as i32;
    let i = match c.and_then(|c| chars.iter().position(|&x| x == c)) {
        Some(i) => (i as i32 + step).rem_euclid(n),
        None => if step > 0 { 0 } else { n - 1 },
    };
    chars[i as usize]
}

/// A typed character as a nickname character (letters made upper case).
pub fn name_char(c: char) -> Option<char> {
    let c = c.to_ascii_uppercase();
    NAME_CHARS.contains(c).then_some(c)
}

/// Nickname being typed, arcade style: up to NAME_MAX characters and a cursor
/// that moves over them (and onto the next empty slot).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NameEdit {
    pub chars: Vec<char>,
    pub cursor: usize,
}

impl NameEdit {
    pub fn new(name: &str) -> NameEdit {
        let chars: Vec<char> = name.chars().filter_map(name_char).take(NAME_MAX).collect();
        let cursor = chars.len().min(NAME_MAX - 1);
        NameEdit { chars, cursor }
    }

    pub fn text(&self) -> String {
        self.chars.iter().collect()
    }

    /// Put a typed character at the cursor and move on.
    pub fn type_char(&mut self, c: char) {
        if self.cursor < self.chars.len() {
            self.chars[self.cursor] = c;
        } else if self.chars.len() < NAME_MAX {
            self.chars.push(c);
        }
        self.cursor = (self.cursor + 1).min(self.chars.len()).min(NAME_MAX - 1);
    }

    /// Delete the character before the cursor, or the one under it when the
    /// cursor is on the last character (as after ^ / ⌄ or filling all eight).
    pub fn backspace(&mut self) {
        if self.cursor + 1 == self.chars.len() {
            self.chars.pop();
        } else if self.cursor > 0 {
            self.cursor -= 1;
            self.chars.remove(self.cursor);
        }
    }

    /// ^ / ⌄: change the character under the cursor (an empty slot starts at
    /// A going up, or the last character going down).
    pub fn cycle(&mut self, step: i32) {
        let c = cycle_char(self.chars.get(self.cursor).copied(), step);
        if self.cursor < self.chars.len() {
            self.chars[self.cursor] = c;
        } else {
            self.chars.push(c);
        }
    }

    /// < / >: move the cursor, as far as the first empty slot.
    pub fn move_cursor(&mut self, step: i32) {
        let max = self.chars.len().min(NAME_MAX - 1) as i32;
        self.cursor = (self.cursor as i32 + step).clamp(0, max) as usize;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_servers_answer() {
        let b = parse("DAILY 2026-11-08 TURTLE\nCOUNT 2\nYOU 2 190000\nRESULT BETTER\n1 185300 ALICE\n2 190000 SAVVA\n").unwrap();
        assert_eq!(b.count, 2);
        assert_eq!(b.you, Some((2, 190000)));
        assert_eq!(b.result.as_deref(), Some("BETTER"));
        assert_eq!(b.top[1], Entry { rank: 2, ms: 190000, name: "SAVVA".into() });
        let b = parse("DAILY 2026-11-08 TURTLE\nCOUNT 0\nYOU - -\n").unwrap();
        assert_eq!((b.you, b.top.len()), (None, 0));
        assert!(parse("<html>").is_err());
    }

    #[test]
    fn builds_requests() {
        assert_eq!(board_url("/api/daily", "2026-11-08", "TURTLE", ""), "/api/daily?date=2026-11-08&layout=TURTLE");
        assert_eq!(board_url("/x", "2026-11-08", "CASTLE", "AB"), "/x?date=2026-11-08&layout=CASTLE&name=AB");
        assert_eq!(
            submit_body("2026-11-08", "TURTLE", "AB", 61234, &[3, 7]),
            r#"{"date":"2026-11-08","layout":"TURTLE","name":"AB","ms":61234,"pairs":[3,7]}"#
        );
        assert_eq!(fmt_ms(201_449), "03:21.4");
        assert_eq!(fmt_ms(3_599_999), "59:59.9");
    }

    #[test]
    fn names_type_and_cycle_like_an_arcade() {
        let mut n = NameEdit::new("ab c");
        assert_eq!((n.text().as_str(), n.cursor), ("ABC", 3));
        n.backspace();
        assert_eq!((n.text().as_str(), n.cursor), ("AB", 2));
        n.cycle(1); // empty slot -> A
        n.cycle(-1); // A -> _ (wraps to the end)
        assert_eq!(n.text(), "AB_");
        n.move_cursor(-1);
        n.type_char('Z');
        assert_eq!((n.text().as_str(), n.cursor), ("AZ_", 2));
        for c in "12345678".chars() {
            n.type_char(c);
        }
        assert_eq!((n.text().as_str(), n.cursor), ("AZ123458", 7)); // 7 and 8 both land on the last slot
        n.type_char('X'); // full: the last character changes
        assert_eq!(n.text(), "AZ12345X");
        n.backspace();
        assert_eq!((n.text().as_str(), n.cursor), ("AZ12345", 7));
        n.move_cursor(5);
        assert_eq!(n.cursor, 7);
        assert_eq!(name_char('q'), Some('Q'));
        assert_eq!(name_char(' '), None);
    }
}
