//! Downloading the pinned Nerd Font (Step 12o, ADR-055).
//!
//! The archive is kept in memory and checked against its pinned SHA-256 before anything is
//! unpacked; each extracted font is checked against its own pinned hash too. Nothing is
//! written to disk here, so a cut or tampered download leaves no file behind. File names come
//! from [`FILES`], never from the archive, so a crafted entry name cannot escape.

use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use sha2::{Digest, Sha256};

/// Nerd Fonts release the app installs. Changes only with an app update.
pub const VERSION: &str = "3.5.1";
pub const URL: &str =
    "https://github.com/ryanoasis/nerd-fonts/releases/download/v3.5.1/CascadiaMono.tar.xz";
/// The archive's size in bytes, shown before the user agrees to download it.
pub const SIZE: u64 = 3_577_856;
const SHA256: &str = "e4fe0248bbe2558e907d26b55ed287403972f946239a4ea3f7a8afd3365ad6ba";
/// GitHub answers `URL` with a redirect to this host; no other redirect is followed.
const ASSET_HOST: &str = "release-assets.githubusercontent.com";
/// The family name the fonts below register (name ID 16), written into terminal settings.
pub const FAMILY: &str = "CaskaydiaMono Nerd Font Mono";

/// The four faces taken from the archive, with their pinned SHA-256.
pub const FILES: [(&str, &str); 4] = [
    (
        "CaskaydiaMonoNerdFontMono-Regular.ttf",
        "0bc1e80eb7d1c0a1debb433a21da6e686b15556e1d54fcfe47f87f7379276830",
    ),
    (
        "CaskaydiaMonoNerdFontMono-Bold.ttf",
        "55fe130e13258d1dfc7b46f373f0a21ef5bdd8cc97b0dfac118e45685f93e516",
    ),
    (
        "CaskaydiaMonoNerdFontMono-Italic.ttf",
        "dd7a5723b449417dc815691b4e793f42fbad2ca4395e60c56198a49d5d90f281",
    ),
    (
        "CaskaydiaMonoNerdFontMono-BoldItalic.ttf",
        "a8fe6269bec2920d335ff801e1d3b75738029437079eaf12a41e330653ea1450",
    ),
];

const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
/// Between two chunks of the body.
const READ_TIMEOUT: Duration = Duration::from_secs(30);
/// The archive's dictionary is 64 MiB; a little room on top for the decoder's own state.
const XZ_MEM_LIMIT_KIB: u32 = 72 * 1024;
/// No single font in the archive comes near this.
const MAX_FONT_BYTES: u64 = 32 << 20;

/// Why the font could not be fetched. Categories only; `reqwest` messages are not passed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum DownloadError {
    #[error("could not connect")]
    Connect,
    #[error("the download timed out")]
    Timeout,
    #[error("the server answered {0}")]
    Status(u16),
    #[error("the download failed")]
    Request,
    #[error("the download does not match the pinned checksum")]
    Checksum,
    #[error("the archive could not be read")]
    Archive,
    #[error("cancelled")]
    Cancelled,
    #[error("could not start the HTTP client")]
    Setup,
}

/// One extracted font: its pinned file name and its bytes, already verified.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFile {
    pub name: &'static str,
    pub bytes: Vec<u8>,
}

/// Downloads [`URL`] and returns the verified fonts. `progress(done, total)` is called as bytes
/// arrive; setting `cancel` stops within a chunk.
pub fn fetch(
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Vec<FontFile>, DownloadError> {
    let archive = download(progress, cancel)?;
    if hex(&Sha256::digest(&archive)) != SHA256 {
        return Err(DownloadError::Checksum);
    }
    extract(&archive)
}

fn download(
    progress: &mut dyn FnMut(u64, u64),
    cancel: &AtomicBool,
) -> Result<Vec<u8>, DownloadError> {
    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| DownloadError::Setup)?;
    let client = {
        let _guard = rt.enter();
        reqwest::Client::builder()
            .user_agent(concat!("TokenRadar/", env!("CARGO_PKG_VERSION")))
            .https_only(true)
            .connect_timeout(CONNECT_TIMEOUT)
            .read_timeout(READ_TIMEOUT)
            .redirect(reqwest::redirect::Policy::custom(|attempt| {
                if attempt.previous().len() < 3 && allowed_redirect(attempt.url()) {
                    attempt.follow()
                } else {
                    attempt.stop()
                }
            }))
            .build()
            .map_err(|_| DownloadError::Setup)?
    };
    rt.block_on(async {
        let mut response = client.get(URL).send().await.map_err(|e| classify(&e))?;
        let status = response.status();
        if !status.is_success() {
            return Err(DownloadError::Status(status.as_u16()));
        }
        // The pinned size is the limit: a longer body cannot match the hash anyway.
        let mut body = Vec::with_capacity(usize::try_from(SIZE).unwrap_or(0));
        progress(0, SIZE);
        while let Some(chunk) = response.chunk().await.map_err(|e| classify(&e))? {
            if cancel.load(Ordering::Relaxed) {
                return Err(DownloadError::Cancelled);
            }
            if (body.len() + chunk.len()) as u64 > SIZE {
                return Err(DownloadError::Checksum);
            }
            body.extend_from_slice(&chunk);
            progress(body.len() as u64, SIZE);
        }
        Ok(body)
    })
}

/// Only GitHub's release-asset host, over HTTPS.
fn allowed_redirect(url: &reqwest::Url) -> bool {
    url.scheme() == "https" && url.host_str() == Some(ASSET_HOST)
}

fn classify(e: &reqwest::Error) -> DownloadError {
    if e.is_timeout() {
        DownloadError::Timeout
    } else if e.is_connect() {
        DownloadError::Connect
    } else if e.is_redirect() {
        DownloadError::Status(302)
    } else {
        DownloadError::Request
    }
}

/// Unpacks the [`FILES`] from a verified `.tar.xz`, checking each one's hash.
fn extract(archive: &[u8]) -> Result<Vec<FontFile>, DownloadError> {
    let xz = lzma_rust2::XzReader::new_mem_limit(archive, false, XZ_MEM_LIMIT_KIB);
    let mut found = untar(xz, &FILES.map(|(name, _)| name))?;
    let mut out = Vec::with_capacity(FILES.len());
    for (name, sha) in FILES {
        let i = found
            .iter()
            .position(|f| f.name == name)
            .ok_or(DownloadError::Archive)?;
        let file = found.swap_remove(i);
        if hex(&Sha256::digest(&file.bytes)) != sha {
            return Err(DownloadError::Checksum);
        }
        out.push(file);
    }
    Ok(out)
}

/// Reads a ustar stream and keeps the regular files named in `wanted`; everything else is
/// skipped without being kept. Stops once all are found.
fn untar(mut r: impl Read, wanted: &[&'static str]) -> Result<Vec<FontFile>, DownloadError> {
    let mut found = Vec::new();
    let mut header = [0u8; 512];
    while found.len() < wanted.len() {
        r.read_exact(&mut header)
            .map_err(|_| DownloadError::Archive)?;
        if header.iter().all(|&b| b == 0) {
            break;
        }
        let size = octal(&header[124..136]).ok_or(DownloadError::Archive)?;
        let padded = size.div_ceil(512) * 512;
        let name = field(&header[..100]);
        let kind = header[156];
        let wanted_name = wanted
            .iter()
            .find(|w| **w == name && matches!(kind, b'0' | 0))
            .copied();
        match wanted_name {
            Some(name) if size <= MAX_FONT_BYTES => {
                let mut bytes = vec![0; usize::try_from(size).map_err(|_| DownloadError::Archive)?];
                r.read_exact(&mut bytes)
                    .map_err(|_| DownloadError::Archive)?;
                skip(&mut r, padded - size)?;
                found.push(FontFile { name, bytes });
            }
            Some(_) => return Err(DownloadError::Archive),
            None => skip(&mut r, padded)?,
        }
    }
    Ok(found)
}

fn skip(r: &mut impl Read, n: u64) -> Result<(), DownloadError> {
    let copied =
        std::io::copy(&mut r.take(n), &mut std::io::sink()).map_err(|_| DownloadError::Archive)?;
    if copied == n {
        Ok(())
    } else {
        Err(DownloadError::Archive)
    }
}

/// A NUL-terminated header field.
fn field(b: &[u8]) -> &str {
    let end = b.iter().position(|&c| c == 0).unwrap_or(b.len());
    std::str::from_utf8(&b[..end]).unwrap_or("")
}

fn octal(b: &[u8]) -> Option<u64> {
    let s = field(b).trim_matches([' ', '\0']);
    u64::from_str_radix(s, 8).ok()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes.iter().fold(String::with_capacity(64), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Whether `bytes` are the pinned file `name`. A removal takes only such files as the ones
/// TokenRadar installed; the same name with other bytes was put there by someone else.
pub(crate) fn is_pinned(name: &str, bytes: &[u8]) -> bool {
    FILES
        .iter()
        .any(|(n, sha)| *n == name && hex(&Sha256::digest(bytes)) == *sha)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(name: &str, kind: u8, body: &[u8]) -> Vec<u8> {
        let mut h = [0u8; 512];
        h[..name.len()].copy_from_slice(name.as_bytes());
        let size = format!("{:011o}\0", body.len());
        h[124..136].copy_from_slice(size.as_bytes());
        h[156] = kind;
        h[257..263].copy_from_slice(b"ustar\0");
        let mut out = h.to_vec();
        out.extend_from_slice(body);
        out.resize(out.len().div_ceil(512) * 512, 0);
        out
    }

    #[test]
    fn untar_keeps_only_the_named_regular_files() {
        let mut tar = Vec::new();
        tar.extend(entry("README.md", b'0', b"readme"));
        tar.extend(entry("../evil.ttf", b'0', b"evil"));
        tar.extend(entry("b.ttf", b'5', b""));
        tar.extend(entry("a.ttf", b'0', &[7; 700]));
        tar.extend(entry("b.ttf", b'0', b"bee"));
        tar.extend([0; 1024]);
        let got = untar(tar.as_slice(), &["a.ttf", "b.ttf"]).expect("untar");
        assert_eq!(got.len(), 2);
        assert_eq!((got[0].name, got[0].bytes.len()), ("a.ttf", 700));
        assert_eq!(
            (got[1].name, got[1].bytes.as_slice()),
            ("b.ttf", &b"bee"[..])
        );
        // Missing files: the stream ends first.
        let partial = untar(&tar[..1536], &["a.ttf"]);
        assert_eq!(partial, Err(DownloadError::Archive));
    }

    #[test]
    fn a_wrong_archive_is_refused_before_unpacking() {
        assert_eq!(extract(b"not xz"), Err(DownloadError::Archive));
        assert_ne!(hex(&Sha256::digest(b"not the font")), SHA256);
        assert_eq!(hex(&[0x0b, 0xc1]), "0bc1");
    }

    #[test]
    fn only_the_asset_host_may_redirect() {
        let ok = |u: &str| allowed_redirect(&reqwest::Url::parse(u).expect("url"));
        assert!(ok(
            "https://release-assets.githubusercontent.com/github-production-release-asset/1"
        ));
        assert!(!ok("http://release-assets.githubusercontent.com/x"));
        assert!(!ok("https://example.com/CascadiaMono.tar.xz"));
        assert!(!ok(
            "https://release-assets.githubusercontent.com.evil.test/x"
        ));
    }

    #[test]
    fn pinned_names_are_plain_file_names() {
        for (name, sha) in FILES {
            assert!(
                !name.contains(['/', '\\', ':']) && name.to_ascii_lowercase().ends_with(".ttf"),
                "{name}"
            );
            assert_eq!(sha.len(), 64);
        }
        assert_eq!(SHA256.len(), 64);
    }

    /// Unpacks the real archive when `TOKENRADAR_NERD_FONT_ARCHIVE` points at a downloaded copy.
    #[test]
    #[ignore = "needs the downloaded archive"]
    fn the_real_archive_unpacks() {
        let path = std::env::var_os("TOKENRADAR_NERD_FONT_ARCHIVE").expect("set the variable");
        let archive = std::fs::read(path).expect("read");
        assert_eq!(hex(&Sha256::digest(&archive)), SHA256);
        let files = extract(&archive).expect("extract");
        assert_eq!(files.len(), 4);
    }
}
