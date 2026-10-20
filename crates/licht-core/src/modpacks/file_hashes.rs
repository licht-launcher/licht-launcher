use std::fs;
use std::io::Read;
use std::path::Path;

use crate::hashes::hex_encode;
use crate::{CoreError, DownloadControl, Result};

/// Returns `(sha1, sha512)` as lowercase hex, or `None` when the file does not exist.
pub(super) fn hash_file(path: &Path) -> Result<Option<(String, String)>> {
    use sha1::{Digest, Sha1};
    use sha2::Sha512;

    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        sha1.update(&buffer[..read]);
        sha512.update(&buffer[..read]);
    }
    Ok(Some((
        hex_encode(sha1.finalize().as_slice()),
        hex_encode(sha512.finalize().as_slice()),
    )))
}

/// `true` when the file must be fetched. A sha1 hit with a wrong sha512 is deleted
/// so the downloader does not skip it on the sha1 alone.
pub(super) async fn prepare_download(
    path: &Path,
    sha1: &str,
    sha512: &str,
    control: &DownloadControl,
) -> Result<bool> {
    control.ready().await?;
    if sha1.is_empty() || sha512.is_empty() {
        return Ok(true);
    }
    let Some((actual_sha1, actual_sha512)) = hash_file_with_control(path, control).await? else {
        return Ok(true);
    };
    if !actual_sha1.eq_ignore_ascii_case(sha1) {
        return Ok(true);
    }
    if actual_sha512.eq_ignore_ascii_case(sha512) {
        return Ok(false);
    }
    tokio::fs::remove_file(path).await?;
    Ok(true)
}

pub(super) async fn ensure_sha512(
    path: &Path,
    expected: &str,
    control: &DownloadControl,
) -> Result<()> {
    let actual = hash_file_with_control(path, control)
        .await?
        .map(|(_, sha512)| sha512)
        .unwrap_or_default();
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    match tokio::fs::remove_file(path).await {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        Err(error) => return Err(error.into()),
    }
    Err(CoreError::Sha512Mismatch {
        expected: expected.to_string(),
        actual,
    })
}

async fn hash_file_with_control(
    path: &Path,
    control: &DownloadControl,
) -> Result<Option<(String, String)>> {
    control.ready().await?;
    let mut file = match tokio::fs::File::open(path).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    Ok(Some(hash_reader(&mut file, control).await?))
}

async fn hash_reader(
    reader: &mut (impl tokio::io::AsyncRead + Unpin),
    control: &DownloadControl,
) -> Result<(String, String)> {
    use sha1::{Digest, Sha1};
    use sha2::Sha512;
    use tokio::io::AsyncReadExt;

    let mut sha1 = Sha1::new();
    let mut sha512 = Sha512::new();
    let mut buffer = vec![0_u8; 64 * 1024];
    loop {
        control.ready().await?;
        // Finish each bounded read before cancellation so its file handle can close safely.
        let read = reader.read(&mut buffer).await?;
        if read == 0 {
            break;
        }
        sha1.update(&buffer[..read]);
        sha512.update(&buffer[..read]);
        tokio::task::yield_now().await;
    }
    control.ready().await?;
    Ok((
        hex_encode(sha1.finalize().as_slice()),
        hex_encode(sha512.finalize().as_slice()),
    ))
}

/// Returns the MD5 as lowercase hex, or `None` when the file does not exist.
pub(super) fn file_hash_md5(path: &Path) -> Result<Option<String>> {
    use md5::{Digest, Md5};

    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let mut md5 = Md5::new();
    let mut buffer = [0_u8; 64 * 1024];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        md5.update(&buffer[..read]);
    }
    Ok(Some(hex_encode(md5.finalize().as_slice())))
}

pub(super) fn ensure_md5(path: &Path, expected: &str) -> Result<()> {
    let actual = file_hash_md5(path)?.unwrap_or_default();
    if actual.eq_ignore_ascii_case(expected) {
        return Ok(());
    }
    let _ = fs::remove_file(path);
    Err(CoreError::Md5Mismatch {
        expected: expected.to_string(),
        actual,
    })
}

/// `true` when the file still has to be fetched (CurseForge: sha1 and/or md5).
///
/// An empty hash is not a match. An existing file is removed so the downloader
/// does not treat "the file exists" as success when SHA1 is missing.
pub(super) fn prepare_destination_sha1_md5(
    path: &Path,
    sha1: &str,
    md5: Option<&str>,
) -> Result<bool> {
    let md5 = md5.filter(|value| !value.is_empty());
    let has_sha1 = !sha1.is_empty();
    if !has_sha1 && md5.is_none() {
        if path.is_file() {
            fs::remove_file(path)?;
        }
        return Ok(true);
    }
    let sha1_ok =
        !has_sha1 || hash_file(path)?.is_some_and(|(actual, _)| actual.eq_ignore_ascii_case(sha1));
    let md5_ok = match md5 {
        None => true,
        Some(expected) => {
            file_hash_md5(path)?.is_some_and(|actual| actual.eq_ignore_ascii_case(expected))
        }
    };
    if sha1_ok && md5_ok && path.is_file() {
        return Ok(false);
    }
    if path.is_file() && (!has_sha1 || sha1_ok) {
        fs::remove_file(path)?;
    }
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    const HELLO_WORLD_SHA1: &str = "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed";
    const HELLO_WORLD_MD5: &str = "5eb63bbbe01eeed093cb22bb8f5acdc3";

    #[tokio::test]
    async fn verification_pauses_between_chunks_and_resumes_with_the_same_hashes() {
        use sha2::{Digest, Sha512};
        use std::future::Future;
        use std::task::Poll;
        use tokio::io::AsyncWriteExt;

        let (mut writer, mut reader) = tokio::io::duplex(64 * 1024);
        let first = vec![b'a'; 64 * 1024];
        let second = vec![b'b'; 64 * 1024];
        let third = vec![b'c'; 64 * 1024];
        let expected = [first.clone(), second.clone(), third.clone()].concat();
        writer.write_all(&first).await.expect("first chunk");
        let control = DownloadControl::new();
        let mut hashing = Box::pin(hash_reader(&mut reader, &control));
        std::future::poll_fn(|cx| {
            assert!(hashing.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        control.pause();
        writer.write_all(&second).await.expect("second chunk");
        std::future::poll_fn(|cx| {
            assert!(hashing.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        let mut blocked = Box::pin(writer.write_all(&third));
        std::future::poll_fn(|cx| {
            assert!(
                blocked.as_mut().poll(cx).is_pending(),
                "paused hashing must not consume the next chunk"
            );
            Poll::Ready(())
        })
        .await;
        drop(blocked);
        control.resume();
        let feed = async {
            writer.write_all(&third).await.expect("third chunk");
            writer.shutdown().await.expect("end of stream");
        };
        let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(10), async {
            tokio::join!(hashing, feed)
        })
        .await
        .expect("resume must complete");
        let (sha1, sha512) = result.expect("hashes");
        assert_eq!(sha1, crate::hashes::sha1_hex(&expected));
        assert_eq!(sha512, hex_encode(Sha512::digest(&expected).as_slice()));
    }

    #[tokio::test]
    async fn verification_cancellation_after_a_chunk_stops_before_the_next_read() {
        use std::future::Future;
        use std::task::Poll;
        use tokio::io::{AsyncReadExt, AsyncWriteExt};

        let (mut writer, mut reader) = tokio::io::duplex(64 * 1024);
        writer
            .write_all(&vec![b'a'; 64 * 1024])
            .await
            .expect("first chunk");
        let control = DownloadControl::new();
        let mut hashing = Box::pin(hash_reader(&mut reader, &control));
        std::future::poll_fn(|cx| {
            assert!(hashing.as_mut().poll(cx).is_pending());
            Poll::Ready(())
        })
        .await;
        writer.write_all(b"unread").await.expect("remaining bytes");
        control.cancel();
        assert!(matches!(hashing.await, Err(CoreError::DownloadCancelled)));
        writer.shutdown().await.expect("end stream");
        let mut remaining = Vec::new();
        reader
            .read_to_end(&mut remaining)
            .await
            .expect("unread bytes");
        assert_eq!(remaining, b"unread");
    }

    #[test]
    fn hashes_a_file_and_reports_missing_ones() {
        let root = temp_dir("hash");
        let path = root.join("mod.jar");
        assert!(hash_file(&path).unwrap().is_none());
        fs::write(&path, b"hello").unwrap();
        let (sha1, sha512) = hash_file(&path).unwrap().expect("hashes");
        assert_eq!(sha1, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
        assert_eq!(sha512.len(), 128);
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn sha512_mismatch_removes_the_cached_file() {
        let root = temp_dir("sha512");
        let path = root.join("mod.jar");
        fs::write(&path, b"hello").unwrap();
        let (sha1, sha512) = hash_file(&path).unwrap().expect("hashes");
        let control = DownloadControl::new();
        assert!(
            !prepare_download(&path, &sha1, &sha512, &control)
                .await
                .unwrap()
        );
        assert!(path.is_file());
        assert!(
            prepare_download(&path, &sha1, "deadbeef", &control)
                .await
                .unwrap()
        );
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn empty_sha1_does_not_accept_an_existing_mod_file() {
        let root = temp_dir("empty-sha");
        let path = root.join("mod.jar");
        fs::write(&path, b"hello").unwrap();
        let control = DownloadControl::new();
        assert!(
            prepare_download(&path, "", "deadbeef", &control)
                .await
                .unwrap()
        );
        assert_eq!(fs::read(&path).unwrap(), b"hello");
        let error = ensure_sha512(&path, "00", &control).await.unwrap_err();
        assert!(matches!(error, CoreError::Sha512Mismatch { .. }));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[tokio::test]
    async fn ensure_sha512_accepts_a_matching_file() {
        let root = temp_dir("ensure");
        let path = root.join("mod.jar");
        fs::write(&path, b"hello").unwrap();
        let (_, sha512) = hash_file(&path).unwrap().expect("hashes");
        ensure_sha512(&path, &sha512.to_uppercase(), &DownloadControl::new())
            .await
            .expect("matches");
        assert!(path.is_file());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_file_without_a_hash_is_not_treated_as_installed() {
        let root = temp_dir("no-hash");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        assert!(prepare_destination_sha1_md5(&path, "", None).unwrap());
        assert!(!path.exists());
        fs::write(&path, b"hello world").unwrap();
        assert!(prepare_destination_sha1_md5(&path, "", Some("")).unwrap());
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sha1_or_md5_match_skips_the_download_and_a_mismatch_does_not() {
        let root = temp_dir("sha1-md5");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        assert!(!prepare_destination_sha1_md5(&path, HELLO_WORLD_SHA1, None).unwrap());
        assert!(path.exists());
        assert!(!prepare_destination_sha1_md5(&path, "", Some(HELLO_WORLD_MD5)).unwrap());
        assert!(path.exists());
        assert!(
            prepare_destination_sha1_md5(&path, HELLO_WORLD_SHA1, Some("0".repeat(32).as_str()))
                .unwrap()
        );
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_file_has_to_be_fetched() {
        let root = temp_dir("missing");
        let path = root.join("a.jar");
        assert!(file_hash_md5(&path).unwrap().is_none());
        assert!(prepare_destination_sha1_md5(&path, HELLO_WORLD_SHA1, None).unwrap());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn md5_mismatch_removes_the_file() {
        let root = temp_dir("md5");
        let path = root.join("a.jar");
        fs::write(&path, b"hello world").unwrap();
        ensure_md5(&path, &HELLO_WORLD_MD5.to_uppercase()).expect("matches");
        assert!(path.is_file());
        let error = ensure_md5(&path, &"0".repeat(32)).unwrap_err();
        assert!(matches!(error, CoreError::Md5Mismatch { .. }));
        assert!(!path.exists());
        let _ = fs::remove_dir_all(&root);
    }

    fn temp_dir(label: &str) -> PathBuf {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "licht-hashes-{label}-{}-{stamp}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        root
    }
}
