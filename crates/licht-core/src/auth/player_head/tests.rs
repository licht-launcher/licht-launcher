use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

fn texture(height: u32) -> Vec<u8> {
    let mut pixels = vec![0; 64 * height as usize * 4];
    for y in 0..8 {
        for x in 0..8 {
            let position = ((y + 8) * 64 + x + 8) * 4;
            pixels[position..position + 4].copy_from_slice(&[100 + x as u8, 50 + y as u8, 0, 255]);
        }
    }
    let hat = (8 * 64 + 40) * 4;
    pixels[hat..hat + 4].copy_from_slice(&[0, 100, 200, 128]);
    let mut bytes = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut bytes, 64, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().expect("fixture header");
        writer.write_image_data(&pixels).expect("fixture pixels");
        writer.finish().expect("fixture finish");
    }
    bytes
}

struct Fixture(SharedCache);
impl Fixture {
    fn new(name: &str) -> Self {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        Self(SharedCache::at(std::env::temp_dir().join(format!(
            "licht-player-head-{}-{name}-{stamp}",
            std::process::id()
        ))))
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        if self.0.root().exists() {
            if let Err(error) = fs::remove_dir_all(self.0.root())
                && !std::thread::panicking()
            {
                panic!("fixture cleanup failed: {error}");
            }
        }
    }
}

fn offline(_client: Client, _source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
    Box::pin(async { Err(CoreError::SkinImage) })
}

#[test]
fn modern_and_legacy_skins_are_cropped_and_the_hat_is_alpha_composited() {
    for height in [32, 64] {
        let bytes = render_head(&texture(height)).expect("head PNG");
        let head = decode_png(&bytes).expect("decoded head");
        assert_eq!((head.width, head.height), (8, 8));
        assert_eq!(&head.rgba[..4], &[50, 75, 100, 255]);
        assert_eq!(&head.rgba[63 * 4..64 * 4], &[107, 57, 0, 255]);
    }
}

#[test]
fn malformed_and_wrong_sized_images_are_rejected() {
    assert!(matches!(
        render_head(b"not a PNG"),
        Err(CoreError::SkinImage)
    ));
    assert!(matches!(
        render_head(&vec![0; MAX_PNG_BYTES + 1]),
        Err(CoreError::SkinImage)
    ));
    let tiny = encode_head(&[0; 8 * 8 * 4]).expect("valid small PNG");
    assert!(matches!(render_head(&tiny), Err(CoreError::SkinImage)));
}

#[tokio::test]
async fn cached_heads_survive_reopening_and_corruption_is_rebuilt() {
    static FETCHES: AtomicUsize = AtomicUsize::new(0);
    fn fetch(_client: Client, _source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
        Box::pin(async {
            FETCHES.fetch_add(1, Ordering::SeqCst);
            Ok(texture(64))
        })
    }
    let fixture = Fixture::new("reuse");
    let client = Client::new();
    let source = format!("https://textures.minecraft.net/texture/{}", "a".repeat(64));
    let first = load_head(&fixture.0, &client, Some(&source), fetch)
        .await
        .expect("initial head");
    assert!(first.starts_with(fixture.0.root().join("cache").join("player-heads")));
    let reopened = SharedCache::at(fixture.0.root().to_path_buf());
    assert_eq!(
        load_head(&reopened, &client, Some(&source), offline)
            .await
            .expect("disk cache without network"),
        first
    );
    assert_eq!(FETCHES.load(Ordering::SeqCst), 1);
    fs::write(&first, b"corrupted head").expect("corrupt PNG");
    assert_eq!(
        load_head(&reopened, &client, Some(&source), fetch)
            .await
            .expect("rebuilt"),
        first
    );
    assert_eq!(FETCHES.load(Ordering::SeqCst), 2);
    fs::write(first.with_extension("sha1"), b"invalid checksum").expect("corrupt receipt");
    load_head(&reopened, &client, Some(&source), fetch)
        .await
        .expect("checksum repaired");
    assert_eq!(FETCHES.load(Ordering::SeqCst), 3);
    let bytes = fs::read(&first).expect("PNG");
    assert_eq!(
        fs::read_to_string(first.with_extension("sha1")).expect("checksum"),
        crate::hashes::sha1_hex(&bytes)
    );
}

#[tokio::test]
async fn player_skin_is_preferred_and_steve_is_cached_only_as_a_fallback() {
    fn fetch(_client: Client, _source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
        Box::pin(async { Ok(texture(64)) })
    }
    fn fallback_only(_client: Client, source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
        Box::pin(async move {
            if source == DEFAULT_SKIN_URL {
                Ok(texture(64))
            } else {
                Err(CoreError::SkinImage)
            }
        })
    }
    let fixture = Fixture::new("fallback");
    let client = Client::new();
    let source = format!("https://textures.minecraft.net/texture/{}", "b".repeat(64));
    let player = load_head(&fixture.0, &client, Some(&source), fetch)
        .await
        .expect("player head");
    assert!(
        !head_path(&fixture.0, DEFAULT_SKIN_URL).exists(),
        "Steve must not be downloaded when the player skin is available"
    );
    let missing = format!("https://textures.minecraft.net/texture/{}", "c".repeat(64));
    let steve = load_head(&fixture.0, &client, Some(&missing), fallback_only)
        .await
        .expect("Steve fallback");
    assert_ne!(player, steve);
    assert_eq!(steve, head_path(&fixture.0, DEFAULT_SKIN_URL));
    assert_eq!(
        load_head(&fixture.0, &client, None, offline)
            .await
            .expect("offline Steve cache"),
        steve
    );
    assert_eq!(
        load_head(&fixture.0, &client, Some(&source), offline)
            .await
            .expect("player cache still preferred"),
        player
    );
}

#[tokio::test]
async fn untrusted_skin_urls_are_rejected_without_network_or_cache_writes() {
    let fixture = Fixture::new("invalid-source");
    for source in [
        "file:///etc/passwd",
        "https://example.com/skin.png",
        "https://textures.minecraft.net/texture/not-a-hash",
    ] {
        assert!(matches!(
            load_head(&fixture.0, &Client::new(), Some(source), offline).await,
            Err(CoreError::SkinImage)
        ));
    }
    assert!(!fixture.0.root().exists());
}

#[tokio::test]
async fn concurrent_requests_publish_one_complete_head() {
    static FETCHES: AtomicUsize = AtomicUsize::new(0);
    fn fetch(_client: Client, _source: String) -> BoxFuture<'static, Result<Vec<u8>>> {
        Box::pin(async {
            FETCHES.fetch_add(1, Ordering::SeqCst);
            tokio::task::yield_now().await;
            Ok(texture(64))
        })
    }
    let fixture = Fixture::new("concurrent");
    let client = Client::new();
    let source = format!("https://textures.minecraft.net/texture/{}", "d".repeat(64));
    let (first, second) = tokio::join!(
        load_head(&fixture.0, &client, Some(&source), fetch),
        load_head(&fixture.0, &client, Some(&source), fetch)
    );
    assert_eq!(first.expect("first"), second.expect("second"));
    assert_eq!(FETCHES.load(Ordering::SeqCst), 1);
}
