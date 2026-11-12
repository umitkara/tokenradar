//! Step 12s: the payload's rounded-down percentage meets the app's fused value (ADR-060).

use super::*;

/// A payload whose weekly window is at `percent`, resetting three hours after `NOW` (the
/// reset of [`app_limit`]).
fn weekly_payload(percent: u32) -> StatuslinePayload {
    let json = format!(
        r#"{{"session_id":"s","rate_limits":{{"seven_day":{{"used_percentage":{percent},"resets_at":1794010800}}}}}}"#
    );
    StatuslinePayload::parse(json.as_bytes()).expect("payload")
}

fn seven_day(payload_percent: u32, app: LimitInfo) -> f64 {
    let mut reply = StatusReply::empty(now());
    reply.limits = vec![app];
    let line = Line::from_sources(Some(&weekly_payload(payload_percent)), Some(&reply), now());
    let w = line.seven_day.expect("7d");
    assert_eq!(w.source, Source::Statusline, "the payload still leads");
    w.percent
}

#[test]
fn the_app_value_one_above_the_payload_is_shown() {
    assert_eq!(
        seven_day(5, app_limit("weekly_all", None, 6.0, "statusline")),
        6.0
    );
}

#[test]
fn an_app_value_that_does_not_fit_the_payload_is_ignored() {
    // Equal or lower: the payload's own number.
    assert_eq!(seven_day(5, app_limit("weekly_all", None, 5.0, "api")), 5.0);
    assert_eq!(seven_day(5, app_limit("weekly_all", None, 4.0, "api")), 5.0);
    // Two points above cannot be the same reading: `[5, 6)` rounds to 6 at most.
    assert_eq!(seven_day(5, app_limit("weekly_all", None, 7.0, "api")), 5.0);
    // An estimate never lifts a measurement.
    assert_eq!(
        seven_day(5, app_limit("weekly_all", None, 6.0, "estimate")),
        5.0
    );
}

#[test]
fn another_window_does_not_lift_the_payload() {
    let mut app = app_limit("weekly_all", None, 6.0, "api");
    app.resets_at = Some(Timestamp::from_millis(NOW + 3 * 3_600_000 + WEEK_MS));
    assert_eq!(seven_day(5, app), 5.0);
}
