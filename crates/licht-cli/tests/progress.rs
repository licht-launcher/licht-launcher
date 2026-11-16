use licht_cli::progress::progress_line;

#[test]
fn an_empty_download_draws_an_empty_bar() {
    assert_eq!(
        progress_line(0, 0),
        "[\u{1b}[90m████████████████████████\u{1b}[0m] 0.0%  0.0MB/0.0MB"
    );
}

#[test]
fn a_half_finished_download_fills_half_the_bar() {
    assert_eq!(
        progress_line(5_000_000, 10_000_000),
        "[████████████\u{1b}[90m████████████\u{1b}[0m] 50.0%  5.0MB/10.0MB"
    );
}

#[test]
fn megabytes_keep_one_decimal() {
    assert_eq!(
        progress_line(50_300_000, 80_200_000),
        "[███████████████\u{1b}[90m█████████\u{1b}[0m] 62.7%  50.3MB/80.2MB"
    );
}
