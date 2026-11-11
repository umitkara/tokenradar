use std::sync::atomic::{AtomicU32, Ordering};

use super::*;

/// `--porcelain=v2 --branch -z` records joined with NUL. File names are made up.
fn out(records: &[&str]) -> Vec<u8> {
    let mut v = Vec::new();
    for r in records {
        v.extend_from_slice(r.as_bytes());
        v.push(0);
    }
    v
}

const OID: &str = "3f2a9c1d0e8b7a6f5e4d3c2b1a0f9e8d7c6b5a49";

#[test]
fn counts_staged_modified_untracked_and_the_upstream_distance() {
    let s = parse(&out(&[
        &format!("# branch.oid {OID}"),
        "# branch.head main",
        "# branch.upstream origin/main",
        "# branch.ab +1 -2",
        &format!("1 M. N... 100644 100644 100644 {OID} {OID} src/a.rs"),
        &format!("1 .M N... 100644 100644 100644 {OID} {OID} src/b.rs"),
        &format!("1 MM N... 100644 100644 100644 {OID} {OID} src/c.rs"),
        &format!("1 A. N... 000000 100644 100644 {OID} {OID} new.rs"),
        &format!("1 .D N... 100644 100644 000000 {OID} {OID} gone.rs"),
        "? notes.txt",
        "? tmp/",
        "! target/",
    ]));
    assert_eq!(
        s,
        GitStatus {
            staged: 3,
            modified: 3,
            untracked: 2,
            conflicted: 0,
            ahead: Some(1),
            behind: Some(2),
            untracked_skipped: false,
        }
    );
    assert!(s.is_dirty() && !s.is_clean());
}

#[test]
fn a_rename_skips_its_original_path() {
    // The original path is a field of its own; read as a record it would count again. Name it
    // like a record to be sure it is skipped.
    let s = parse(&out(&[
        "# branch.head main",
        &format!("2 R. N... 100644 100644 100644 {OID} {OID} R100 new name.rs"),
        "? looks-untracked.rs",
        &format!("2 RM N... 100644 100644 100644 {OID} {OID} C75 copy.rs"),
        "1 M. this is the original path",
    ]));
    assert_eq!((s.staged, s.modified, s.untracked), (2, 1, 0));
}

#[test]
fn conflicts_detached_heads_no_upstream_and_submodules() {
    let s = parse(&out(&[
        &format!("# branch.oid {OID}"),
        "# branch.head (detached)",
        &format!("u UU N... 100644 100644 100644 100644 {OID} {OID} {OID} both.rs"),
        &format!("u AA N... 000000 100644 100644 100644 {OID} {OID} {OID} added.rs"),
        // A submodule with new commits and a dirty tree: an ordinary change.
        &format!("1 .M SCM. 160000 160000 160000 {OID} {OID} vendor/lib"),
    ]));
    assert_eq!(s.conflicted, 2);
    assert_eq!(s.modified, 1);
    assert_eq!(
        (s.ahead, s.behind),
        (None, None),
        "no upstream, no distance"
    );
    let initial = parse(&out(&["# branch.oid (initial)", "# branch.head main"]));
    assert!(initial.is_clean());
}

#[test]
fn garbage_and_unknown_records_count_nothing() {
    for bad in [
        &b""[..],
        b"\0\0",
        b"# branch.ab +x -y\0",
        b"z what\0",
        b"1\0",
        b"\xff\xfe\0",
    ] {
        let s = parse(bad);
        assert_eq!(
            (s.staged, s.modified, s.untracked, s.conflicted),
            (0, 0, 0, 0)
        );
    }
    assert_eq!(parse(b"# branch.ab +3 -0\0").ahead, Some(3));
}

#[test]
fn the_run_never_takes_the_optional_index_lock() {
    for untracked in [true, false] {
        let a = args(untracked);
        assert_eq!(a[0], "--no-optional-locks", "before the subcommand");
        assert!(a.contains(&"--porcelain=v2") && a.contains(&"-z"));
        assert_eq!(a.contains(&"-uno"), !untracked);
    }
}

fn repo() -> tempfile::TempDir {
    let tmp = tempfile::tempdir().expect("tmp");
    std::fs::create_dir_all(tmp.path().join(".git")).expect("mkdir");
    std::fs::write(tmp.path().join(".git/HEAD"), "ref: refs/heads/main\n").expect("head");
    std::fs::write(tmp.path().join(".git/index"), "x").expect("index");
    tmp
}

/// Waits for the background run to land.
fn settled(cache: &GitStatusCache, cwd: &Path, untracked: bool) -> Option<GitStatus> {
    for _ in 0..200 {
        let running = cache.lock().entries.iter().any(|e| e.running);
        if !running {
            return cache.get(cwd, untracked);
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    panic!("the run did not finish");
}

#[test]
fn the_first_query_starts_a_run_and_the_next_one_shows_it() {
    let tmp = repo();
    let runs = Arc::new(AtomicU32::new(0));
    let counter = runs.clone();
    let cache = GitStatusCache::new(Arc::new(move |_: &Path, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Ok(GitStatus {
            modified: 3,
            ..GitStatus::default()
        })
    }));
    let deep = tmp.path().join("src/a");
    std::fs::create_dir_all(&deep).expect("mkdir");
    assert_eq!(cache.get(&deep, true), None, "cold: hidden this round");
    let s = settled(&cache, &deep, true).expect("warm");
    assert_eq!(s.modified, 3);
    // Within the TTL nothing runs again, from any folder of the tree.
    assert_eq!(cache.get(tmp.path(), true), Some(s));
    assert_eq!(runs.load(Ordering::SeqCst), 1);
    // Outside a repository: nothing, no run.
    let plain = tempfile::tempdir().expect("tmp");
    let _ = cache.get(&plain.path().join("no/such"), true);
}

#[test]
fn an_old_result_is_served_while_a_new_one_runs_but_not_after_a_commit() {
    let tmp = repo();
    let n = Arc::new(AtomicU32::new(0));
    let counter = n.clone();
    let cache = GitStatusCache::new(Arc::new(move |_: &Path, _| {
        let i = counter.fetch_add(1, Ordering::SeqCst);
        Ok(GitStatus {
            staged: i,
            ..GitStatus::default()
        })
    }))
    .with_ttl(Duration::ZERO);
    let _ = cache.get(tmp.path(), true);
    let first = settled(&cache, tmp.path(), true).expect("first");
    // Past the TTL: the old value is still shown and a new run starts.
    let served = cache.get(tmp.path(), true);
    assert!(served.is_some(), "no flicker between two runs");
    let _ = settled(&cache, tmp.path(), true);
    assert!(n.load(Ordering::SeqCst) >= 2);
    // The index changed (a commit, an add): the old counts are wrong, so hidden.
    std::thread::sleep(Duration::from_millis(20));
    std::fs::write(tmp.path().join(".git/index"), "changed").expect("index");
    let t = SystemTime::now() + Duration::from_secs(2);
    let f = std::fs::File::options()
        .write(true)
        .open(tmp.path().join(".git/index"))
        .expect("open");
    f.set_modified(t).expect("mtime");
    drop(f);
    assert_eq!(cache.get(tmp.path(), true), None);
    assert_ne!(settled(&cache, tmp.path(), true), Some(first));
}

#[test]
fn two_timeouts_turn_the_untracked_scan_off() {
    let tmp = repo();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let seen = calls.clone();
    let cache = GitStatusCache::new(Arc::new(move |_: &Path, untracked| {
        seen.lock().expect("lock").push(untracked);
        if untracked {
            Err(RunError::TimedOut)
        } else {
            Ok(GitStatus {
                modified: 1,
                untracked: 9,
                ..GitStatus::default()
            })
        }
    }));
    assert_eq!(cache.get(tmp.path(), true), None);
    assert_eq!(settled(&cache, tmp.path(), true), None, "first timeout");
    assert_eq!(settled(&cache, tmp.path(), true), None, "second timeout");
    let s = settled(&cache, tmp.path(), true).expect("then -uno");
    assert!(cache.untracked_off(tmp.path()));
    assert!(s.untracked_skipped);
    assert_eq!(s.untracked, 0, "unknown, never a made-up count");
    assert_eq!(*calls.lock().expect("lock"), vec![true, true, false]);
}

#[test]
fn without_git_nothing_is_tried_again() {
    let tmp = repo();
    let runs = Arc::new(AtomicU32::new(0));
    let counter = runs.clone();
    let cache = GitStatusCache::new(Arc::new(move |_: &Path, _| {
        counter.fetch_add(1, Ordering::SeqCst);
        Err(RunError::Missing)
    }));
    assert_eq!(cache.get(tmp.path(), true), None);
    assert_eq!(settled(&cache, tmp.path(), true), None);
    assert_eq!(cache.get(tmp.path(), true), None);
    assert!(cache.git_missing());
    assert_eq!(runs.load(Ordering::SeqCst), 1);
}

#[test]
fn at_most_eight_repositories_are_kept() {
    let repos: Vec<_> = (0..10).map(|_| repo()).collect();
    let cache = GitStatusCache::new(Arc::new(|_: &Path, _| Ok(GitStatus::default())));
    for r in &repos {
        let _ = cache.get(r.path(), true);
        let _ = settled(&cache, r.path(), true);
    }
    assert_eq!(cache.lock().entries.len(), MAX_REPOS);
    let oldest = cache
        .lock()
        .entries
        .iter()
        .any(|e| e.root == repos[0].path());
    assert!(!oldest, "the least recently asked went first");
}

/// The real git, when there is one: this repository's own tree reads without error.
#[test]
fn the_real_git_reads_a_real_tree() {
    if !git_available() {
        return;
    }
    let tmp = tempfile::tempdir().expect("tmp");
    let init = Command::new("git")
        .args(["init", "-q"])
        .current_dir(tmp.path())
        .status()
        .expect("git init");
    assert!(init.success());
    std::fs::write(tmp.path().join("a.txt"), "x").expect("write");
    let s = run_git(tmp.path(), true).expect("status");
    assert_eq!(s.untracked, 1);
    assert_eq!((s.staged, s.modified, s.ahead), (0, 0, None));
    let s = run_git(tmp.path(), false).expect("status -uno");
    assert_eq!(s.untracked, 0);
    assert!(!tmp.path().join(".git/index.lock").exists());
}
