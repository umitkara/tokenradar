use tokenradar_i18n::{Domain, Locale};

use super::*;

const MIN: i64 = 60_000;

fn line() -> Line {
    Line {
        five_hour: Some(Window {
            percent: 72.0,
            resets_in: Some(108 * MIN),
            eta: Some(38 * MIN),
            estimated: false,
        }),
        seven_day: Some(Window {
            percent: 34.0,
            resets_in: Some(3 * 24 * 60 * MIN),
            eta: None,
            estimated: false,
        }),
        session_share: Some(9.0),
    }
}

fn plain(locale: Locale, line: &Line, width: Option<usize>) -> String {
    let l = Localizer::new(locale, &[Domain::Cli]);
    render(
        &l,
        line,
        Style {
            color: false,
            width,
        },
    )
}

#[test]
fn full_line_in_english_and_turkish() {
    assert_eq!(
        plain(Locale::En, &line(), None),
        "◕ 5h 72% · 1h48m ▲38m │ 7d 34% · 3d │ ≈9% this session"
    );
    assert_eq!(
        plain(Locale::Tr, &line(), None),
        "◕ 5s %72 · 1sa48dk ▲38dk │ 7g %34 · 3g │ ≈%9 bu oturum"
    );
}

#[test]
fn a_narrow_terminal_drops_detail_from_the_end() {
    let full = plain(Locale::En, &line(), None);
    let width = full.chars().count();
    assert_eq!(plain(Locale::En, &line(), Some(width)), full);
    assert_eq!(
        plain(Locale::En, &line(), Some(width - 1)),
        "◕ 5h 72% · 1h48m ▲38m │ 7d 34% · 3d"
    );
    assert_eq!(plain(Locale::Tr, &line(), Some(20)), "◕ 5s %72 │ 7g %34");
    // Narrower than anything: the shortest form.
    assert_eq!(plain(Locale::Tr, &line(), Some(3)), "◕ 5s %72");
}

#[test]
fn estimates_are_marked_and_missing_data_is_said() {
    let mut l = line();
    l.five_hour = Some(Window {
        percent: 0.0,
        resets_in: None,
        eta: None,
        estimated: true,
    });
    l.session_share = None;
    assert_eq!(plain(Locale::En, &l, None), "○ ≈5h 0% │ 7d 34% · 3d");
    assert_eq!(
        plain(Locale::En, &Line::default(), None),
        "no limit data yet"
    );
}

#[test]
fn colors_follow_the_thresholds() {
    let l10n = Localizer::new(Locale::En, &[Domain::Cli]);
    let mut l = line();
    l.five_hour.as_mut().expect("5h").percent = 93.0;
    let out = render(
        &l10n,
        &l,
        Style {
            color: true,
            width: None,
        },
    );
    assert!(out.starts_with("\x1b[31m●\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[31m5h 93%\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[36m7d 34%\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[33m▲38m\x1b[0m"), "{out:?}");
    assert!(out.contains("\x1b[2m │ \x1b[0m"), "{out:?}");
}

#[test]
fn the_measurement_wins_and_the_app_adds_projection_and_share() {
    use tokenradar_core::ipc::{SessionInfo, StatusReply};

    let now = Timestamp::from_millis(1_793_059_200_000);
    let payload = StatuslinePayload::parse(
        br#"{"session_id":"s","rate_limits":{"five_hour":{"used_percentage":50,"resets_at":1793066400}}}"#,
    )
    .expect("payload");
    let mut reply = StatusReply::empty(now);
    let app_limit = |kind: &str, percent: f64| LimitInfo {
        kind: kind.into(),
        scope: None,
        percent,
        resets_at: Some(Timestamp::from_millis(now.as_millis() + 3 * 3_600_000)),
        source: "api".into(),
        estimated: false,
        observed_at: now,
        freshness: "fresh".into(),
        eta_100_at: Some(Timestamp::from_millis(now.as_millis() + 30 * MIN)),
        will_hit_before_reset: true,
        projection_confidence: Some("medium".into()),
    };
    reply.limits = vec![app_limit("session", 40.0), app_limit("weekly_all", 20.0)];
    reply.session = Some(SessionInfo {
        session_uuid: Some("s".into()),
        name: None,
        title: None,
        project: None,
        status: None,
        share_percent: Some(12.5),
        cost_usd: 0.0,
        tokens_per_min: 0.0,
    });
    let line = Line::from_sources(Some(&payload), Some(&reply), now);
    let five = line.five_hour.expect("5h");
    assert!(
        (five.percent - 50.0).abs() < 1e-9,
        "the payload's measurement"
    );
    assert_eq!(five.resets_in, Some(2 * 3_600_000));
    assert_eq!(five.eta, Some(30 * MIN));
    let seven = line.seven_day.expect("7d from the app");
    assert!((seven.percent - 20.0).abs() < 1e-9);
    assert_eq!(line.session_share, Some(12.5));

    // Offline: the payload alone; a passed reset is an estimated 0 %.
    let later = Timestamp::from_millis(1_793_066_400_000 + 1);
    let line = Line::from_sources(Some(&payload), None, later);
    assert_eq!(
        line.five_hour,
        Some(Window {
            percent: 0.0,
            resets_in: None,
            eta: None,
            estimated: true
        })
    );
    assert_eq!(line.seven_day, None);
}
