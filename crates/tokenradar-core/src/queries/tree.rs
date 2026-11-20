//! The projects page's folder tree (Step 16b): project rows → a common-prefix tree with totals
//! per folder, depth first. A folder with one child and no project of its own is merged with
//! that child, so `D:\Projects\Tools\aktif` is one row instead of four that never branch.
//!
//! Folders are matched ignoring case (Windows compares paths so); a folder's label keeps the
//! case of the first path that named it. Unresolved projects (`unresolved:<dir>`) are leaves at
//! the top.

use std::collections::HashMap;

use super::projects::{ProjectNode, ProjectRow, ProjectSort, TREND_WEEKS};
use crate::domain::MicroUsd;
use crate::providers::claude::UNRESOLVED_PREFIX;

#[derive(Default)]
struct Folder {
    label: String,
    sep: char,
    project: Option<ProjectRow>,
    children: Vec<usize>,
    by_key: HashMap<String, usize>,
    // Totals of the folder and everything under it, filled bottom-up.
    projects: u32,
    sessions: u32,
    last_active: Option<crate::domain::Timestamp>,
    cost: i64,
    weekly: Option<f64>,
    trend: Vec<i64>,
}

/// The parts of a path and its separator: `C:\a\b` → `C:`, `a`, `b`; `\\srv\share\x` →
/// `\\srv`, `share`, `x`; `/home/a` → `/home`, `a`.
fn parts(path: &str) -> (Vec<String>, char) {
    if path.starts_with(UNRESOLVED_PREFIX) {
        return (vec![path.to_owned()], '\\');
    }
    // Normalized Windows paths use `\`; a bare drive (`C:`) too.
    let sep = if path.contains('\\') || !path.contains('/') {
        '\\'
    } else {
        '/'
    };
    let mut out: Vec<String> = path
        .split(sep)
        .filter(|p| !p.is_empty())
        .map(str::to_owned)
        .collect();
    let lead: String = path.chars().take_while(|&c| c == sep).collect();
    match out.first_mut() {
        Some(first) if !lead.is_empty() => first.insert_str(0, &lead),
        Some(_) => {}
        None => out.push(path.to_owned()),
    }
    (out, sep)
}

fn join(left: &str, right: &str, sep: char) -> String {
    if left.is_empty() {
        right.to_owned()
    } else if left.ends_with(sep) {
        format!("{left}{right}")
    } else {
        format!("{left}{sep}{right}")
    }
}

/// The tree of `rows`, each level sorted by `sort`, largest first.
pub(crate) fn build(rows: Vec<ProjectRow>, sort: ProjectSort) -> Vec<ProjectNode> {
    let mut arena = vec![Folder::default()];
    for row in rows {
        let (names, sep) = parts(&row.path);
        let mut at = 0;
        for name in names {
            let key = name.to_lowercase();
            at = if let Some(&next) = arena[at].by_key.get(&key) {
                next
            } else {
                let next = arena.len();
                arena.push(Folder {
                    label: name,
                    sep,
                    ..Folder::default()
                });
                arena[at].by_key.insert(key, next);
                arena[at].children.push(next);
                next
            };
        }
        // The last part shows the project's own name, which keeps its case.
        if let Some(last) = row.path.rsplit(['\\', '/']).find(|p| !p.is_empty())
            && last.eq_ignore_ascii_case(&row.name)
        {
            arena[at].label.clone_from(&row.name);
        }
        arena[at].project = Some(row);
    }
    total(&mut arena, 0);
    sort_children(&mut arena, sort);
    let mut out = Vec::new();
    let top = arena[0].children.clone();
    for child in top {
        emit(&arena, child, 0, "", &mut out);
    }
    out
}

/// Fills the totals of `at` and everything under it.
fn total(arena: &mut [Folder], at: usize) {
    let children = arena[at].children.clone();
    for &c in &children {
        total(arena, c);
    }
    let mut projects = 0;
    let mut sessions = 0;
    let mut last = None;
    let mut cost = 0;
    let mut weekly: Option<f64> = None;
    let mut trend = vec![0i64; TREND_WEEKS];
    let mut add =
        |p: u32, s: u32, l: Option<crate::domain::Timestamp>, c: i64, w: Option<f64>, t: &[i64]| {
            projects += p;
            sessions += s;
            last = last.max(l);
            cost += c;
            if let Some(w) = w {
                weekly = Some(weekly.unwrap_or(0.0) + w);
            }
            for (slot, v) in trend.iter_mut().zip(t) {
                *slot += v;
            }
        };
    if let Some(p) = &arena[at].project {
        let t: Vec<i64> = p.trend.iter().map(|m| m.0).collect();
        add(1, p.sessions, p.last_active, p.cost.0, p.weekly_percent, &t);
    }
    for &c in &children {
        let f = &arena[c];
        add(
            f.projects,
            f.sessions,
            f.last_active,
            f.cost,
            f.weekly,
            &f.trend,
        );
    }
    let f = &mut arena[at];
    f.projects = projects;
    f.sessions = sessions;
    f.last_active = last;
    f.cost = cost;
    f.weekly = weekly;
    f.trend = trend;
}

fn sort_children(arena: &mut [Folder], sort: ProjectSort) {
    // Largest first; a folder without a value sorts last.
    let desc = |a: &Folder, b: &Folder| -> std::cmp::Ordering {
        match sort {
            ProjectSort::LastActive => b.last_active.cmp(&a.last_active),
            ProjectSort::Cost => b.cost.cmp(&a.cost),
            ProjectSort::Impact => b
                .weekly
                .unwrap_or(-1.0)
                .total_cmp(&a.weekly.unwrap_or(-1.0)),
            ProjectSort::Sessions => b.sessions.cmp(&a.sessions),
        }
    };
    for i in 0..arena.len() {
        let mut children = std::mem::take(&mut arena[i].children);
        children.sort_by(|&a, &b| {
            desc(&arena[a], &arena[b]).then_with(|| arena[a].label.cmp(&arena[b].label))
        });
        arena[i].children = children;
    }
}

/// Writes `at` (merged with its single-child chain) and its subtree.
fn emit(arena: &[Folder], mut at: usize, depth: u32, parent: &str, out: &mut Vec<ProjectNode>) {
    let sep = arena[at].sep;
    let mut label = arena[at].label.clone();
    while arena[at].project.is_none() && arena[at].children.len() == 1 {
        at = arena[at].children[0];
        label = join(&label, &arena[at].label, sep);
    }
    let f = &arena[at];
    let path = join(parent, &label, sep);
    out.push(ProjectNode {
        depth,
        label,
        path: path.clone(),
        project: f.project.clone(),
        projects: f.projects,
        sessions: f.sessions,
        last_active: f.last_active,
        cost: MicroUsd(f.cost),
        weekly_percent: f.weekly,
        trend: f.trend.iter().map(|&v| MicroUsd(v)).collect(),
        children: u32::try_from(f.children.len()).unwrap_or(u32::MAX),
    });
    for &c in &f.children {
        emit(arena, c, depth + 1, &path, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProjectId, Timestamp};

    fn row(id: i64, path: &str, name: &str, cost: i64) -> ProjectRow {
        ProjectRow {
            id: ProjectId(id),
            name: name.into(),
            path: path.into(),
            sessions: 1,
            last_active: Some(Timestamp::from_millis(id)),
            cost: MicroUsd(cost),
            weekly_percent: (id % 2 == 0).then_some(1.5),
            trend: vec![MicroUsd(cost); TREND_WEEKS],
        }
    }

    fn shape(nodes: &[ProjectNode]) -> Vec<String> {
        nodes
            .iter()
            .map(|n| {
                format!(
                    "{}{}{} ={}",
                    "  ".repeat(n.depth as usize),
                    n.label,
                    if n.project.is_some() { " *" } else { "" },
                    n.cost.0
                )
            })
            .collect()
    }

    #[test]
    fn chains_merge_and_levels_sort_by_the_key() {
        let nodes = build(
            vec![
                row(1, r"D:\Projects\Tools\aktif\TokenRadar", "TokenRadar", 5),
                row(2, r"D:\Projects\Tools\aktif\Lucid", "Lucid", 7),
                row(3, r"D:\projects\Notes", "Notes", 1),
                row(4, r"C:\Users\ada", "ada", 2),
                row(5, r"C:\Users\ada\x\y", "y", 3),
            ],
            ProjectSort::Cost,
        );
        assert_eq!(
            shape(&nodes),
            [
                r"D:\Projects =13",
                r"  Tools\aktif =12",
                "    Lucid * =7",
                "    TokenRadar * =5",
                "  Notes * =1",
                r"C:\Users\ada * =5",
                r"  x\y * =3",
            ]
        );
        assert_eq!(nodes[0].projects, 3);
        assert_eq!(nodes[0].children, 2);
        assert_eq!(nodes[1].path, r"D:\Projects\Tools\aktif");
        assert_eq!(nodes[6].path, r"C:\Users\ada\x\y");
        assert_eq!(nodes[0].trend[0], MicroUsd(13));
        // Weekly impact adds up where it is known (even ids here); unknown stays unknown.
        assert_eq!(nodes[0].weekly_percent, Some(1.5));
        assert_eq!(nodes[3].weekly_percent, None);
        assert_eq!(nodes[5].weekly_percent, Some(1.5));
    }

    #[test]
    fn unc_posix_and_unresolved_paths() {
        let nodes = build(
            vec![
                row(1, r"\\srv\share\a", "a", 1),
                row(2, r"\\srv\share\b", "b", 2),
                row(3, "/home/ada/repo", "repo", 3),
                row(4, "unresolved:D--x", "D--x", 4),
            ],
            ProjectSort::Cost,
        );
        assert_eq!(
            shape(&nodes),
            [
                "unresolved:D--x * =4",
                "/home/ada/repo * =3",
                r"\\srv\share =3",
                "  b * =2",
                "  a * =1",
            ]
        );
        assert_eq!(nodes[3].path, r"\\srv\share\b");
    }

    #[test]
    fn last_active_sorts_newest_first() {
        let nodes = build(
            vec![row(1, r"C:\a", "a", 9), row(2, r"D:\b", "b", 1)],
            ProjectSort::LastActive,
        );
        assert_eq!(nodes[0].label, r"D:\b");
    }
}
