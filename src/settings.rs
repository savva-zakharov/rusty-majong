//! Choices kept between sessions: the daily nickname and the theme (tile set,
//! colour scheme, CRT scanlines), saved as `key=value` lines.
//!
//! Desktop: `settings.txt` in the user's config folder (`%APPDATA%\rusty-majong`
//! on Windows, `~/Library/Application Support/rusty-majong` on macOS,
//! `$XDG_CONFIG_HOME` or `~/.config/rusty-majong` elsewhere). Android: the
//! app's private files folder. Web: the browser's localStorage (web/net.js).

/// What's saved. Tile sets and schemes go by name, so adding or reordering
/// them doesn't change a saved choice.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Settings {
    pub name: String,
    pub tiles: String,
    pub colours: String,
    pub crt: Option<bool>,
}

impl Settings {
    /// Read saved lines; unknown keys and bad lines are skipped.
    pub fn parse(text: &str) -> Settings {
        let mut s = Settings::default();
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let v = v.trim().to_string();
            match k.trim() {
                "name" => s.name = v,
                "tiles" => s.tiles = v,
                "colours" => s.colours = v,
                "crt" => s.crt = v.parse().ok(),
                _ => {}
            }
        }
        s
    }

    pub fn text(&self) -> String {
        let mut t = format!("name={}\ntiles={}\ncolours={}\n", self.name, self.tiles, self.colours);
        if let Some(crt) = self.crt {
            t += &format!("crt={crt}\n");
        }
        t
    }
}

/// The saved settings, or defaults when there are none.
pub fn load() -> Settings {
    read().map(|t| Settings::parse(&t)).unwrap_or_default()
}

/// Save the settings (failures are ignored: the game plays on without them).
pub fn save(s: &Settings) {
    write(&s.text());
}

#[cfg(not(any(target_arch = "wasm32", target_os = "android")))]
fn path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    let env = |k: &str| std::env::var_os(k).filter(|v| !v.is_empty()).map(PathBuf::from);
    let dir = if cfg!(windows) {
        env("APPDATA")?
    } else if cfg!(target_os = "macos") {
        env("HOME")?.join("Library/Application Support")
    } else {
        env("XDG_CONFIG_HOME").or_else(|| env("HOME").map(|h| h.join(".config")))?
    };
    Some(dir.join("rusty-majong").join("settings.txt"))
}

/// Android: the app's private files folder (Android turns the hyphen in
/// Cargo.toml's package name into an underscore).
#[cfg(target_os = "android")]
fn path() -> Option<std::path::PathBuf> {
    Some("/data/data/io.github.savvazakharov.rusty_majong/files/settings.txt".into())
}

#[cfg(not(target_arch = "wasm32"))]
fn read() -> Option<String> {
    std::fs::read_to_string(path()?).ok()
}

#[cfg(not(target_arch = "wasm32"))]
fn write(text: &str) {
    let Some(p) = path() else { return };
    if let Some(dir) = p.parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    if let Err(e) = std::fs::write(&p, text) {
        eprintln!("rusty-majong: couldn't save {}: {e}", p.display());
    }
}

#[cfg(target_arch = "wasm32")]
fn read() -> Option<String> {
    let len = unsafe { js::settings_len() };
    if len < 0 {
        return None;
    }
    let mut buf = vec![0u8; len as usize];
    unsafe { js::settings_read(buf.as_mut_ptr()) };
    String::from_utf8(buf).ok()
}

#[cfg(target_arch = "wasm32")]
fn write(text: &str) {
    unsafe { js::settings_write(text.as_ptr(), text.len() as u32) };
}

#[cfg(target_arch = "wasm32")]
mod js {
    extern "C" {
        /// Byte length of the saved settings, or -1 when there are none.
        pub fn settings_len() -> i32;
        /// Copy the saved settings to `dest`.
        pub fn settings_read(dest: *mut u8);
        /// Save `len` bytes of settings text.
        pub fn settings_write(text: *const u8, len: u32);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_survive_a_round_trip() {
        let s = Settings { name: "SAVVA".into(), tiles: "DEFAULT".into(), colours: "AMBER".into(), crt: Some(false) };
        assert_eq!(Settings::parse(&s.text()), s);
        let old = Settings::parse("name=AB\nfuture=1\njunk\ncrt=maybe\n");
        assert_eq!(old, Settings { name: "AB".into(), ..Default::default() });
    }
}
