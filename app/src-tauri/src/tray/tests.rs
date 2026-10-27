//! The update throttle: nothing reaches the tray unless the user would see a change.

use tokenradar_core::domain::LimitKind;
use tokenradar_core::limits::Freshness;
use tokenradar_core::providers::claude::usage_api::ApiState;

use super::visual::tests::{limit, snapshot};
use super::*;

#[derive(Default)]
struct Recorder {
    icons: usize,
    tooltips: Vec<String>,
    menus: Vec<(Locale, bool)>,
    labels: Vec<bool>,
}

impl Sink for Recorder {
    fn set_icon(&mut self, _visual: TrayVisual, _env: Environment) {
        self.icons += 1;
    }
    fn set_tooltip(&mut self, text: &str) {
        self.tooltips.push(text.to_owned());
    }
    fn rebuild_menu(&mut self, l10n: &Localizer, muted: bool) {
        self.menus.push((l10n.locale(), muted));
    }
    fn set_mute_label(&mut self, _l10n: &Localizer, muted: bool) {
        self.labels.push(muted);
    }
}

const ENV: Environment = Environment {
    px: 16,
    theme: Theme::Dark,
};

fn driver() -> (Driver<Recorder>, Sender<TrayMsg>) {
    let (tx, rx) = mpsc::channel();
    let l10n = Localizer::new(Locale::En, &[Domain::Tray]);
    let driver = Driver {
        rx,
        model: TrayModel::default(),
        presenter: Presenter {
            icon: Some((TrayVisual::default(), ENV)),
            tooltip: Some(tooltip::tooltip(
                &TrayModel::default(),
                &l10n,
                &SystemClock,
                Timestamp::now(),
            )),
            menu: Some((Locale::En, false)),
        },
        l10n,
        env: ENV,
        store: None,
        sink: Recorder::default(),
    };
    (driver, tx)
}

fn snap(five_hour: f64) -> Box<PanelSnapshot> {
    Box::new(snapshot(
        vec![limit(LimitKind::Session, five_hour, Freshness::Fresh)],
        ApiState::Active,
    ))
}

const NOW: Timestamp = Timestamp::from_millis(1_793_145_600_000);

#[test]
fn tray_unchanged_frames_touch_nothing() {
    let (mut d, _tx) = driver();
    d.present(NOW);
    d.present(NOW);
    assert_eq!(d.sink.icons, 0, "the initial icon is already on screen");
    assert_eq!(d.sink.tooltips, Vec::<String>::new());
    assert_eq!(d.sink.menus, Vec::new());
    assert_eq!(d.sink.labels, Vec::<bool>::new());
}

#[test]
fn tray_sub_point_changes_do_not_redraw() {
    let (mut d, _tx) = driver();
    d.handle(TrayMsg::Snapshot(snap(42.0)));
    d.present(NOW);
    assert_eq!(d.sink.icons, 1);
    d.handle(TrayMsg::Snapshot(snap(42.3)));
    d.present(NOW);
    assert_eq!(d.sink.icons, 1, "42.3 % rounds to the same ring");
    assert_eq!(d.sink.tooltips.len(), 1, "and to the same tooltip");
    d.handle(TrayMsg::Snapshot(snap(43.0)));
    d.present(NOW);
    assert_eq!(d.sink.icons, 2);
}

#[test]
fn tray_environment_change_redraws_only_when_it_differs() {
    let (mut d, _tx) = driver();
    d.env = Environment {
        px: 24,
        theme: Theme::Dark,
    };
    d.present(NOW);
    d.present(NOW);
    assert_eq!(d.sink.icons, 1);
}

#[test]
fn tray_mute_changes_the_label_not_the_menu() {
    let (mut d, _tx) = driver();
    d.handle(TrayMsg::MutedUntil(Some(Timestamp::from_millis(
        NOW.as_millis() + MUTE_FOR_MS,
    ))));
    d.present(NOW);
    assert_eq!(d.sink.labels, vec![true]);
    assert_eq!(d.sink.menus, Vec::new());
    assert_eq!(d.sink.icons, 1, "pause mark");
    // The mute ends by itself.
    d.present(Timestamp::from_millis(NOW.as_millis() + MUTE_FOR_MS));
    assert_eq!(d.sink.labels, vec![true, false]);
}

#[test]
fn tray_locale_change_rebuilds_menu_and_tooltip() {
    let (mut d, _tx) = driver();
    d.handle(TrayMsg::Locale(Locale::Tr));
    d.present(NOW);
    assert_eq!(d.sink.menus, vec![(Locale::Tr, false)]);
    assert_eq!(
        d.sink.tooltips,
        vec!["TokenRadar\nHenüz limit verisi yok".to_owned()]
    );
    assert_eq!(d.sink.icons, 0, "the icon has no text");
}

#[test]
fn tray_toggle_mute_without_core_works_in_memory() {
    let (mut d, _tx) = driver();
    d.handle(TrayMsg::ToggleMute);
    assert!(d.model.is_muted(Timestamp::now()));
    d.handle(TrayMsg::ToggleMute);
    assert!(!d.model.is_muted(Timestamp::now()));
}

#[test]
fn tray_mute_survives_a_restart() {
    let dir = std::env::temp_dir().join(format!("tokenradar-tray-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("temp dir");
    let db = dir.join("mute.db");
    {
        let (mut d, _tx) = driver();
        d.handle(TrayMsg::CoreReady(Store::open(&db).expect("store")));
        d.handle(TrayMsg::ToggleMute);
        assert!(d.model.muted_until.is_some());
    }
    let (mut d, _tx) = driver();
    d.handle(TrayMsg::CoreReady(Store::open(&db).expect("store")));
    assert!(d.model.is_muted(Timestamp::now()));
    d.handle(TrayMsg::ToggleMute);
    drop(d);
    let store = Store::open(&db).expect("store");
    assert_eq!(read_mute(&store), None);
    drop(store);
    let _ = std::fs::remove_dir_all(&dir);
}
