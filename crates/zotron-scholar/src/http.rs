//! Small shared HTTP helpers.

use std::time::Duration;

/// GET a URL, retrying a few times on TRANSIENT transport errors (TLS handshake
/// failures, dropped connections, timeouts) — these flake intermittently and
/// should not abort a whole fetch. An HTTP *status* error (404, other 4xx/5xx)
/// is returned immediately: it is deterministic — e.g. a CrossRef 404 means the
/// DOI does not exist and must not be retried.
///
/// Returns the error as a `String` (rather than the large `ureq::Error`) so the
/// signature stays small.
pub fn get_with_retry(url: &str) -> Result<ureq::Response, String> {
    const MAX_ATTEMPTS: u32 = 3;
    let mut attempt = 0u32;
    loop {
        match ureq::get(url).call() {
            Ok(resp) => return Ok(resp),
            Err(ureq::Error::Status(code, _)) => return Err(format!("HTTP status {code}")),
            Err(transport) => {
                attempt += 1;
                if attempt >= MAX_ATTEMPTS {
                    return Err(format!("transport error: {transport}"));
                }
                std::thread::sleep(Duration::from_millis(300 * u64::from(attempt)));
            }
        }
    }
}
