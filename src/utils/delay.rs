use rand::Rng;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

use crate::constants::MICROSECONDS_PER_SECOND;

/// Apply random delay between requests if configured
pub async fn apply_random_delay(random_delay_str: &Option<String>) {
    if let Some(delay_str) = random_delay_str {
        let parts: Vec<&str> = delay_str.split(':').collect();
        if parts.len() == 2 {
            if let (Ok(min_delay), Ok(max_delay)) =
                (parts[0].parse::<u64>(), parts[1].parse::<u64>())
            {
                if max_delay >= min_delay {
                    let delay = rand::thread_rng().gen_range(min_delay..=max_delay);
                    tokio::time::sleep(Duration::from_millis(delay)).await;
                } else {
                    eprintln!(
                        "[Warning] Invalid --random-delay format: MAX must be greater than or equal to MIN. Got: {}",
                        delay_str
                    );
                }
            } else {
                eprintln!(
                    "[Warning] Invalid --random-delay format: Could not parse min/max values. Got: {}",
                    delay_str
                );
            }
        } else {
            eprintln!(
                "[Warning] Invalid --random-delay format. Expected MIN:MAX. Got: {}",
                delay_str
            );
        }
    }
}

/// Apply rate limiting to control request frequency
pub async fn apply_rate_limit(rate_limit: Option<u64>, last_request_time: &Arc<Mutex<Instant>>) {
    if let Some(rate_limit) = rate_limit {
        let mut last_req_guard = last_request_time.lock().await;
        let elapsed = last_req_guard.elapsed();
        let min_delay_micros = MICROSECONDS_PER_SECOND / rate_limit;
        if elapsed.as_micros() < min_delay_micros as u128 {
            let sleep_duration =
                Duration::from_micros(min_delay_micros - elapsed.as_micros() as u64);
            tokio::time::sleep(sleep_duration).await;
        }
        *last_req_guard = Instant::now();
    }
}
