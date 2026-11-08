//! Just enough HTTP for the daily challenge leaderboard: one GET or POST at a
//! time per request, polled from the game loop so the game never waits.
//!
//! Desktop and Android send with `ureq` on a background thread. The web build
//! uses the browser's `fetch`, through the small JS plugin in `web/net.js`.

/// What a finished request gives: the response body, or a short message to
/// show the player (`OFFLINE`, or the server's `ERROR ...` text).
pub type Reply = Result<String, String>;

/// The message for an error response: the server answers `ERROR <MESSAGE>`.
fn error_text(body: &str) -> String {
    match body.trim().strip_prefix("ERROR ") {
        Some(m) if !m.is_empty() => m.to_string(),
        _ => "SERVER TROUBLE".into(),
    }
}

pub struct Request {
    #[cfg(not(target_arch = "wasm32"))]
    rx: std::sync::mpsc::Receiver<Reply>,
    #[cfg(target_arch = "wasm32")]
    id: u32,
    done: bool,
}

impl Request {
    pub fn get(url: &str) -> Request {
        Request::send(false, url, "")
    }

    /// POST a JSON body.
    pub fn post(url: &str, body: &str) -> Request {
        Request::send(true, url, body)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn send(post: bool, url: &str, body: &str) -> Request {
        let (tx, rx) = std::sync::mpsc::channel();
        let (url, body) = (url.to_string(), body.to_string());
        std::thread::spawn(move || {
            let agent = ureq::AgentBuilder::new().timeout(std::time::Duration::from_secs(15)).build();
            let r = if post {
                agent.post(&url).set("Content-Type", "application/json").send_string(&body)
            } else {
                agent.get(&url).call()
            };
            let reply = match r {
                Ok(resp) => resp.into_string().map_err(|_| "OFFLINE".to_string()),
                Err(ureq::Error::Status(_, resp)) => Err(error_text(&resp.into_string().unwrap_or_default())),
                Err(_) => Err("OFFLINE".into()),
            };
            let _ = tx.send(reply);
        });
        Request { rx, done: false }
    }

    #[cfg(target_arch = "wasm32")]
    fn send(post: bool, url: &str, body: &str) -> Request {
        let id = unsafe { js::net_request(post as u32, url.as_ptr(), url.len() as u32, body.as_ptr(), body.len() as u32) };
        Request { id, done: false }
    }

    /// The reply once the request has finished (given once), else None.
    pub fn poll(&mut self) -> Option<Reply> {
        if self.done {
            return None;
        }
        let reply = self.try_reply()?;
        self.done = true;
        Some(reply)
    }

    #[cfg(not(target_arch = "wasm32"))]
    fn try_reply(&mut self) -> Option<Reply> {
        match self.rx.try_recv() {
            Ok(r) => Some(r),
            Err(std::sync::mpsc::TryRecvError::Empty) => None,
            Err(std::sync::mpsc::TryRecvError::Disconnected) => Some(Err("OFFLINE".into())),
        }
    }

    #[cfg(target_arch = "wasm32")]
    fn try_reply(&mut self) -> Option<Reply> {
        let status = unsafe { js::net_status(self.id) };
        if status == 0 {
            return None;
        }
        if status < 0 {
            unsafe { js::net_read(self.id, std::ptr::null_mut()) };
            return Some(Err("OFFLINE".into()));
        }
        let len = unsafe { js::net_len(self.id) } as usize;
        let mut buf = vec![0u8; len];
        unsafe { js::net_read(self.id, buf.as_mut_ptr()) };
        let body = String::from_utf8_lossy(&buf).into_owned();
        Some(if status >= 400 { Err(error_text(&body)) } else { Ok(body) })
    }
}

#[cfg(target_arch = "wasm32")]
mod js {
    extern "C" {
        /// Start a fetch (post: 1 for POST with a JSON body); returns its id.
        pub fn net_request(post: u32, url: *const u8, url_len: u32, body: *const u8, body_len: u32) -> u32;
        /// 0 while running, the HTTP status when done, -1 on a network error.
        pub fn net_status(id: u32) -> i32;
        /// Length of the finished response body.
        pub fn net_len(id: u32) -> u32;
        /// Copy the body to `dest` (if not null) and forget the request.
        pub fn net_read(id: u32, dest: *mut u8);
    }
}

/// Tells web/net.js which version of its API this build expects.
#[cfg(target_arch = "wasm32")]
#[no_mangle]
pub extern "C" fn rusty_net_crate_version() -> u32 {
    1
}

#[cfg(test)]
pub mod tests {
    use super::*;

    #[test]
    fn error_messages_come_from_the_server() {
        assert_eq!(error_text("ERROR BAD NAME\n"), "BAD NAME");
        assert_eq!(error_text("<html>gateway</html>"), "SERVER TROUBLE");
    }

    /// The local test server's API: DAILY_TEST_URL, else the default port.
    pub fn test_url() -> String {
        std::env::var("DAILY_TEST_URL").unwrap_or_else(|_| "http://localhost:8080/api/daily".into())
    }

    /// Against the local test server: `node web/dev-server.js`, then
    /// `cargo test -- --ignored daily` (DAILY_TEST_URL for another address).
    #[test]
    #[ignore]
    fn daily_round_trip() {
        let wait = |mut r: Request| loop {
            if let Some(reply) = r.poll() {
                break reply;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        };
        let url = &test_url();
        let date = crate::game::today_date();
        let pairs: Vec<usize> = (0..144).collect();
        let body = crate::daily::submit_body(&date, "PYRAMID", "ROUNDTRP", 99_900, &pairs);
        let b = crate::daily::parse(&wait(Request::post(url, &body)).unwrap()).unwrap();
        assert_eq!(b.you.map(|y| y.1), Some(99_900));
        let b = crate::daily::parse(&wait(Request::get(&crate::daily::board_url(url, &date, "PYRAMID", "ROUNDTRP"))).unwrap()).unwrap();
        assert!(b.top.iter().any(|e| e.name == "ROUNDTRP"));
        let bad = crate::daily::submit_body(&date, "PYRAMID", "bad name", 99_900, &pairs);
        assert_eq!(wait(Request::post(url, &bad)), Err("BAD NAME".into()));
        assert_eq!(wait(Request::get("http://localhost:1/api/daily")), Err("OFFLINE".into()));
    }
}
