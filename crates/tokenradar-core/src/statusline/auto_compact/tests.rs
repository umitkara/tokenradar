use std::fs;

use super::*;

struct Dirs {
    _tmp: tempfile::TempDir,
    loc: Locations,
}

impl Dirs {
    fn new() -> Self {
        let tmp = tempfile::tempdir().expect("tempdir");
        let loc = Locations {
            managed_dir: Some(tmp.path().join("managed")),
            project_dir: Some(tmp.path().join("project")),
            config_dir: Some(tmp.path().join("config")),
        };
        Self { _tmp: tmp, loc }
    }

    fn write(&self, which: &str, json: &str) -> &Self {
        let path = match which {
            "user" => self
                .loc
                .config_dir
                .as_ref()
                .expect("dir")
                .join("settings.json"),
            "project" => self
                .loc
                .project_dir
                .as_ref()
                .expect("dir")
                .join(".claude/settings.json"),
            "local" => self
                .loc
                .project_dir
                .as_ref()
                .expect("dir")
                .join(".claude/settings.local.json"),
            "managed" => self
                .loc
                .managed_dir
                .as_ref()
                .expect("dir")
                .join("managed-settings.json"),
            drop_in => self
                .loc
                .managed_dir
                .as_ref()
                .expect("dir")
                .join("managed-settings.d")
                .join(drop_in),
        };
        fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        fs::write(path, json).expect("write");
        self
    }

    fn resolve(&self, env: &[(&str, &str)], model: Option<&str>) -> Option<AutoCompact> {
        let env: HashMap<String, String> = env
            .iter()
            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
            .collect();
        resolve(
            &self.loc.files(),
            |k| env.get(k).cloned(),
            model,
            Some(1_000_000),
        )
    }
}

#[test]
fn no_files_means_nothing_to_show() {
    let d = Dirs::new();
    assert_eq!(d.resolve(&[], None), None);
    // Enabled without a window: the default window is not guessed.
    d.write("user", r#"{"autoCompactEnabled": true}"#);
    assert_eq!(d.resolve(&[], None), None);
}

#[test]
fn files_follow_claude_codes_precedence() {
    let d = Dirs::new();
    d.write(
        "user",
        r#"{"autoCompactWindow": 200000, "env": {"SECRET": "x"}}"#,
    );
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(200_000)));
    d.write("project", r#"{"autoCompactWindow": 300000}"#);
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(300_000)));
    d.write("local", r#"{"autoCompactWindow": 400000}"#);
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(400_000)));
    d.write("managed", r#"{"autoCompactWindow": 500000}"#);
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(500_000)));
    // Drop-ins merge after the base file, alphabetically: the last one wins.
    d.write("10-a.json", r#"{"autoCompactWindow": 600000}"#)
        .write("20-b.json", r#"{"autoCompactWindow": 700000}"#)
        .write(".hidden.json", r#"{"autoCompactWindow": 800000}"#)
        .write("30-c.txt", r#"{"autoCompactWindow": 900000}"#);
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(700_000)));
}

#[test]
fn a_lower_file_fills_what_a_higher_one_leaves_out() {
    let d = Dirs::new();
    d.write("user", r#"{"autoCompactWindow": 250000}"#).write(
        "project",
        r#"{"autoCompactEnabled": true, "permissions": {}}"#,
    );
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(250_000)));
    d.write("local", r#"{"autoCompactEnabled": false}"#);
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Off));
}

#[test]
fn a_per_model_window_beats_the_files_top_level_one() {
    let d = Dirs::new();
    d.write(
        "user",
        r#"{"autoCompactWindow": 200000,
            "modelSettings": {"claude-opus-5-5": {"effortLevel": "high", "autoCompactWindow": 350000},
                              "claude-sonnet-5-5": {"autoCompactWindow": "auto"}}}"#,
    );
    let opus = Some(AutoCompact::Window(350_000));
    assert_eq!(d.resolve(&[], Some("claude-opus-5-5")), opus);
    assert_eq!(d.resolve(&[], Some("claude-opus-5-5[1m]")), opus);
    assert_eq!(d.resolve(&[], Some("claude-opus-5-5-20261001")), opus);
    // Another model, or none known: the top-level window.
    assert_eq!(
        d.resolve(&[], Some("claude-haiku-5-5")),
        Some(AutoCompact::Window(200_000))
    );
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(200_000)));
    // `"auto"` leaves the window to Claude Code: nothing to show.
    assert_eq!(d.resolve(&[], Some("claude-sonnet-5-5")), None);
    // But a higher file's top-level window still beats a lower file's per-model one.
    d.write("project", r#"{"autoCompactWindow": 450000}"#);
    assert_eq!(
        d.resolve(&[], Some("claude-opus-5-5")),
        Some(AutoCompact::Window(450_000))
    );
}

#[test]
fn environment_variables_override_the_files() {
    let d = Dirs::new();
    d.write(
        "managed",
        r#"{"autoCompactEnabled": true, "autoCompactWindow": 300000}"#,
    );
    for flag in ["DISABLE_AUTO_COMPACT", "DISABLE_COMPACT"] {
        assert_eq!(d.resolve(&[(flag, "1")], None), Some(AutoCompact::Off));
        assert_eq!(
            d.resolve(&[(flag, "0")], None),
            Some(AutoCompact::Window(300_000))
        );
    }
    let window = |v| d.resolve(&[("CLAUDE_CODE_AUTO_COMPACT_WINDOW", v)], None);
    assert_eq!(window("200000"), Some(AutoCompact::Window(200_000)));
    assert_eq!(window("250k"), Some(AutoCompact::Window(250_000)));
    assert_eq!(window("1M"), Some(AutoCompact::Window(1_000_000)));
    // Values Claude Code ignores fall back to the files.
    for ignored in ["5k", "2M", "lots", ""] {
        assert_eq!(
            window(ignored),
            Some(AutoCompact::Window(300_000)),
            "{ignored}"
        );
    }
}

#[test]
fn bad_files_and_values_are_skipped() {
    let d = Dirs::new();
    d.write("user", r#"{"autoCompactWindow": 200000}"#)
        .write("project", "{ not json")
        .write(
            "local",
            r#"{"autoCompactWindow": 50000, "autoCompactEnabled": "no"}"#,
        );
    // Out of range and wrong types are skipped; the user file still counts.
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(200_000)));
    // A BOM is tolerated.
    d.write("local", "\u{feff}{\"autoCompactWindow\": 600000}");
    assert_eq!(d.resolve(&[], None), Some(AutoCompact::Window(600_000)));
}

#[test]
fn the_window_is_capped_at_the_context_window() {
    let d = Dirs::new();
    d.write("user", r#"{"autoCompactWindow": 500000}"#);
    let files = d.loc.files();
    assert_eq!(
        resolve(&files, |_| None, None, Some(200_000)),
        Some(AutoCompact::Window(200_000))
    );
    assert_eq!(
        resolve(&files, |_| None, None, None),
        Some(AutoCompact::Window(500_000))
    );
}

#[test]
fn model_names_match_their_variants_only() {
    assert!(model_matches("claude-opus-5-5", "claude-opus-5-5"));
    assert!(model_matches("claude-opus-5-5", "Claude-Opus-5-5[1m]"));
    assert!(model_matches("claude-opus-5-5", "claude-opus-5-5@20261001"));
    assert!(!model_matches("claude-opus-5", "claude-opus-5-5"));
    assert!(!model_matches("claude-opus-5-5", "claude-opus-5-5-preview"));
    assert!(!model_matches("claude-opus-5-5", "claude-opus"));
}
