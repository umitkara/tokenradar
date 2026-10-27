use super::*;
use crate::notifier::QuietHours;

fn datetime(s: &str) -> toml::value::Datetime {
    s.parse().expect("datetime")
}

#[test]
fn missing_file_means_defaults_and_no_consent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let cfg = AppConfig::load(&dir.path().join("config.toml")).expect("load");
    assert_eq!(cfg, AppConfig::default());
    assert!(!cfg.api_consent_granted(Path::new("C:/Users/Ada/.claude")));
    assert_eq!(cfg.notifications, NotifyPrefs::default());
    assert_eq!(cfg.data.raw_retention_days, DEFAULT_RAW_RETENTION_DAYS);
    assert_eq!(cfg.general.panel_warm_ttl_s(), DEFAULT_PANEL_WARM_TTL_S);
}

#[test]
fn schema_1_consent_still_counts_and_ignores_unknown_fields() {
    let cfg = AppConfig::parse(
        "schema_version = 1\nfuture_setting = \"x\"\n\n[api_consent]\ngranted = true\n\
         granted_at = 2026-10-27T10:00:00Z\nversion = 1\n",
    )
    .expect("parse");
    assert_eq!(cfg.schema_version, 1);
    assert!(cfg.legacy_api_consent.is_valid());
    assert!(cfg.api_consent_granted(Path::new("C:/any/.claude")));
}

#[test]
fn an_accounts_own_consent_wins_over_the_legacy_one() {
    let cfg = AppConfig::parse(
        "[api_consent]\ngranted = true\nversion = 1\n\n\
         [[accounts]]\nprovider = \"claude\"\nconfig_dir = 'C:/a/.claude'\n\
         [accounts.api_consent]\ngranted = false\n\n\
         [[accounts]]\nprovider = \"claude\"\nconfig_dir = 'C:/b/.claude'\n",
    )
    .expect("parse");
    assert!(!cfg.api_consent_granted(Path::new("C:/a/.claude")));
    // No entry of its own: the schema 1 consent applies.
    assert!(cfg.api_consent_granted(Path::new("C:/b/.claude")));
}

#[test]
fn consent_for_an_older_text_does_not_count() {
    let cfg = AppConfig::parse("[api_consent]\ngranted = true\nversion = 0\n").expect("parse");
    assert!(!cfg.legacy_api_consent.is_valid());
    let cfg = AppConfig::parse("[api_consent]\ngranted = false\nversion = 1\n").expect("parse");
    assert!(!cfg.legacy_api_consent.is_valid());
}

#[test]
fn a_malformed_section_falls_back_to_its_defaults_only() {
    let cfg = AppConfig::parse(
        "[notifications]\nfive_hour_thresholds = \"lots\"\n\n[data]\nraw_retention_days = 30\n\n\
         [[accounts]]\nprovider = \"claude\"\nconfig_dir = 'C:/a/.claude'\n\
         [accounts.api_consent]\ngranted = true\nconsent_version = 1\n",
    )
    .expect("parse");
    assert_eq!(cfg.notifications, NotifyPrefs::default());
    assert_eq!(cfg.data.raw_retention_days, 30);
    assert!(cfg.api_consent_granted(Path::new("C:/a/.claude")));
}

#[test]
fn unknown_preferences_mean_system() {
    let cfg = AppConfig::parse("[general]\nlocale = \"de\"\ntheme = \"sepia\"\n").expect("parse");
    assert_eq!(cfg.general.locale_pref(), LocalePref::System);
    assert_eq!(cfg.general.theme_pref(), ThemePref::System);
}

#[test]
fn panel_warm_ttl_is_clamped() {
    for (raw, want) in [(-5, 0), (0, 0), (30, 30), (100_000, MAX_PANEL_WARM_TTL_S)] {
        let cfg = AppConfig::parse(&format!("[general]\npanel_warm_ttl = {raw}\n")).expect("parse");
        assert_eq!(cfg.general.panel_warm_ttl_s(), want, "{raw}");
    }
}

#[test]
fn statusline_state_round_trips_and_keeps_unknown_content() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "# my comment\nschema_version = 2\nfuture = \"x\"\n\n[general]\nlocale = \"tr\"\n",
    )
    .expect("seed");
    let claude = dir.path().join(".claude");
    let state = StatuslineConfig {
        installed: true,
        previous: Some(r#"{"type": "command", "command": "py 's.py'"}"#.into()),
        previous_command: Some("py 's.py'".into()),
        chain: true,
        chain_position: ChainPosition::Before,
    };
    save_statusline(&path, &claude, &state).expect("save");
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.starts_with("# my comment\n"), "{text}");
    assert!(text.contains("future = \"x\""));
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(cfg.general.locale(), Locale::Tr);
    assert_eq!(cfg.account(&claude).map(|a| &a.statusline), Some(&state));

    // A second save updates the same entry instead of adding one.
    let cleared = StatuslineConfig::default();
    save_statusline(&path, &claude, &cleared).expect("save again");
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(cfg.accounts.len(), 1);
    assert_eq!(cfg.account(&claude).map(|a| &a.statusline), Some(&cleared));
}

#[test]
fn general_keys_are_saved_without_touching_the_rest() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("sub").join("config.toml");
    save_general(&path, "theme", ThemePref::Dark.as_str()).expect("save new");
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(i64::from(cfg.schema_version), SCHEMA_VERSION);
    assert_eq!(cfg.general.theme_pref(), ThemePref::Dark);
    assert_eq!(cfg.general.locale_pref(), LocalePref::System);

    std::fs::write(
        &path,
        "# keep\nschema_version = 2\n[general]\nlocale = \"tr\"\nfuture = 1\n",
    )
    .expect("seed");
    save_general(&path, "locale", LocalePref::En.as_str()).expect("save");
    save_panel_warm_ttl(&path, 5000).expect("save ttl");
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.starts_with("# keep\n"), "{text}");
    assert!(text.contains("future = 1"), "{text}");
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(cfg.general.locale_pref(), LocalePref::En);
    assert_eq!(cfg.general.panel_warm_ttl_s(), MAX_PANEL_WARM_TTL_S);
}

#[test]
fn sections_are_saved_normalized_and_keep_unknown_keys() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(
        &path,
        "schema_version = 2\n\n[notifications]\n# why\nsound = \"chime\"\n\n\
         [notifications.quiet_hours]\nweekdays_only = true\n",
    )
    .expect("seed");
    let prefs = NotifyPrefs {
        five_hour_thresholds: vec![90, 50, 50],
        quiet_hours: QuietHours {
            enabled: true,
            start: "22:30".into(),
            end: "7:00".into(),
        },
        ..NotifyPrefs::default()
    };
    save_notifications(&path, &prefs).expect("save");
    save_data(
        &path,
        &DataConfig {
            raw_retention_days: 1,
            update_check: false,
        },
    )
    .expect("save data");
    save_advanced(
        &path,
        &AdvancedConfig {
            log_level: LogLevel::Debug,
        },
    )
    .expect("save advanced");

    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.contains("# why\nsound = \"chime\""), "{text}");
    assert!(text.contains("weekdays_only = true"), "{text}");
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(cfg.notifications.five_hour_thresholds, [50, 90]);
    assert_eq!(cfg.notifications.quiet_hours.end, "07:00");
    assert!(cfg.notifications.quiet_hours.enabled);
    assert_eq!(cfg.notifications, prefs.normalized());
    assert_eq!(cfg.data.raw_retention_days, MIN_RAW_RETENTION_DAYS);
    assert!(!cfg.data.update_check);
    assert_eq!(cfg.advanced.log_level, LogLevel::Debug);
}

#[test]
fn consent_is_granted_and_withdrawn_per_account() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let a = Path::new("C:/a/.claude");
    let at = datetime("2026-10-27T10:00:00Z");
    save_api_consent(&path, a, true, at).expect("grant");
    let cfg = AppConfig::load(&path).expect("load");
    let consent = cfg.account(a).and_then(|c| c.api_consent.clone());
    assert_eq!(
        consent,
        Some(ApiConsent {
            granted: true,
            granted_at: Some(at),
            consent_version: API_CONSENT_VERSION,
        })
    );
    assert!(cfg.api_consent_granted(a));

    save_api_consent(&path, a, false, at).expect("withdraw");
    let cfg = AppConfig::load(&path).expect("load");
    assert!(!cfg.api_consent_granted(a));
    assert_eq!(cfg.accounts.len(), 1);
}

#[test]
fn migration_moves_the_consent_into_the_primary_account() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    let primary = Path::new("C:/Users/Ada/.claude");
    std::fs::write(
        &path,
        "# mine\nschema_version = 1\n\n[general]\nlocale = \"tr\"\n\n\
         [api_consent]\ngranted = true\ngranted_at = 2026-10-27T10:00:00Z\nversion = 1\n\n\
         [[accounts]]\nprovider = \"claude\"\nconfig_dir = 'C:/Users/Ada/.claude'\n\n\
         [accounts.statusline]\ninstalled = true\nchain = false\nchain_position = \"before\"\n",
    )
    .expect("seed");
    assert!(migrate(&path, Some(primary)).expect("migrate"));
    let text = std::fs::read_to_string(&path).expect("read");
    assert!(text.starts_with("# mine\n"), "{text}");
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(i64::from(cfg.schema_version), SCHEMA_VERSION);
    assert_eq!(cfg.legacy_api_consent, ApiConsent::default());
    let account = cfg.account(primary).expect("account");
    assert!(account.statusline.installed);
    assert_eq!(
        account.api_consent,
        Some(ApiConsent {
            granted: true,
            granted_at: Some(datetime("2026-10-27T10:00:00Z")),
            consent_version: 1,
        })
    );
    assert!(cfg.api_consent_granted(primary));

    // Already current: nothing is written.
    let before = std::fs::metadata(&path).and_then(|m| m.modified()).ok();
    assert!(!migrate(&path, Some(primary)).expect("again"));
    assert_eq!(
        std::fs::metadata(&path).and_then(|m| m.modified()).ok(),
        before
    );
}

#[test]
fn migration_without_a_primary_account_keeps_the_legacy_consent() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "[api_consent]\ngranted = true\nversion = 1\n").expect("seed");
    assert!(migrate(&path, None).expect("migrate"));
    let cfg = AppConfig::load(&path).expect("load");
    assert_eq!(i64::from(cfg.schema_version), SCHEMA_VERSION);
    assert!(cfg.api_consent_granted(Path::new("C:/x/.claude")));
}

#[test]
fn migration_leaves_missing_and_newer_files_alone() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join("config.toml");
    assert!(!migrate(&path, None).expect("missing"));
    assert!(!path.exists());
    std::fs::write(&path, "schema_version = 9\n[api_consent]\ngranted = true\n").expect("seed");
    assert!(!migrate(&path, Some(Path::new("C:/a"))).expect("newer"));
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "schema_version = 9\n[api_consent]\ngranted = true\n"
    );
}
