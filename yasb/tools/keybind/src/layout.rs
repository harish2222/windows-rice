//! Geometry and layout calculation for the keybinds cheatsheet panel.

pub const W: i32 = 820;
pub const PAD: i32 = 18;
pub const SEARCH_H: i32 = 48;
pub const ROW_H: i32 = 42;
pub const GAP: i32 = 6;
pub const VISIBLE_ROWS: usize = 11;
pub const FOOT_H: i32 = 34;
pub const RADIUS: i32 = 18;
pub const SCROLLBAR_W: i32 = 5;

pub fn total_height(row_count: usize) -> i32 {
    let rows = row_count.min(VISIBLE_ROWS).max(1) as i32;
    PAD + SEARCH_H + 12 + (ROW_H + GAP) * rows + FOOT_H + PAD
}

pub fn search_box(w: i32, pad: i32, search_h: i32) -> (i32, i32, i32, i32) {
    (pad, pad, w - pad, pad + search_h)
}

pub fn row_y(origin_y: i32, visual_idx: usize) -> i32 {
    origin_y + visual_idx as i32 * (ROW_H + GAP)
}

pub fn key_badge_rect(x: i32, y: i32) -> (i32, i32, i32, i32) {
    (x + 10, y + 6, x + 236, y + ROW_H - 6)
}

pub fn scrollbar_rects(
    w: i32,
    origin_y: i32,
    rows_h: i32,
    total: usize,
    visible: usize,
    scroll: usize,
) -> Option<((i32, i32, i32, i32), (i32, i32, i32, i32))> {
    if total <= visible || total == 0 {
        return None;
    }
    let track_x0 = w - PAD + 3;
    let track_x1 = track_x0 + SCROLLBAR_W;
    let track_y0 = origin_y;
    let track_y1 = origin_y + rows_h;

    let track_h = (track_y1 - track_y0).max(20);
    let thumb_h = ((visible as f32 / total as f32) * track_h as f32).max(30.0) as i32;
    let max_scroll = (total - visible) as f32;
    let thumb_travel = track_h - thumb_h;
    let thumb_y0 = track_y0 + ((scroll as f32 / max_scroll) * thumb_travel as f32) as i32;
    let thumb_y1 = thumb_y0 + thumb_h;

    Some(((track_x0, track_y0, track_x1, track_y1), (track_x0, thumb_y0, track_x1, thumb_y1)))
}

/// Computes the starting visible index to ensure the selected row is visible.
pub fn calculate_scroll(sel: usize, cur_scroll: usize, total: usize, visible: usize) -> usize {
    if total <= visible {
        return 0;
    }
    let mut s = cur_scroll;
    if sel < s {
        s = sel;
    } else if sel >= s + visible {
        s = sel + 1 - visible;
    }
    s.min(total.saturating_sub(visible))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scroll_keeps_selection_visible() {
        assert_eq!(calculate_scroll(0, 0, 20, 10), 0);
        assert_eq!(calculate_scroll(9, 0, 20, 10), 0);
        assert_eq!(calculate_scroll(10, 0, 20, 10), 1);
        assert_eq!(calculate_scroll(15, 0, 20, 10), 6);
        assert_eq!(calculate_scroll(4, 6, 20, 10), 4);
    }

    #[test]
    fn total_height_is_bounded() {
        let h = total_height(20);
        assert!(h <= 700);
        assert!(h >= 200);
    }

    #[test]
    fn scrollbar_rects_work() {
        assert!(scrollbar_rects(820, 76, 500, 10, 11, 0).is_none());
        let res = scrollbar_rects(820, 76, 500, 50, 11, 0);
        assert!(res.is_some());
        let (track, thumb) = res.unwrap();
        assert_eq!(track.0, thumb.0);
        assert!(thumb.3 > thumb.1);
    }
}
