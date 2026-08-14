use crate::cache::Entry;

pub struct OriginResponse {
    pub entry: Entry,
    /// Bytes actually transferred from the origin. A 304 moves headers only,
    /// which is the entire point of revalidation.
    pub bytes_from_origin: usize,
}

/// Parse `max-age` out of a Cache-Control header.
///
/// Objects without one get a conservative default rather than being cached
/// forever. Guessing long is how a stale asset survives a deploy.
pub fn parse_max_age(cache_control: Option<&str>, default_secs: u64) -> u64 {
    let Some(cc) = cache_control else { return default_secs };
    for part in cc.split(',') {
        let part = part.trim();
        if let Some(v) = part.strip_prefix("max-age=") {
            if let Ok(secs) = v.parse::<u64>() {
                return secs;
            }
        }
        if part == "no-store" || part == "no-cache" {
            return 0;
        }
    }
    default_secs
}

/// Fetch or revalidate.
///
/// When we hold a stale entry we send its ETag in If-None-Match. If the origin
/// answers 304 the object is unchanged, and we refresh the expiry WITHOUT
/// transferring the body again.
pub async fn fetch_or_revalidate(
    client: &reqwest::Client,
    url: &str,
    stale: Option<&Entry>,
    now: u64,
) -> Result<OriginResponse, reqwest::Error> {
    let mut req = client.get(url);
    if let Some(entry) = stale {
        req = req.header("If-None-Match", entry.etag.clone());
    }

    let resp = req.send().await?;

    if resp.status().as_u16() == 304 {
        let entry = stale.expect("304 only possible when we sent an ETag");
        let max_age = parse_max_age(
            resp.headers().get("cache-control").and_then(|v| v.to_str().ok()),
            60,
        );
        return Ok(OriginResponse {
            entry: Entry {
                body: entry.body.clone(),
                etag: entry.etag.clone(),
                expires_at: now + max_age,
            },
            bytes_from_origin: 0,
        });
    }

    let etag = resp
        .headers()
        .get("etag")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("")
        .to_string();
    let max_age = parse_max_age(
        resp.headers().get("cache-control").and_then(|v| v.to_str().ok()),
        60,
    );

    let body = resp.bytes().await?.to_vec();
    let len = body.len();

    Ok(OriginResponse {
        entry: Entry { body, etag, expires_at: now + max_age },
        bytes_from_origin: len,
    })
}
