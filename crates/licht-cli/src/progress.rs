use std::io::{IsTerminal, Write};
use std::time::{Duration, Instant};

use licht_core::DownloadProgress;
use tokio::sync::mpsc;

const PROGRESS_WIDTH: usize = 24;
/// Eight steps per cell, so a download between two cells still moves the bar.
const PROGRESS_STEPS: u64 = (PROGRESS_WIDTH * 8) as u64;
const PARTIAL_BLOCK: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];
/// Same glyph as the filled cells, drawn gray so the track is not a taller shade.
const PENDING: &str = "\u{1b}[90m";
const RESET: &str = "\u{1b}[0m";

pub async fn drain_download_progress(incoming: &mut mpsc::Receiver<DownloadProgress>) {
    let interactive = std::io::stdout().is_terminal();
    let mut previous_total = None;
    let mut last_draw = None;
    while let Some(progress) = incoming.recv().await {
        let complete = progress.total > 0 && progress.finished + progress.failed == progress.total;
        let phase_changed = previous_total.is_some_and(|total| total != progress.total);
        let due =
            last_draw.is_none_or(|drawn: Instant| drawn.elapsed() >= Duration::from_millis(100));
        let line = progress_line(progress.bytes_done, progress.bytes_total);
        if interactive {
            if !complete && !phase_changed && !due {
                continue;
            }
            if phase_changed {
                println!();
            }
            print!("\r{line}");
            let _ = std::io::stdout().flush();
            if complete {
                println!();
            }
        } else if complete || phase_changed || progress.finished % 100 == 0 {
            println!("{line}");
        } else {
            continue;
        }
        previous_total = Some(progress.total);
        last_draw = Some(Instant::now());
    }
}

pub fn progress_line(done: u64, total: u64) -> String {
    let done = done.min(total);
    let steps = if total == 0 {
        0
    } else {
        (u128::from(done) * u128::from(PROGRESS_STEPS) / u128::from(total)) as usize
    };
    let steps = steps.min(PROGRESS_STEPS as usize);
    let full = steps / 8;
    let partial = steps % 8;
    let mut filled = "█".repeat(full);
    if partial > 0 {
        filled.push(PARTIAL_BLOCK[partial]);
    }
    let rest = PROGRESS_WIDTH - full - usize::from(partial > 0);
    let bar = if rest == 0 {
        format!("[{filled}]")
    } else {
        format!("[{filled}{PENDING}{}{RESET}]", "█".repeat(rest))
    };
    let percent = if total == 0 {
        0
    } else {
        u128::from(done) * 1000 / u128::from(total)
    };
    format!(
        "{bar} {percent}.{percent_frac}%  {done_mb}.{done_frac}MB/{total_mb}.{total_frac}MB",
        percent = percent / 10,
        percent_frac = percent % 10,
        done_mb = megabytes(done).0,
        done_frac = megabytes(done).1,
        total_mb = megabytes(total).0,
        total_frac = megabytes(total).1,
    )
}

fn megabytes(bytes: u64) -> (u64, u64) {
    let tenths = bytes.saturating_mul(10) / 1_000_000;
    (tenths / 10, tenths % 10)
}
