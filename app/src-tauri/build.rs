use std::path::{Path, PathBuf};

/// Marks a placeholder resource; a release build refuses to bundle a file that starts with it.
const STUB: &[u8] = b"TokenRadar placeholder resource";

fn main() {
    // tauri.conf.json bundles the size-optimized CLI and the third-party notices, and tauri_build
    // checks that every resource exists, also for `cargo check/clippy/test`. Both files are only
    // produced by beforeBuildCommand, so a fresh clone (and CI's first `pnpm check:all`) has
    // neither. Debug builds get placeholders; release builds, which `tauri build` makes after
    // beforeBuildCommand, must find the real files, so a placeholder can never ship.
    let target = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../target");
    let resources: [(PathBuf, &[u8]); 2] = [
        (target.join("release-cli/tokenradar.exe"), b"MZ"),
        (target.join("third-party/THIRD-PARTY-NOTICES.txt"), b""),
    ];
    let release = std::env::var("PROFILE").as_deref() == Ok("release");
    for (path, magic) in &resources {
        println!("cargo:rerun-if-changed={}", path.display());
        let head = std::fs::read(path)
            .ok()
            .map(|bytes| bytes[..bytes.len().min(64)].to_vec());
        match head {
            Some(head) if release && (head.starts_with(STUB) || !head.starts_with(magic)) => {
                panic!(
                    "{} is a placeholder; build with `pnpm tauri build`",
                    path.display()
                )
            }
            Some(_) => {}
            None if release => panic!(
                "{} is missing; build with `pnpm tauri build`",
                path.display()
            ),
            None => {
                std::fs::create_dir_all(path.parent().expect("has a parent"))
                    .expect("create the resource directory");
                let mut stub = STUB.to_vec();
                stub.extend_from_slice(b"; `pnpm tauri build` writes the real file.\n");
                std::fs::write(path, stub).expect("write the resource placeholder");
            }
        }
    }
    tauri_build::build();
}
