//! Which installed fonts are Nerd Fonts (Step 12o).
//!
//! Only the table directory and the `name`, `cmap`, `hhea` and `hmtx` tables are read, each
//! with one seek, so scanning the whole Windows font folder stays cheap even with large CJK
//! fonts in it. Nothing is cached: fonts can be installed at any time.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use specta::Type;

/// Powerline's branch glyph: every Powerline-patched font has it, but so does Segoe UI Symbol,
/// so it alone does not make a Nerd Font.
const POWERLINE: u32 = 0xE0A0;
/// `oct-git_branch`, in every Nerd Fonts 3.x font.
const OCTICON: u32 = 0xF418;
/// `extra-progress_empty_left`, added in Nerd Fonts 3.4.0 (2025-04-24).
const PROGRESS: u32 = 0xEE00;

/// How much of Nerd Fonts a font covers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Type)]
#[serde(rename_all = "snake_case")]
pub enum NerdTier {
    None,
    /// Icons, but not the progress glyphs of 3.4 (Microsoft's own "Cascadia Code NF", say).
    Old,
    /// Nerd Fonts 3.4 or later.
    Full,
}

/// One face of an installed font.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Face {
    /// The typographic family name (name ID 16), else the family name (ID 1).
    pub family: String,
    /// The other of the two names when they differ (`CaskaydiaMono NFM`).
    pub short_family: Option<String>,
    pub tier: NerdTier,
    /// Icons are as wide as a letter: one terminal cell (the `Nerd Font Mono` variants).
    pub mono: bool,
    pub path: PathBuf,
}

/// Every face found in the font folders.
#[derive(Debug, Clone, Default)]
pub struct FontScan {
    pub faces: Vec<Face>,
}

impl FontScan {
    /// The best Nerd Font installed: highest tier, then one-cell icons, then by name.
    pub fn best(&self) -> Option<&Face> {
        self.faces
            .iter()
            .filter(|f| f.tier > NerdTier::None)
            .max_by(|a, b| {
                (a.tier, a.mono)
                    .cmp(&(b.tier, b.mono))
                    .then_with(|| b.family.cmp(&a.family))
            })
    }

    /// The best face of the family called `name` (either of its names, any case).
    pub fn family(&self, name: &str) -> Option<&Face> {
        let name = name.trim();
        self.faces
            .iter()
            .filter(|f| {
                f.family.eq_ignore_ascii_case(name)
                    || f.short_family
                        .as_deref()
                        .is_some_and(|s| s.eq_ignore_ascii_case(name))
            })
            .max_by_key(|f| (f.tier, f.mono))
    }
}

/// The folders fonts are installed into: the system's and the current user's.
pub fn font_dirs(env: &dyn Fn(&str) -> Option<std::ffi::OsString>) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = env("WINDIR").or_else(|| env("SystemRoot")) {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    if let Some(local) = env("LOCALAPPDATA") {
        dirs.push(super::install::user_font_dir(Path::new(&local)));
    }
    dirs
}

/// Scans every font file in `dirs`. Unreadable or malformed files are skipped.
pub fn scan(dirs: &[PathBuf]) -> FontScan {
    let mut faces = Vec::new();
    for dir in dirs {
        let Ok(entries) = std::fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let is_font = path.extension().and_then(|e| e.to_str()).is_some_and(|e| {
                ["ttf", "otf", "ttc", "otc"]
                    .iter()
                    .any(|x| e.eq_ignore_ascii_case(x))
            });
            if is_font {
                faces.extend(read_file(&path).unwrap_or_default());
            }
        }
    }
    FontScan { faces }
}

/// The faces of one font file (several for a collection).
pub fn read_file(path: &Path) -> Option<Vec<Face>> {
    let mut f = File::open(path).ok()?;
    let head = read_at(&mut f, 0, 12)?;
    let offsets = if &head[..4] == b"ttcf" {
        let count = u32_at(&head, 8)?.min(64);
        let raw = read_at(&mut f, 12, count as usize * 4)?;
        (0..count as usize)
            .filter_map(|i| u32_at(&raw, i * 4))
            .collect()
    } else {
        vec![0]
    };
    Some(
        offsets
            .into_iter()
            .filter_map(|at| read_face(&mut f, u64::from(at), path))
            .collect(),
    )
}

fn read_face(f: &mut File, at: u64, path: &Path) -> Option<Face> {
    let head = read_at(f, at, 12)?;
    let count = usize::from(u16_at(&head, 4)?);
    let dir = read_at(f, at + 12, count * 16)?;
    let table = |tag: &[u8; 4]| {
        (0..count).find_map(|i| {
            let rec = &dir[i * 16..i * 16 + 16];
            (&rec[..4] == tag).then(|| (u32_at(rec, 8), u32_at(rec, 12)))
        })
    };
    let (name_at, name_len) = table(b"name")?;
    let name = read_at(f, u64::from(name_at?), (name_len?).min(1 << 20) as usize)?;
    let (family, short_family) = family_names(&name)?;
    let (cmap_at, _) = table(b"cmap")?;
    let cmap = Cmap::read(f, u64::from(cmap_at?))?;
    let has = |c: u32| cmap.glyph(c).is_some();
    let tier = match (has(POWERLINE), has(OCTICON), has(PROGRESS)) {
        (_, true, true) => NerdTier::Full,
        (true, true, false) => NerdTier::Old,
        _ => NerdTier::None,
    };
    let mono = tier > NerdTier::None
        && match (cmap.glyph(OCTICON), cmap.glyph(u32::from('A'))) {
            (Some(icon), Some(letter)) => {
                let hhea = table(b"hhea").and_then(|(o, _)| o);
                let hmtx = table(b"hmtx").and_then(|(o, _)| o);
                match (hhea, hmtx) {
                    (Some(hhea), Some(hmtx)) => {
                        let mut advance = |g| advance(f, u64::from(hhea), u64::from(hmtx), g);
                        matches!((advance(icon), advance(letter)), (Some(i), Some(l)) if i <= l)
                    }
                    _ => false,
                }
            }
            _ => false,
        };
    Some(Face {
        family,
        short_family,
        tier,
        mono,
        path: path.to_path_buf(),
    })
}

/// `(typographic family, family)` from a `name` table, Windows English names first.
fn family_names(data: &[u8]) -> Option<(String, Option<String>)> {
    let table = ttf_parser::name::Table::parse(data)?;
    let pick = |id: u16| {
        let mut best: Option<(u8, String)> = None;
        for n in table.names {
            if n.name_id != id {
                continue;
            }
            let Some(s) = n.to_string().filter(|s| !s.trim().is_empty()) else {
                continue;
            };
            let rank = match (n.platform_id, n.language_id) {
                (ttf_parser::PlatformId::Windows, 0x0409) => 0,
                (ttf_parser::PlatformId::Windows, _) => 1,
                _ => 2,
            };
            if best.as_ref().is_none_or(|(r, _)| rank < *r) {
                best = Some((rank, s));
            }
        }
        best.map(|(_, s)| s)
    };
    let family = pick(1);
    match pick(16) {
        Some(typographic) => {
            let short = family.filter(|f| *f != typographic);
            Some((typographic, short))
        }
        None => family.map(|f| (f, None)),
    }
}

/// The one `cmap` subtable that maps the Basic Multilingual Plane, which holds every code point
/// the check needs.
struct Cmap {
    /// A one-subtable `cmap` table rebuilt around the chosen subtable, so ttf-parser can read it.
    table: Vec<u8>,
}

impl Cmap {
    fn read(f: &mut File, at: u64) -> Option<Self> {
        let head = read_at(f, at, 4)?;
        let count = usize::from(u16_at(&head, 2)?);
        let records = read_at(f, at + 4, count * 8)?;
        let rank = |platform: u16, encoding: u16| match (platform, encoding) {
            (3, 1) | (0, 3) => Some(0),
            (3, 10) | (0, 4 | 6) => Some(1),
            (0, _) => Some(2),
            _ => None,
        };
        let (_, platform, encoding, offset) = (0..count)
            .filter_map(|i| {
                let r = &records[i * 8..i * 8 + 8];
                let (p, e) = (u16_at(r, 0)?, u16_at(r, 2)?);
                Some((rank(p, e)?, p, e, u32_at(r, 4)?))
            })
            .min_by_key(|(rank, ..)| *rank)?;
        let sub_at = at + u64::from(offset);
        let sub_head = read_at(f, sub_at, 8)?;
        let len = match u16_at(&sub_head, 0)? {
            0 | 4 | 6 => usize::from(u16_at(&sub_head, 2)?),
            12 | 13 => u32_at(&sub_head, 4)? as usize,
            _ => return None,
        };
        let sub = read_at(f, sub_at, len.min(16 << 20))?;
        let mut table = Vec::with_capacity(12 + sub.len());
        table.extend_from_slice(&[0, 0, 0, 1]);
        table.extend_from_slice(&platform.to_be_bytes());
        table.extend_from_slice(&encoding.to_be_bytes());
        table.extend_from_slice(&12u32.to_be_bytes());
        table.extend_from_slice(&sub);
        Some(Self { table })
    }

    fn glyph(&self, c: u32) -> Option<u16> {
        let table = ttf_parser::cmap::Table::parse(&self.table)?;
        let sub = table.subtables.into_iter().next()?;
        sub.glyph_index(c).map(|g| g.0).filter(|&g| g != 0)
    }
}

/// The advance width of glyph `g`, from `hhea` and the one `hmtx` entry it needs.
fn advance(f: &mut File, hhea: u64, hmtx: u64, g: u16) -> Option<u16> {
    let metrics = u16_at(&read_at(f, hhea + 34, 2)?, 0)?;
    let index = u64::from(g.min(metrics.checked_sub(1)?));
    u16_at(&read_at(f, hmtx + index * 4, 2)?, 0)
}

fn read_at(f: &mut File, at: u64, len: usize) -> Option<Vec<u8>> {
    f.seek(SeekFrom::Start(at)).ok()?;
    let mut buf = vec![0; len];
    f.read_exact(&mut buf).ok()?;
    Some(buf)
}

fn u16_at(b: &[u8], i: usize) -> Option<u16> {
    Some(u16::from_be_bytes(b.get(i..i + 2)?.try_into().ok()?))
}

fn u32_at(b: &[u8], i: usize) -> Option<u32> {
    Some(u32::from_be_bytes(b.get(i..i + 4)?.try_into().ok()?))
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    /// A minimal synthetic sfnt: `name` (IDs 1 and 16), a format 4 `cmap` mapping each of
    /// `chars` to glyphs 1.., `hhea` and `hmtx` with the given advances (glyph 0 first).
    #[allow(clippy::cast_possible_truncation)] // Test fonts are a few hundred bytes.
    pub(crate) fn font(
        family: &str,
        typographic: Option<&str>,
        chars: &[u32],
        advances: &[u16],
    ) -> Vec<u8> {
        let mut names: Vec<(u16, &str)> = vec![(1, family)];
        if let Some(t) = typographic {
            names.push((16, t));
        }
        let mut name = Vec::new();
        let mut strings = Vec::new();
        name.extend_from_slice(&0u16.to_be_bytes());
        name.extend_from_slice(&(names.len() as u16).to_be_bytes());
        name.extend_from_slice(&(6 + 12 * names.len() as u16).to_be_bytes());
        for (id, s) in &names {
            let utf16: Vec<u8> = s.encode_utf16().flat_map(u16::to_be_bytes).collect();
            for v in [
                3u16,
                1,
                0x0409,
                *id,
                utf16.len() as u16,
                strings.len() as u16,
            ] {
                name.extend_from_slice(&v.to_be_bytes());
            }
            strings.extend(utf16);
        }
        name.extend(strings);

        // Format 4, one segment per char plus the 0xFFFF terminator, idDelta mapping to glyph i+1.
        let mut sorted = chars.to_vec();
        sorted.sort_unstable();
        let seg = sorted.len() as u16 + 1;
        let mut sub = Vec::new();
        let mut body = Vec::new();
        let ends: Vec<u16> = sorted.iter().map(|&c| c as u16).chain([0xFFFF]).collect();
        for e in &ends {
            body.extend_from_slice(&e.to_be_bytes());
        }
        body.extend_from_slice(&0u16.to_be_bytes());
        for e in &ends {
            body.extend_from_slice(&e.to_be_bytes());
        }
        for (i, &c) in sorted.iter().enumerate() {
            let delta = (i as u16 + 1).wrapping_sub(c as u16);
            body.extend_from_slice(&delta.to_be_bytes());
        }
        body.extend_from_slice(&1u16.to_be_bytes());
        for _ in 0..seg {
            body.extend_from_slice(&0u16.to_be_bytes());
        }
        let len = 14 + body.len() as u16;
        for v in [4u16, len, 0, seg * 2, 0, 0, 0] {
            sub.extend_from_slice(&v.to_be_bytes());
        }
        sub.extend(body);
        let mut cmap = Vec::new();
        for v in [0u16, 1, 3, 1] {
            cmap.extend_from_slice(&v.to_be_bytes());
        }
        cmap.extend_from_slice(&12u32.to_be_bytes());
        cmap.extend(sub);

        let mut hhea = vec![0u8; 36];
        hhea[34..36].copy_from_slice(&(advances.len() as u16).to_be_bytes());
        let hmtx: Vec<u8> = advances
            .iter()
            .flat_map(|a| a.to_be_bytes().into_iter().chain([0, 0]))
            .collect();

        let tables: [(&[u8; 4], Vec<u8>); 4] = [
            (b"cmap", cmap),
            (b"hhea", hhea),
            (b"hmtx", hmtx),
            (b"name", name),
        ];
        let mut out = Vec::new();
        out.extend_from_slice(&0x0001_0000u32.to_be_bytes());
        out.extend_from_slice(&(tables.len() as u16).to_be_bytes());
        out.extend_from_slice(&[0; 6]);
        let mut offset = 12 + 16 * tables.len();
        let mut data = Vec::new();
        for (tag, t) in &tables {
            out.extend_from_slice(*tag);
            out.extend_from_slice(&0u32.to_be_bytes());
            out.extend_from_slice(&(offset as u32).to_be_bytes());
            out.extend_from_slice(&(t.len() as u32).to_be_bytes());
            data.extend_from_slice(t);
            while data.len() % 4 != 0 {
                data.push(0);
            }
            offset = 12 + 16 * tables.len() + data.len();
        }
        out.extend(data);
        out
    }

    fn scan_one(bytes: &[u8]) -> Face {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("f.TTF"), bytes).expect("write");
        std::fs::write(dir.path().join("notes.txt"), "not a font").expect("write");
        let mut scan = scan(&[dir.path().to_path_buf()]);
        assert_eq!(scan.faces.len(), 1);
        scan.faces.remove(0)
    }

    #[test]
    fn tiers_follow_the_code_points() {
        let a = u32::from('A');
        // Glyph order follows the sorted code points: A=1, E0A0=2, EE00=3, F418=4.
        let full = scan_one(&font(
            "CaskaydiaMono NFM",
            Some("CaskaydiaMono Nerd Font Mono"),
            &[a, POWERLINE, PROGRESS, OCTICON],
            &[0, 1200, 1200, 1200, 1200],
        ));
        assert_eq!(
            (
                full.family.as_str(),
                full.short_family.as_deref(),
                full.tier,
                full.mono
            ),
            (
                "CaskaydiaMono Nerd Font Mono",
                Some("CaskaydiaMono NFM"),
                NerdTier::Full,
                true
            )
        );
        let old = scan_one(&font(
            "Cascadia Mono NF",
            None,
            &[a, POWERLINE, OCTICON],
            &[0, 1200, 1200, 2400],
        ));
        assert_eq!(
            (old.tier, old.mono, old.short_family),
            (NerdTier::Old, false, None)
        );
        // Segoe UI Symbol has the Powerline glyph but no Nerd Font icons.
        let symbol = scan_one(&font(
            "Segoe UI Symbol",
            None,
            &[a, POWERLINE],
            &[0, 1200, 1200],
        ));
        assert_eq!(symbol.tier, NerdTier::None);
    }

    #[test]
    fn best_prefers_tier_then_one_cell_icons() {
        let face = |family: &str, tier, mono| Face {
            family: family.into(),
            short_family: None,
            tier,
            mono,
            path: PathBuf::new(),
        };
        let scan = FontScan {
            faces: vec![
                face("Consolas", NerdTier::None, false),
                face("Cascadia Code NF", NerdTier::Old, true),
                face("CaskaydiaMono Nerd Font", NerdTier::Full, false),
                face("CaskaydiaMono Nerd Font Mono", NerdTier::Full, true),
            ],
        };
        assert_eq!(
            scan.best().map(|f| f.family.as_str()),
            Some("CaskaydiaMono Nerd Font Mono")
        );
        assert_eq!(
            scan.family("cascadia code nf").map(|f| f.tier),
            Some(NerdTier::Old)
        );
        assert!(scan.family("Fira Code").is_none());
        assert!(FontScan::default().best().is_none());
    }

    #[test]
    fn broken_files_are_skipped() {
        let dir = tempfile::tempdir().expect("tempdir");
        std::fs::write(dir.path().join("short.ttf"), b"\0\x01\0\0\0\x09").expect("write");
        let mut bytes = font("X", None, &[u32::from('A')], &[0, 1]);
        bytes.truncate(60);
        std::fs::write(dir.path().join("cut.otf"), bytes).expect("write");
        let found = scan(&[dir.path().to_path_buf(), dir.path().join("missing")]);
        assert_eq!(found.faces, Vec::new());
    }
}

#[cfg(test)]
mod real {
    use super::*;

    /// Scans this machine's font folders and, with `TOKENRADAR_FONT_DIR`, a folder of extra
    /// fonts; prints the Nerd Fonts found and how long the scan took.
    #[test]
    #[ignore = "reads this machine's fonts"]
    fn scan_this_machine() {
        let env = |k: &str| std::env::var_os(k);
        let mut dirs = font_dirs(&env);
        dirs.extend(std::env::var_os("TOKENRADAR_FONT_DIR").map(PathBuf::from));
        let start = std::time::Instant::now();
        let scan = scan(&dirs);
        let took = start.elapsed();
        for f in scan.faces.iter().filter(|f| f.tier > NerdTier::None) {
            println!(
                "{:?} mono={} {} / {:?}",
                f.tier, f.mono, f.family, f.short_family
            );
        }
        println!("{} faces in {took:?}", scan.faces.len());
    }
}
