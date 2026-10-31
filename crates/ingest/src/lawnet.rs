//! HTTP for the law check, with the traffic rules of docs/design/law-change-detection.md:
//! one named user agent, one request a second per host, one retry, `Retry-After` honoured up to
//! 60 seconds, no redirects followed here (the check decides), and errors that never carry a
//! URL, because the NY Senate key travels in one.

use std::collections::HashMap;
use std::time::{Duration, Instant};

pub const USER_AGENT: &str =
    "HouseCheck-law-check/0.2 (+https://github.com/nessaisling-lab/housecheck)";

/// One response, headers lowercased.
#[derive(Debug, Clone, Default)]
pub struct Resp {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: Vec<u8>,
}

impl Resp {
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(name).map(String::as_str)
    }

    /// A bot challenge instead of the page. Never parsed and never retried with other headers:
    /// a protected page is a person's job (docs/design/law-change-detection.md, "Failure
    /// modes").
    pub fn blocked(&self) -> bool {
        let cloudflare = self
            .header("server")
            .is_some_and(|s| s.contains("cloudflare"));
        let head = String::from_utf8_lossy(&self.body[..self.body.len().min(4096)]);
        self.header("cf-mitigated").is_some()
            || (cloudflare && matches!(self.status, 403 | 503))
            || head.contains("<title>Just a moment...</title>")
    }
}

/// How the check reaches the network, so every route can run offline in a test.
pub trait Fetch {
    /// One request; `head` for a HEAD. An `Err` is a message with no URL in it.
    fn get(&mut self, url: &str, headers: &[(&str, String)], head: bool) -> Result<Resp, String>;
    /// Bytes received so far.
    fn bytes(&self) -> u64;
    /// Requests sent so far, retries included.
    fn requests(&self) -> u32;
}

pub struct Net {
    client: reqwest::blocking::Client,
    last: HashMap<String, Instant>,
    bytes: u64,
    requests: u32,
}

impl Net {
    pub fn new() -> anyhow::Result<Net> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_secs(60))
            .build()?;
        Ok(Net {
            client,
            last: HashMap::new(),
            bytes: 0,
            requests: 0,
        })
    }

    fn once(&mut self, url: &str, headers: &[(&str, String)], head: bool) -> Result<Resp, String> {
        let host = reqwest::Url::parse(url)
            .map_err(|_| "bad URL".to_string())?
            .host_str()
            .unwrap_or_default()
            .to_string();
        if let Some(t) = self.last.get(&host) {
            let wait = Duration::from_secs(1).saturating_sub(t.elapsed());
            std::thread::sleep(wait);
        }
        self.last.insert(host, Instant::now());
        self.requests += 1;
        let mut req = if head {
            self.client.head(url)
        } else {
            self.client.get(url)
        };
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        let r = req.send().map_err(|e| e.without_url().to_string())?;
        let status = r.status().as_u16();
        let headers = r
            .headers()
            .iter()
            .map(|(k, v)| {
                (
                    k.as_str().to_ascii_lowercase(),
                    v.to_str().unwrap_or_default().to_string(),
                )
            })
            .collect();
        let body = r.bytes().map_err(|e| e.without_url().to_string())?.to_vec();
        self.bytes += body.len() as u64;
        Ok(Resp {
            status,
            headers,
            body,
        })
    }
}

impl Fetch for Net {
    fn get(&mut self, url: &str, headers: &[(&str, String)], head: bool) -> Result<Resp, String> {
        let first = self.once(url, headers, head);
        let wait = match &first {
            Err(_) => 2,
            Ok(r) if r.status == 429 || r.status >= 500 => r
                .header("retry-after")
                .and_then(|s| s.trim().parse::<u64>().ok())
                .unwrap_or(2)
                .min(60),
            Ok(_) => return first,
        };
        // A challenge page is final: retrying it is the bypassing the design rules out.
        if first.as_ref().is_ok_and(Resp::blocked) {
            return first;
        }
        std::thread::sleep(Duration::from_secs(wait));
        self.once(url, headers, head)
    }

    fn bytes(&self) -> u64 {
        self.bytes
    }

    fn requests(&self) -> u32 {
        self.requests
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The NY Senate key rides in the query string. A failed request must not print it.
    #[test]
    fn the_key_never_leaks_into_an_error() {
        let mut net = Net::new().unwrap();
        // Port 9 (discard) on the loopback address: refused at once, no traffic leaves.
        let err = net
            .get(
                "http://127.0.0.1:9/api/3/laws/RPP/235-B?key=SECRET-TEST",
                &[],
                false,
            )
            .expect_err("nothing listens there");
        assert!(!err.contains("SECRET-TEST"), "{err}");
        assert!(!err.contains("127.0.0.1"), "{err}");
    }

    #[test]
    fn challenge_pages_are_blocked() {
        let mut r = Resp {
            status: 403,
            ..Default::default()
        };
        r.headers.insert("server".into(), "cloudflare".into());
        assert!(r.blocked());
        r.status = 200;
        assert!(
            !r.blocked(),
            "a normal page behind Cloudflare (hcr.ny.gov) is not blocked"
        );
        // Synthetic: the marker a Cloudflare challenge page carries.
        r.body = b"<html><head><title>Just a moment...</title>".to_vec();
        assert!(r.blocked());
        let mut m = Resp::default();
        m.headers.insert("cf-mitigated".into(), "challenge".into());
        assert!(m.blocked());
    }
}
