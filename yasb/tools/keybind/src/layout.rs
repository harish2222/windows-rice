//! Geometry and layout calculation for the keybinds cheatsheet panel.

pub const W: i32 = 780;
pub const PAD: i32 = 16;
pub const SEARCH_H: i32 = 46;
pub const ROW_H: i32 = 40;
pub const GAP: i32 = 6;
pub const VISIBLE_ROWS: usize = 10;
pub const FOOT_H: i32 = 32;
pub const RADIUS: i32 = 18;

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
    (x + 10, y + 6, x + 210, y + ROW_H - 6)
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
        assert!(h <= 600);
        assert!(h >= 200);
    }
}
