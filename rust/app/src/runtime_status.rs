//! server.getRuntimeStatus — tokio scheduler-lag analog of the upstream
//! eventLoopMonitor. The upstream reads a Node `monitorEventLoopDelay`
//! histogram plus `eventLoopUtilization`; the honest Rust equivalent samples
//! timer drift (elapsed - requested sleep) once per second and reports the
//! drift percentiles as event delay. `utilization` is the observed scheduler
//! backlog fraction `drift / elapsed`, clamped to [0,1] — a lag proxy, not
//! Node's ELU.
//!
//! A stall window is a sample whose drift exceeds 2s (same as upstream's
//! STALL_MS); gaps we can't measure (e.g. sleep while the process was
//! suspended) count as discarded idle gaps instead of stalls.

use serde_json::{Value, json};
use std::collections::VecDeque;
use std::sync::{Mutex, OnceLock};
use std::time::Instant;

const SAMPLE_MS: u64 = 1000;
const STALL_MS: u64 = 2000;
/// A gap longer than this is treated as a suspend/idle gap, not a stall:
/// the monitor can't distinguish OS suspend from scheduler lag past 10s.
const SUSPEND_GAP_MS: u128 = 10_000;
const KEEP_SAMPLES: usize = 4096;

struct Stats {
    started: Instant,
    samples: VecDeque<u64>,
    sample_count: u64,
    stall_window_count: u64,
    max_stall_ms: u64,
    discarded_idle_gap_count: u64,
    discarded_idle_gap_ms: u64,
    last_stall_at: Option<Instant>,
    last_stall_ms: u64,
}

static STATS: OnceLock<Mutex<Stats>> = OnceLock::new();

fn stats() -> &'static Mutex<Stats> {
    STATS.get_or_init(|| {
        Mutex::new(Stats {
            started: Instant::now(),
            samples: VecDeque::with_capacity(KEEP_SAMPLES),
            sample_count: 0,
            stall_window_count: 0,
            max_stall_ms: 0,
            discarded_idle_gap_count: 0,
            discarded_idle_gap_ms: 0,
            last_stall_at: None,
            last_stall_ms: 0,
        })
    })
}

/// Spawned once from main. Wakes every SAMPLE_MS and records the drift.
pub fn start_sampler() {
    tokio::spawn(async move {
        let mut expected = Instant::now();
        loop {
            expected += std::time::Duration::from_millis(SAMPLE_MS);
            tokio::time::sleep_until(tokio::time::Instant::from_std(expected)).await;
            let now = Instant::now();
            let drift = now.saturating_duration_since(expected).as_millis();
            let mut s = match stats().lock() {
                Ok(s) => s,
                Err(e) => e.into_inner(),
            };
            s.sample_count += 1;
            if drift >= SUSPEND_GAP_MS {
                s.discarded_idle_gap_count += 1;
                s.discarded_idle_gap_ms += drift as u64;
                continue;
            }
            let drift = drift as u64;
            s.samples.push_back(drift);
            if s.samples.len() > KEEP_SAMPLES {
                s.samples.pop_front();
            }
            if drift > STALL_MS {
                s.stall_window_count += 1;
                if drift > s.max_stall_ms {
                    s.max_stall_ms = drift;
                }
                s.last_stall_at = Some(now);
                s.last_stall_ms = drift;
            }
        }
    });
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((sorted.len() - 1) * p / 100).min(sorted.len() - 1);
    sorted[idx]
}

pub fn get_runtime_status() -> Value {
    let s = match stats().lock() {
        Ok(s) => s,
        Err(e) => e.into_inner(),
    };
    let mut sorted: Vec<u64> = s.samples.iter().copied().collect();
    sorted.sort_unstable();
    let last_stall = s.last_stall_at.map(|t| {
        json!({
            "durationMs": s.last_stall_ms,
            "ageMs": t.elapsed().as_millis() as u64,
        })
    });
    let utilization = if sorted.is_empty() {
        0.0
    } else {
        (sorted[sorted.len() / 2] as f64 / SAMPLE_MS as f64).clamp(0.0, 1.0)
    };
    json!({
        "available": true,
        "sampleWindowMs": s.started.elapsed().as_millis() as u64,
        "sampleCount": s.sample_count,
        "delayP50Ms": percentile(&sorted, 50),
        "delayP99Ms": percentile(&sorted, 99),
        "delayMaxMs": sorted.last().copied().unwrap_or(0),
        "utilization": utilization,
        "stallWindowCount": s.stall_window_count,
        "maxStallMs": s.max_stall_ms,
        "discardedIdleGapCount": s.discarded_idle_gap_count,
        "discardedIdleGapMs": s.discarded_idle_gap_ms,
        "lastStall": last_stall,
    })
}
