use rand::Rng;
use std::time::{Duration, Instant};

/// Generate a request stream that looks like real traffic.
///
/// Uniform random keys would give every object the same popularity, which no
/// real workload does and which makes any cache look bad. Real traffic is
/// heavily skewed: a small number of objects account for most requests.
pub fn zipf_key(rng: &mut impl Rng, num_keys: usize, skew: f64) -> String {
    let u: f64 = rng.gen_range(0.0..1.0);
    // Inverse-transform sample from a power-law distribution.
    let idx = ((num_keys as f64).powf(1.0 - skew) * u).powf(1.0 / (1.0 - skew)) as usize;
    format!("/asset/{}", idx.min(num_keys - 1))
}

/// Open-loop generator: issue requests on a schedule regardless of whether
/// prior ones finished.
///
/// A closed loop (wait for a response, then send the next) silently reduces
/// its own load when the system slows down, which hides exactly the overload
/// behavior you are trying to measure.
pub async fn drive(target_rps: u64, duration: Duration, mut send: impl FnMut(String)) {
    let mut rng = rand::thread_rng();
    let interval = Duration::from_nanos(1_000_000_000 / target_rps.max(1));
    let start = Instant::now();
    let mut next = start;

    while start.elapsed() < duration {
        let key = zipf_key(&mut rng, 10_000, 0.9);
        send(key);

        next += interval;
        let now = Instant::now();
        if next > now {
            tokio::time::sleep(next - now).await;
        }
        // If we are behind schedule, do NOT sleep. Falling behind is a signal
        // worth recording, not one to smooth away.
    }
}
