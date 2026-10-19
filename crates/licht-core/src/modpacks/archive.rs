use std::io::Read;

use crate::{CoreError, Result};

const MAX_METADATA_BYTES: u64 = 64 * 1024 * 1024;

pub(super) fn read_metadata(reader: impl Read) -> Result<String> {
    let mut body = String::new();
    // Read one extra byte to distinguish an exact-size entry from an oversized one.
    reader
        .take(MAX_METADATA_BYTES + 1)
        .read_to_string(&mut body)
        .map_err(|_| CoreError::ModpackArchive)?;

    if body.len() as u64 > MAX_METADATA_BYTES {
        return Err(CoreError::ModpackArchive);
    }

    Ok(body)
}

#[cfg(test)]
pub(super) fn metadata_zip(name: &str, json: &str, uncompressed_size: usize) -> Vec<u8> {
    use std::io::{Cursor, Write};

    use zip::CompressionMethod;
    use zip::write::SimpleFileOptions;

    let mut buffer = Cursor::new(Vec::new());
    {
        let mut archive = zip::ZipWriter::new(&mut buffer);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        archive.start_file(name, options).expect("metadata entry");
        archive.write_all(json.as_bytes()).expect("JSON body");
        let padding_size = uncompressed_size
            .checked_sub(json.len())
            .expect("JSON fits");
        let mut padding = std::io::repeat(b' ').take(padding_size as u64);
        std::io::copy(&mut padding, &mut archive).expect("JSON whitespace");
        archive.finish().expect("compressed archive");
    }
    buffer.into_inner()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stops_reading_after_the_limit_and_one_detection_byte() {
        let mut source = std::io::repeat(b' ').take(64 * 1024 * 1024 + 1024);

        assert!(matches!(
            read_metadata(&mut source),
            Err(CoreError::ModpackArchive)
        ));
        assert_eq!(source.limit(), 1023);
    }

    #[test]
    fn rejects_invalid_utf8() {
        assert!(matches!(
            read_metadata(&b"\xff"[..]),
            Err(CoreError::ModpackArchive)
        ));
    }

    #[test]
    fn rejects_read_failure() {
        struct FailingReader;

        impl Read for FailingReader {
            fn read(&mut self, _buffer: &mut [u8]) -> std::io::Result<usize> {
                Err(std::io::Error::other("fixture read failure"))
            }
        }

        assert!(matches!(
            read_metadata(FailingReader),
            Err(CoreError::ModpackArchive)
        ));
    }
}
