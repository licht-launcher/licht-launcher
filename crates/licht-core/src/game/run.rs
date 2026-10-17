use std::path::Path;

use serde::Serialize;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::{mpsc, oneshot};
use ts_rs::TS;

/// How one game process encodes both stdout and stderr.
///
/// Java 18 and newer print UTF-8. Older Java prints the system code page.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogCodec {
    Utf8,
    Windows1252,
}

use crate::{CoreError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "lowercase")]
pub enum OutputStream {
    Stdout,
    Stderr,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
pub struct GameLine {
    pub stream: OutputStream,
    pub line: String,
}

/// One console line tagged with the instance folder that produced it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct GameLogEvent {
    pub folder: String,
    pub stream: OutputStream,
    pub line: String,
}

/// `code` is missing when the process was killed by a signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, TS)]
pub struct GameExit {
    pub code: Option<i32>,
}

impl GameExit {
    pub fn succeeded(self) -> bool {
        self.code == Some(0)
    }
}

/// Runs `command[0]` with the remaining strings as arguments. No shell.
///
/// Stdin is closed. Stdout and stderr are read together so neither pipe can
/// fill up and stall the process. A dropped `output` receiver does not stop
/// the process. Cancelling this task kills the child.
///
/// When `pid_tx` is set, the child PID is sent once after spawn so the caller
/// can write a launch lock before waiting for exit.
pub async fn run_game(
    command: &[String],
    current_dir: Option<&Path>,
    env: &[(&str, &str)],
    codec: LogCodec,
    output: mpsc::Sender<GameLine>,
    pid_tx: Option<oneshot::Sender<u32>>,
) -> Result<GameExit> {
    let Some(program) = command.first() else {
        return Err(CoreError::GameCommand);
    };
    if program.is_empty() {
        return Err(CoreError::GameCommand);
    }

    let mut process = Command::new(program);
    #[cfg(windows)]
    {
        // The GUI launcher must capture Java's output without opening a console window.
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        process.creation_flags(CREATE_NO_WINDOW);
    }
    process
        .args(&command[1..])
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true);
    for (key, value) in env {
        process.env(key, value);
    }
    if let Some(directory) = current_dir {
        process.current_dir(directory);
    }

    let mut child = process.spawn()?;
    if let Some(tx) = pid_tx
        && let Some(pid) = child.id()
    {
        let _ = tx.send(pid);
    }
    let stdout = child.stdout.take().ok_or_else(missing_pipe)?;
    let stderr = child.stderr.take().ok_or_else(missing_pipe)?;
    let (stdout_result, stderr_result) = futures_util::future::join(
        read_pipe(stdout, OutputStream::Stdout, codec, output.clone()),
        read_pipe(stderr, OutputStream::Stderr, codec, output),
    )
    .await;
    stdout_result?;
    stderr_result?;

    let status = child.wait().await?;
    Ok(GameExit {
        code: status.code(),
    })
}

/// Java 18 prints UTF-8. An older runtime prints the system code page.
pub fn log_codec(java_major: u32) -> LogCodec {
    if java_major >= 18 {
        LogCodec::Utf8
    } else {
        system_codec()
    }
}

fn system_codec() -> LogCodec {
    #[cfg(windows)]
    {
        codec_for_page(Some(system_code_page()))
    }
    #[cfg(not(windows))]
    {
        LogCodec::Utf8
    }
}

/// `None` is Linux, where the usual locale is UTF-8. `65001` is the UTF-8
/// code page. Any other page is read as lossy UTF-8 so the game keeps running.
///
/// Linux never calls this: `system_codec` returns UTF-8 there. The function
/// stays available in tests so the page mapping is checked on both systems.
#[cfg(any(windows, test))]
fn codec_for_page(code_page: Option<u32>) -> LogCodec {
    match code_page {
        Some(1252) => LogCodec::Windows1252,
        _ => LogCodec::Utf8,
    }
}

/// The ANSI code page. The standard library does not expose it.
#[cfg(windows)]
fn system_code_page() -> u32 {
    unsafe { GetACP() }
}

#[cfg(windows)]
unsafe extern "system" {
    fn GetACP() -> u32;
}

async fn read_pipe(
    pipe: impl tokio::io::AsyncRead + Unpin,
    stream: OutputStream,
    codec: LogCodec,
    output: mpsc::Sender<GameLine>,
) -> Result<()> {
    let mut reader = BufReader::new(pipe);
    let mut bytes = Vec::new();
    loop {
        bytes.clear();
        let read = reader.read_until(b'\n', &mut bytes).await?;
        if read == 0 {
            return Ok(());
        }
        if bytes.last() == Some(&b'\n') {
            bytes.pop();
            if bytes.last() == Some(&b'\r') {
                bytes.pop();
            }
        }
        let line = decode_line(&bytes, codec);
        let _ = output.send(GameLine { stream, line }).await;
    }
}

fn decode_line(bytes: &[u8], codec: LogCodec) -> String {
    match codec {
        LogCodec::Utf8 => String::from_utf8_lossy(bytes).into_owned(),
        LogCodec::Windows1252 => bytes.iter().copied().map(windows_1252).collect(),
    }
}

fn windows_1252(byte: u8) -> char {
    match byte {
        0x80..=0x9F => WINDOWS_1252_80[usize::from(byte - 0x80)],
        _ => char::from(byte),
    }
}

/// Windows-1252 bytes `80` through `9F`. Five of them are undefined.
const WINDOWS_1252_80: [char; 32] = [
    '\u{20AC}', '\u{FFFD}', '\u{201A}', '\u{0192}', '\u{201E}', '\u{2026}', '\u{2020}', '\u{2021}',
    '\u{02C6}', '\u{2030}', '\u{0160}', '\u{2039}', '\u{0152}', '\u{FFFD}', '\u{017D}', '\u{FFFD}',
    '\u{FFFD}', '\u{2018}', '\u{2019}', '\u{201C}', '\u{201D}', '\u{2022}', '\u{2013}', '\u{2014}',
    '\u{02DC}', '\u{2122}', '\u{0161}', '\u{203A}', '\u{0153}', '\u{FFFD}', '\u{017E}', '\u{0178}',
];

fn missing_pipe() -> CoreError {
    CoreError::Io(std::io::Error::other("process pipe was not captured"))
}

#[cfg(test)]
mod tests {
    use super::{LogCodec, OutputStream, codec_for_page, log_codec, read_pipe};
    use tokio::sync::mpsc;

    #[test]
    fn java_18_uses_utf8_and_the_code_page_picks_the_older_codec() {
        assert_eq!(log_codec(18), LogCodec::Utf8);
        assert_eq!(log_codec(25), LogCodec::Utf8);
        assert_eq!(codec_for_page(Some(1252)), LogCodec::Windows1252);
        assert_eq!(codec_for_page(Some(65001)), LogCodec::Utf8);
        assert_eq!(codec_for_page(Some(936)), LogCodec::Utf8);
        assert_eq!(codec_for_page(None), LogCodec::Utf8);
    }

    #[tokio::test]
    async fn a_windows_1252_line_keeps_the_word_and_the_next_line() {
        let mut bytes = b"informa\xe7\xe3o\r\n".to_vec();
        bytes.extend_from_slice(b"next\n");
        let lines = read_lines(bytes, LogCodec::Windows1252).await;
        assert_eq!(lines, ["informação".to_string(), "next".to_string()]);
    }

    #[tokio::test]
    async fn utf8_keeps_an_accent_and_replaces_a_lone_byte() {
        let lines = read_lines(b"a\xc3\xa7\xc3\xa3o\n\xff\n".to_vec(), LogCodec::Utf8).await;
        assert_eq!(lines, ["ação".to_string(), "\u{FFFD}".to_string()]);
    }

    async fn read_lines(bytes: Vec<u8>, codec: LogCodec) -> Vec<String> {
        let input = std::io::Cursor::new(bytes);
        let (output, mut incoming) = mpsc::channel(4);
        read_pipe(input, OutputStream::Stdout, codec, output)
            .await
            .expect("a bad byte should not stop the reader");
        let mut lines = Vec::new();
        while let Ok(line) = incoming.try_recv() {
            lines.push(line.line);
        }
        lines
    }
}
