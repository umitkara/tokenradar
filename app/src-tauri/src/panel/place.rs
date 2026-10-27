//! Where the panel goes: centered on the tray icon, against the taskbar, inside the work area
//! of the icon's monitor. Pure arithmetic in physical pixels, so every taskbar edge, scale and
//! monitor layout is a unit test.

/// A screen rectangle in physical pixels; `right` and `bottom` are exclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Self {
        Self {
            left,
            top,
            right,
            bottom,
        }
    }

    const fn center(self) -> (i32, i32) {
        (
            self.left + (self.right - self.left) / 2,
            self.top + (self.bottom - self.top) / 2,
        )
    }

    const fn intersects(self, other: Self) -> bool {
        self.left < other.right
            && other.left < self.right
            && self.top < other.bottom
            && other.top < self.bottom
    }
}

/// The screen edge the taskbar sits on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Bottom,
    Top,
    Left,
    Right,
}

/// The monitor the tray icon is on.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Screen {
    pub monitor: Rect,
    /// The monitor minus docked taskbars. Equal to `monitor` when the taskbar auto-hides.
    pub work: Rect,
    /// Effective DPI / 96.
    pub scale: f64,
}

/// The taskbar as `SHAppBarMessage(ABM_GETTASKBARPOS)` reports it (primary taskbar only).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Taskbar {
    pub edge: Edge,
    pub rect: Rect,
}

/// Distance from the taskbar and the work area's edges, in logical pixels: Windows 11 flyouts
/// keep 12 px.
pub const GAP: f64 = 12.0;

/// The panel's top-left corner for a panel of `size` physical pixels.
///
/// `anchor` is the tray icon (or, when it sits in the overflow flyout, that flyout's slot).
pub fn place(
    anchor: Rect,
    screen: &Screen,
    taskbar: Option<Taskbar>,
    size: (i32, i32),
) -> (i32, i32) {
    #[allow(clippy::cast_possible_truncation)]
    let gap = (GAP * screen.scale).round() as i32;
    let (area, edge) = usable_area(anchor, screen, taskbar);
    let (w, h) = size;
    let (cx, cy) = anchor.center();
    let (x, y) = match edge {
        Edge::Bottom => (cx - w / 2, area.bottom - gap - h),
        Edge::Top => (cx - w / 2, area.top + gap),
        Edge::Left => (area.left + gap, cy - h / 2),
        Edge::Right => (area.right - gap - w, cy - h / 2),
    };
    (
        clamp(x, area.left + gap, area.right - gap - w),
        clamp(y, area.top + gap, area.bottom - gap - h),
    )
}

/// The area the panel may cover and the edge it hangs from.
fn usable_area(anchor: Rect, screen: &Screen, taskbar: Option<Taskbar>) -> (Rect, Edge) {
    let mut area = screen.work;
    // An auto-hidden taskbar slides out over the work area: keep clear of it as well.
    if let Some(bar) = taskbar.filter(|t| t.rect.intersects(screen.monitor)) {
        match bar.edge {
            Edge::Bottom => area.bottom = area.bottom.min(bar.rect.top),
            Edge::Top => area.top = area.top.max(bar.rect.bottom),
            Edge::Left => area.left = area.left.max(bar.rect.right),
            Edge::Right => area.right = area.right.min(bar.rect.left),
        }
        return (area, bar.edge);
    }
    (
        area,
        inset_edge(screen).unwrap_or_else(|| nearest_edge(anchor, screen.monitor)),
    )
}

/// The side where the work area is inset from the monitor: a docked taskbar on this monitor.
fn inset_edge(screen: &Screen) -> Option<Edge> {
    let (m, w) = (screen.monitor, screen.work);
    [
        (m.bottom - w.bottom, Edge::Bottom),
        (w.top - m.top, Edge::Top),
        (w.left - m.left, Edge::Left),
        (m.right - w.right, Edge::Right),
    ]
    .into_iter()
    .filter(|(inset, _)| *inset > 0)
    .max_by_key(|(inset, _)| *inset)
    .map(|(_, edge)| edge)
}

/// No taskbar information at all: hang from the monitor edge closest to the icon.
fn nearest_edge(anchor: Rect, monitor: Rect) -> Edge {
    let (cx, cy) = anchor.center();
    [
        (monitor.bottom - cy, Edge::Bottom),
        (cy - monitor.top, Edge::Top),
        (cx - monitor.left, Edge::Left),
        (monitor.right - cx, Edge::Right),
    ]
    .into_iter()
    .min_by_key(|(distance, _)| *distance)
    .map_or(Edge::Bottom, |(_, edge)| edge)
}

/// `value` within `lo..=hi`; `lo` wins when the panel is larger than the area.
fn clamp(value: i32, lo: i32, hi: i32) -> i32 {
    value.min(hi).max(lo)
}

#[cfg(test)]
mod tests {
    use super::*;

    const W: i32 = 360;
    const H: i32 = 480;

    /// 1920×1080 at 100 % with a 48 px taskbar on `edge`.
    fn screen(edge: Edge) -> (Screen, Taskbar) {
        let monitor = Rect::new(0, 0, 1920, 1080);
        let (work, bar) = match edge {
            Edge::Bottom => (Rect::new(0, 0, 1920, 1032), Rect::new(0, 1032, 1920, 1080)),
            Edge::Top => (Rect::new(0, 48, 1920, 1080), Rect::new(0, 0, 1920, 48)),
            Edge::Left => (Rect::new(62, 0, 1920, 1080), Rect::new(0, 0, 62, 1080)),
            Edge::Right => (Rect::new(0, 0, 1858, 1080), Rect::new(1858, 0, 1920, 1080)),
        };
        (
            Screen {
                monitor,
                work,
                scale: 1.0,
            },
            Taskbar { edge, rect: bar },
        )
    }

    fn icon(x: i32, y: i32) -> Rect {
        Rect::new(x, y, x + 24, y + 32)
    }

    #[test]
    fn bottom_taskbar_centers_on_the_icon_above_the_taskbar() {
        let (s, bar) = screen(Edge::Bottom);
        let at = place(icon(1500, 1040), &s, Some(bar), (W, H));
        assert_eq!(at, (1512 - 180, 1032 - 12 - H));
    }

    #[test]
    fn an_icon_near_the_corner_keeps_the_panel_on_screen() {
        let (s, bar) = screen(Edge::Bottom);
        let at = place(icon(1880, 1040), &s, Some(bar), (W, H));
        assert_eq!(at, (1920 - 12 - W, 1032 - 12 - H));
    }

    #[test]
    fn top_taskbar_hangs_below_it() {
        let (s, bar) = screen(Edge::Top);
        let at = place(icon(1500, 8), &s, Some(bar), (W, H));
        assert_eq!(at, (1332, 48 + 12));
    }

    #[test]
    fn side_taskbars_center_vertically_on_the_icon() {
        let (s, bar) = screen(Edge::Left);
        assert_eq!(
            place(icon(20, 600), &s, Some(bar), (W, H)),
            (62 + 12, 616 - 240)
        );
        let (s, bar) = screen(Edge::Right);
        let at = place(icon(1880, 1000), &s, Some(bar), (W, H));
        // Near the bottom: clamped into the work area.
        assert_eq!(at, (1858 - 12 - W, 1080 - 12 - H));
    }

    #[test]
    fn an_auto_hidden_taskbar_is_avoided_although_the_work_area_covers_it() {
        let (mut s, bar) = screen(Edge::Bottom);
        s.work = s.monitor;
        let at = place(icon(1500, 1040), &s, Some(bar), (W, H));
        assert_eq!(at.1, 1032 - 12 - H);
    }

    #[test]
    fn the_overflow_flyout_anchor_still_hangs_from_the_taskbar() {
        let (s, bar) = screen(Edge::Bottom);
        // The icon in the "show hidden icons" flyout, well above the taskbar.
        let at = place(icon(1400, 900), &s, Some(bar), (W, H));
        assert_eq!(at, (1412 - 180, 1032 - 12 - H));
    }

    #[test]
    fn a_second_monitor_with_another_scale_uses_its_own_work_area() {
        // Primary taskbar on the left monitor; the icon is on a 150 % monitor to the right
        // (Windows 11 can show the tray on every taskbar).
        let s = Screen {
            monitor: Rect::new(1920, -200, 1920 + 3840, -200 + 2160),
            work: Rect::new(1920, -200, 1920 + 3840, -200 + 2088),
            scale: 1.5,
        };
        let bar = Taskbar {
            edge: Edge::Bottom,
            rect: Rect::new(0, 1032, 1920, 1080),
        };
        let size = (540, 720);
        let at = place(icon(5000, 1900), &s, Some(bar), size);
        assert_eq!(at, (5012 - 270, -200 + 2088 - 18 - 720));
    }

    #[test]
    fn without_any_taskbar_information_the_nearest_edge_wins() {
        let s = Screen {
            monitor: Rect::new(0, 0, 1920, 1080),
            work: Rect::new(0, 0, 1920, 1080),
            scale: 1.0,
        };
        assert_eq!(place(icon(1500, 1050), &s, None, (W, H)).1, 1080 - 12 - H);
        assert_eq!(place(icon(1500, 4), &s, None, (W, H)).1, 12);
    }

    #[test]
    fn a_panel_taller_than_the_area_starts_at_its_top() {
        let (s, bar) = screen(Edge::Bottom);
        assert_eq!(place(icon(1500, 1040), &s, Some(bar), (W, 1100)).1, 12);
    }
}
