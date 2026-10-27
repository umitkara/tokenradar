use std::cell::RefCell;

use super::*;

const MS: Duration = Duration::from_millis(1);

#[test]
fn a_burst_inside_one_window_is_one_batch() {
    let t0 = Instant::now();
    let mut d = Debouncer::new(DEBOUNCE);
    assert_eq!(d.deadline(), None);
    for i in 0..100u32 {
        // 100 writes to two files over 500 ms.
        let file = if i % 2 == 0 { "a.jsonl" } else { "b.jsonl" };
        d.add(PathBuf::from(file), t0 + MS * (i * 5));
    }
    assert_eq!(d.deadline(), Some(t0 + DEBOUNCE));
    assert_eq!(d.take_due(t0 + MS * 749), None);
    let batch = d.take_due(t0 + DEBOUNCE).expect("due");
    assert_eq!(batch.len(), 2);
    assert_eq!(d.deadline(), None);
    assert_eq!(d.take_due(t0 + DEBOUNCE * 2), None, "nothing pending");
}

#[test]
fn continuous_writes_still_flush_every_window() {
    // A sliding window would starve here; the fixed window flushes on schedule.
    let t0 = Instant::now();
    let mut d = Debouncer::new(DEBOUNCE);
    let mut batches = 0;
    for i in 0..30u32 {
        let now = t0 + MS * (i * 100);
        d.add(PathBuf::from("live.jsonl"), now);
        if d.take_due(now).is_some() {
            batches += 1;
        }
    }
    // 3 s of writes every 100 ms: batches at 0.8, 1.7 and 2.6 s.
    assert_eq!(batches, 3);
}

#[test]
fn hot_files_report_changes_and_expire() {
    let t0 = Instant::now();
    let stamp = |size| Some(Stamp { size, mtime: None });
    let disk: RefCell<HashMap<PathBuf, u64>> = RefCell::new(HashMap::new());
    let stat = |p: &Path| disk.borrow().get(p).map(|s| stamp(*s).expect("some"));

    let mut hot = HotFiles::new(HOT_TTL);
    disk.borrow_mut().insert("a".into(), 10);
    disk.borrow_mut().insert("b".into(), 20);
    hot.touch("a".into(), stamp(10), t0);
    hot.touch("b".into(), stamp(20), t0);
    assert_eq!(hot.changed(t0 + HOT_INTERVAL, stat), Vec::<PathBuf>::new());

    // An append the watcher missed.
    disk.borrow_mut().insert("a".into(), 15);
    assert_eq!(
        hot.changed(t0 + HOT_INTERVAL * 2, stat),
        [PathBuf::from("a")]
    );
    assert_eq!(
        hot.changed(t0 + HOT_INTERVAL * 3, stat),
        Vec::<PathBuf>::new()
    );

    // A deleted file is reported once and dropped.
    disk.borrow_mut().remove(Path::new("b"));
    assert_eq!(
        hot.changed(t0 + HOT_INTERVAL * 4, stat),
        [PathBuf::from("b")]
    );
    assert_eq!(hot.len(), 1);

    // `a` last changed at 2 × interval; it goes cold one TTL later.
    assert_eq!(
        hot.changed(t0 + HOT_INTERVAL * 2 + HOT_TTL, stat),
        Vec::<PathBuf>::new()
    );
    assert!(hot.is_empty());
}

#[test]
fn fs_watcher_reports_writes_under_a_watched_directory() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let nested = tmp.path().join("p").join("s");
    std::fs::create_dir_all(&nested).expect("mkdir");
    let (tx, rx) = mpsc::channel();
    let mut w = FsWatcher::new(tx, |s| s).expect("watcher");
    assert!(w.ensure(tmp.path(), true));
    assert!(w.ensure(tmp.path(), true), "idempotent");
    assert!(!w.ensure(&tmp.path().join("missing"), false));

    let file = nested.join("x.jsonl");
    std::fs::write(&file, b"{}\n").expect("write");
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left).expect("an event within 10 s") {
            FsSignal::Path(p) if p == file => break,
            _ => {}
        }
    }
}
