//! Where the widget goes. Floating: snapping to the work area's edges and corners after a drag,
//! and the position kept across monitors, resolutions and scales. Docked (Step 18b): on the
//! primary taskbar, next to the notification area or at the left end. Pure arithmetic in
//! physical pixels, like the panel's placement.

use tokenradar_core::config::{MAX_TASKBAR_OFFSET, TaskbarAlign};

pub use crate::panel::place::{Edge, Rect};

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

/// A docked widget keeps this far from the notification area and the taskbar's left end.
pub const DOCK_GAP: f64 = 8.0;
/// From this taskbar height (logical pixels) on, the docked widget has room for two rows.
pub const TWO_ROWS_FROM: f64 = 40.0;

/// The primary taskbar as docking sees it (ADR-058): only the `Shell_TrayWnd` and
/// `TrayNotifyWnd` rectangles, never the buttons inside (Windows 11 draws those with XAML).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Taskbar {
    pub edge: Edge,
    pub rect: Rect,
    /// The notification area (tray icons and clock).
    pub notify: Rect,
    pub auto_hide: bool,
    /// The taskbar's effective DPI / 96.
    pub scale: f64,
}

/// Why the widget cannot dock and floats instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Undockable {
    /// No taskbar window (Explorer is not running or restarting).
    NoTaskbar,
    /// Left, right or top: Windows 11 only has a bottom taskbar; the strip is horizontal.
    Edge,
    /// The widget would stay on screen while the taskbar hides.
    AutoHide,
    /// Not enough free space left of the notification area.
    NoRoom,
}

impl Taskbar {
    /// The taskbar's height in logical pixels.
    pub fn logical_height(&self) -> f64 {
        f64::from(self.rect.bottom - self.rect.top) / self.scale
    }

    /// Two rows when the taskbar is at least [`TWO_ROWS_FROM`] logical pixels tall.
    pub fn two_rows(&self) -> bool {
        self.logical_height() + 0.01 >= TWO_ROWS_FROM
    }

    /// The span a docked widget may use: between the left end and the notification area,
    /// [`DOCK_GAP`] away from both.
    fn span(&self) -> (i32, i32) {
        let gap = px(DOCK_GAP, self.scale);
        (self.rect.left + gap, self.notify.left - gap)
    }

    fn check(&self, width: i32) -> Result<(i32, i32), Undockable> {
        if self.edge != Edge::Bottom {
            return Err(Undockable::Edge);
        }
        if self.auto_hide {
            return Err(Undockable::AutoHide);
        }
        let (lo, hi) = self.span();
        if hi - lo < width {
            return Err(Undockable::NoRoom);
        }
        Ok((lo, hi))
    }

    fn middle(&self, height: i32) -> i32 {
        self.rect.top + (self.rect.bottom - self.rect.top - height) / 2
    }
}

/// The top-left corner of a docked widget of `size`: `offset` logical pixels away from the
/// anchor, kept inside the free span and centered on the taskbar's height.
pub fn dock(
    taskbar: &Taskbar,
    size: (i32, i32),
    align: TaskbarAlign,
    offset: u16,
) -> Result<(i32, i32), Undockable> {
    let (lo, hi) = taskbar.check(size.0)?;
    let off = px(f64::from(offset), taskbar.scale);
    let x = match align {
        TaskbarAlign::Tray => hi - size.0 - off,
        TaskbarAlign::Left => lo + off,
    };
    Ok((clamp(x, lo, hi - size.0), taskbar.middle(size.1)))
}

/// While a docked widget is dragged: only sideways, and never off the free span.
pub fn drag_docked(taskbar: &Taskbar, x: i32, size: (i32, i32)) -> (i32, i32) {
    let (lo, hi) = taskbar.span();
    (clamp(x, lo, hi - size.0), taskbar.middle(size.1))
}

/// The offset (logical pixels from `align`'s anchor) that puts a docked widget at `x`.
pub fn dock_offset(taskbar: &Taskbar, x: i32, width: i32, align: TaskbarAlign) -> u16 {
    let (lo, hi) = taskbar.span();
    let x = clamp(x, lo, hi - width);
    let physical = match align {
        TaskbarAlign::Tray => hi - width - x,
        TaskbarAlign::Left => x - lo,
    };
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let logical = (f64::from(physical.max(0)) / taskbar.scale).round() as u32;
    u16::try_from(logical.min(u32::from(MAX_TASKBAR_OFFSET))).unwrap_or(MAX_TASKBAR_OFFSET)
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

    /// 1920×1200 at 125 %: a 60 px (48 logical) taskbar, the notification area from x = 1574.
    fn taskbar() -> Taskbar {
        Taskbar {
            edge: Edge::Bottom,
            rect: Rect::new(0, 1140, 1920, 1200),
            notify: Rect::new(1574, 1140, 1920, 1200),
            auto_hide: false,
            scale: 1.25,
        }
    }

    #[test]
    fn docked_it_sits_left_of_the_notification_area() {
        // 275×55: the two-row strip at 125 %. Gap 8 logical = 10 px.
        let at = dock(&taskbar(), (275, 55), TaskbarAlign::Tray, 0);
        assert_eq!(at, Ok((1574 - 10 - 275, 1140 + 2)));
        // An offset of 40 logical px moves it 50 px further left.
        let at = dock(&taskbar(), (275, 55), TaskbarAlign::Tray, 40);
        assert_eq!(at.map(|p| p.0), Ok(1574 - 10 - 275 - 50));
        // At the left end, the offset moves it to the right.
        let at = dock(&taskbar(), (275, 55), TaskbarAlign::Left, 40);
        assert_eq!(at.map(|p| p.0), Ok(10 + 50));
    }

    #[test]
    fn docking_follows_the_scale() {
        // 100 %, 48 px taskbar: gap 8 px, two rows (48 logical).
        let tb = Taskbar {
            rect: Rect::new(0, 1032, 1920, 1080),
            notify: Rect::new(1650, 1032, 1920, 1080),
            scale: 1.0,
            ..taskbar()
        };
        assert!(tb.two_rows());
        assert_eq!(
            dock(&tb, (220, 44), TaskbarAlign::Tray, 0),
            Ok((1650 - 8 - 220, 1032 + 2))
        );
        // 150 %, 72 px: gap 12 px.
        let tb = Taskbar {
            rect: Rect::new(0, 2088, 3840, 2160),
            notify: Rect::new(3400, 2088, 3840, 2160),
            scale: 1.5,
            ..taskbar()
        };
        assert!(tb.two_rows());
        assert_eq!(
            dock(&tb, (330, 66), TaskbarAlign::Tray, 0),
            Ok((3400 - 12 - 330, 2088 + 3))
        );
    }

    #[test]
    fn a_small_taskbar_gets_one_row() {
        // Windows 10 small buttons: 30 px at 100 %.
        let tb = Taskbar {
            rect: Rect::new(0, 1050, 1920, 1080),
            scale: 1.0,
            ..taskbar()
        };
        assert!(!tb.two_rows());
        // 40 logical px exactly is enough for two.
        let tb = Taskbar {
            rect: Rect::new(0, 1150, 1920, 1200),
            ..taskbar()
        };
        assert!(tb.two_rows());
    }

    #[test]
    fn unsupported_layouts_float() {
        for edge in [Edge::Left, Edge::Top, Edge::Right] {
            let tb = Taskbar { edge, ..taskbar() };
            assert_eq!(
                dock(&tb, (275, 55), TaskbarAlign::Tray, 0),
                Err(Undockable::Edge)
            );
        }
        let hiding = Taskbar {
            auto_hide: true,
            ..taskbar()
        };
        assert_eq!(
            dock(&hiding, (275, 55), TaskbarAlign::Tray, 0),
            Err(Undockable::AutoHide)
        );
    }

    #[test]
    fn a_widget_wider_than_the_free_space_floats() {
        // 300 px between the left end and the notification area, minus two 10 px gaps.
        let narrow = Taskbar {
            notify: Rect::new(300, 1140, 1920, 1200),
            ..taskbar()
        };
        assert_eq!(
            dock(&narrow, (281, 55), TaskbarAlign::Tray, 0),
            Err(Undockable::NoRoom)
        );
        assert_eq!(
            dock(&narrow, (280, 55), TaskbarAlign::Tray, 0),
            Ok((10, 1142))
        );
    }

    #[test]
    fn a_large_offset_stops_at_the_far_end() {
        assert_eq!(
            dock(&taskbar(), (275, 55), TaskbarAlign::Tray, 4000).map(|p| p.0),
            Ok(10)
        );
        assert_eq!(
            dock(&taskbar(), (275, 55), TaskbarAlign::Left, 4000).map(|p| p.0),
            Ok(1574 - 10 - 275)
        );
    }

    #[test]
    fn a_docked_drag_moves_sideways_and_sets_the_offset() {
        let tb = taskbar();
        // Pulled too far right: it stays on the free span, centered on the taskbar.
        assert_eq!(drag_docked(&tb, 1500, (275, 55)), (1574 - 10 - 275, 1142));
        assert_eq!(drag_docked(&tb, -40, (275, 55)), (10, 1142));
        // Dropped 50 px left of the anchor: 40 logical px.
        let x = 1574 - 10 - 275 - 50;
        assert_eq!(dock_offset(&tb, x, 275, TaskbarAlign::Tray), 40);
        assert_eq!(dock_offset(&tb, x, 275, TaskbarAlign::Left), 983);
        // The round trip lands where it was dropped.
        let off = dock_offset(&tb, 700, 275, TaskbarAlign::Left);
        assert_eq!(
            dock(&tb, (275, 55), TaskbarAlign::Left, off).map(|p| p.0),
            Ok(700)
        );
    }

    #[test]
    fn a_widget_wider_than_the_area_starts_at_its_left() {
        let tiny = Rect::new(0, 0, 200, 300);
        assert_eq!(from_relative((1.0, 0.0), (W, H), tiny, 1.0).0, 0);
        assert_eq!(relative(at(0, 0), tiny, 1.0).0, 0.0);
    }
}
