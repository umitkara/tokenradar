//! Where the floating widget goes: snapping to the work area's edges and corners after a drag,
//! and the position kept across monitors, resolutions and scales. Pure arithmetic in physical
//! pixels, like the panel's placement.

pub use crate::panel::place::Rect;

/// Within this many logical pixels of an edge, a dropped widget snaps to it.
pub const SNAP: f64 = 16.0;
/// A snapped widget keeps this far from the edge, like the panel keeps from the taskbar.
pub const MARGIN: f64 = 8.0;

/// The inset area a snapped widget lines up with.
fn inset(work: Rect, scale: f64) -> Rect {
    let m = px(MARGIN, scale);
    Rect::new(work.left + m, work.top + m, work.right - m, work.bottom - m)
}

#[allow(clippy::cast_possible_truncation)]
fn px(logical: f64, scale: f64) -> i32 {
    (logical * scale).round() as i32
}

/// The top-left corner after dropping the widget at `win`: within [`SNAP`] of an edge it moves
/// onto the edge's margin, two edges make a corner. It always ends up inside `work`.
pub fn snap(win: Rect, work: Rect, scale: f64) -> (i32, i32) {
    let area = inset(work, scale);
    let threshold = px(SNAP, scale);
    let (w, h) = (win.right - win.left, win.bottom - win.top);
    let axis = |lo: i32, hi: i32, start: i32, len: i32, outer_lo: i32, outer_hi: i32| {
        let to_lo = (start - lo).abs();
        let to_hi = (hi - (start + len)).abs();
        let snapped = if to_lo <= threshold && to_lo <= to_hi {
            lo
        } else if to_hi <= threshold {
            hi - len
        } else {
            start
        };
        clamp(snapped, outer_lo, outer_hi - len)
    };
    (
        axis(area.left, area.right, win.left, w, work.left, work.right),
        axis(area.top, area.bottom, win.top, h, work.top, work.bottom),
    )
}

/// `win`'s place as a share of the free space within the margins: `(0, 0)` is the top-left
/// corner, `(1, 1)` the bottom-right one.
pub fn relative(win: Rect, work: Rect, scale: f64) -> (f64, f64) {
    let area = inset(work, scale);
    let share = |start: i32, lo: i32, hi: i32, len: i32| {
        let free = hi - lo - len;
        if free <= 0 {
            return 0.0;
        }
        (f64::from(start - lo) / f64::from(free)).clamp(0.0, 1.0)
    };
    (
        share(win.left, area.left, area.right, win.right - win.left),
        share(win.top, area.top, area.bottom, win.bottom - win.top),
    )
}

/// The top-left corner for a widget of `size` at the share `rel` of `work`'s free space.
pub fn from_relative(rel: (f64, f64), size: (i32, i32), work: Rect, scale: f64) -> (i32, i32) {
    let area = inset(work, scale);
    #[allow(clippy::cast_possible_truncation)]
    let at = |share: f64, lo: i32, hi: i32, len: i32| {
        let free = (hi - lo - len).max(0);
        lo + (share.clamp(0.0, 1.0) * f64::from(free)).round() as i32
    };
    let (x, y) = (
        at(rel.0, area.left, area.right, size.0),
        at(rel.1, area.top, area.bottom, size.1),
    );
    (
        clamp(x, work.left, work.right - size.0),
        clamp(y, work.top, work.bottom - size.1),
    )
}

/// Without a remembered place: the bottom-right corner of the primary monitor's work area.
pub fn default_position(size: (i32, i32), work: Rect, scale: f64) -> (i32, i32) {
    from_relative((1.0, 1.0), size, work, scale)
}

/// `value` within `lo..=hi`; `lo` wins when the widget is larger than the area.
fn clamp(value: i32, lo: i32, hi: i32) -> i32 {
    value.min(hi).max(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 1920×1080 with a 48 px taskbar at the bottom.
    const WORK: Rect = Rect::new(0, 0, 1920, 1032);
    const W: i32 = 220;
    const H: i32 = 44;

    fn at(x: i32, y: i32) -> Rect {
        Rect::new(x, y, x + W, y + H)
    }

    #[test]
    fn near_an_edge_it_snaps_to_the_margin() {
        // 10 px from the left edge's margin, far from the others.
        assert_eq!(snap(at(18, 400), WORK, 1.0), (8, 400));
        // 16 px is still close enough; 17 px is not.
        assert_eq!(snap(at(1920 - 8 - W - 16, 400), WORK, 1.0).0, 1920 - 8 - W);
        assert_eq!(
            snap(at(1920 - 8 - W - 17, 400), WORK, 1.0).0,
            1920 - 8 - W - 17
        );
    }

    #[test]
    fn near_two_edges_it_snaps_to_the_corner() {
        let (x, y) = snap(at(1920 - W - 2, 1032 - H - 20), WORK, 1.0);
        assert_eq!((x, y), (1920 - 8 - W, 1032 - 8 - H));
    }

    #[test]
    fn the_threshold_and_margin_scale_with_the_monitor() {
        let work = Rect::new(0, 0, 2880, 1548);
        // 150 %: margin 12, threshold 24.
        assert_eq!(snap(Rect::new(30, 600, 360, 666), work, 1.5), (12, 600));
        assert_eq!(snap(Rect::new(37, 600, 367, 666), work, 1.5), (37, 600));
    }

    #[test]
    fn a_widget_dragged_off_screen_comes_back_inside() {
        assert_eq!(snap(at(-150, 1100), WORK, 1.0), (0, 1032 - H));
        assert_eq!(snap(at(1900, -30), WORK, 1.0), (1920 - W, 0));
    }

    #[test]
    fn the_place_survives_a_resolution_change() {
        let corner = at(1920 - 8 - W, 1032 - 8 - H);
        let rel = relative(corner, WORK, 1.0);
        assert_eq!(rel, (1.0, 1.0));
        let big = Rect::new(-2560, 0, 0, 1392);
        assert_eq!(from_relative(rel, (W, H), big, 1.0), (-8 - W, 1392 - 8 - H));
        let middle = relative(at(850, 494), WORK, 1.0);
        assert!((middle.0 - 0.5).abs() < 0.01 && (middle.1 - 0.5).abs() < 0.01);
    }

    #[test]
    fn the_default_is_the_bottom_right_corner() {
        assert_eq!(
            default_position((W, H), WORK, 1.0),
            (1920 - 8 - W, 1032 - 8 - H)
        );
    }

    #[test]
    fn a_widget_wider_than_the_area_starts_at_its_left() {
        let tiny = Rect::new(0, 0, 200, 300);
        assert_eq!(from_relative((1.0, 0.0), (W, H), tiny, 1.0).0, 0);
        assert_eq!(relative(at(0, 0), tiny, 1.0).0, 0.0);
    }
}
