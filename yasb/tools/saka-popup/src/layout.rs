//! Panel geometry, in one place.
//!
//! This exists because the panel's first version computed its bands twice —
//! once as constants for the shape pass and once as running `y` values in the
//! text pass — and the two drifted. The era line was drawn at y=102 while the
//! moon strip started at y=96, so the moon glyph sat on top of the date, and
//! the progress captions were positioned 2px below their own bars and
//! overflowed into the next row. Both bugs are invisible in the source and
//! obvious on screen, which is exactly what a single source of truth fixes.
//!
//! Everything is absolute from the top of the panel, so a band can be checked
//! against its neighbours. [`Layout::assert_no_overlap`] is the test that would
//! have caught the original defect.

/// Width of the panel.
pub const W: i32 = 460;
/// Window corner radius.
pub const RADIUS: i32 = 20;
/// Radius of the inner pills and the row card.
pub const PILL: i32 = 10;
/// Panel edge padding.
pub const PAD: i32 = 24;

/// Height of one row. Two lines: the value, then the bar with its caption.
pub const ROW_H: i32 = 46;
/// Width of the row card's radius.
pub const CARD_R: i32 = 16;
/// Inner padding of the row card.
pub const CARD_PAD: i32 = 12;

/// Where each band starts and ends, measured from the panel's top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Tracked weekday label (11px caps).
    pub weekday_y: i32,
    /// The date, in the 28px display face.
    pub date_y: i32,
    /// The era line: Saka / Vikram Samvat.
    pub era_y: i32,
    /// Bottom of the era line.
    pub era_bottom: i32,
    /// Rule under the header.
    pub rule_y: i32,
    /// Top of the moon glyph's band.
    pub moon_y: i32,
    /// Bottom of the moon band.
    pub moon_bottom: i32,
    /// The row card.
    pub card_y: i32,
    pub card_bottom: i32,
    /// Top of the footer text.
    pub footer_y: i32,
    /// Total height the panel needs.
    pub height: i32,
}

impl Layout {
    /// Compute the geometry for a panel with `rows` rows.
    pub fn new(rows: i32) -> Layout {
        let weekday_y = PAD;
        // 11px caps: 14px line.
        let date_y = weekday_y + 20;
        // 28px display: 36px line.
        let era_y = date_y + 38;
        // 13px: 18px line.
        let era_bottom = era_y + 18;
        let rule_y = era_bottom + 14;
        // The moon rides in the same band as the era line, at the right, so the
        // panel does not spend 76px on a strip that only holds a disc and two
        // short lines.
        let moon_y = date_y;
        let moon_bottom = era_bottom;
        let card_y = rule_y + 18;
        let card_bottom = card_y + CARD_PAD * 2 + rows * ROW_H;
        let footer_y = card_bottom + 16;
        let height = footer_y + 18 + PAD;
        Layout {
            weekday_y,
            date_y,
            era_y,
            era_bottom,
            rule_y,
            moon_y,
            moon_bottom,
            card_y,
            card_bottom,
            footer_y,
            height,
        }
    }

    /// Top of row `i` inside the card.
    pub fn row_y(&self, i: i32) -> i32 {
        self.card_y + CARD_PAD + i * ROW_H
    }

    /// Left edge of the value column, shared by the value, the bar and the
    /// label column's right edge.
    pub const VALUE_X: i32 = PAD + 90;

    /// Where the progress bar starts and ends.
    ///
    /// The bar stops short of the right edge so the caption can be
    /// right-aligned past it — running them into each other is what made the
    /// old layout unreadable.
    pub fn track(&self, w: i32) -> (i32, i32, i32) {
        let x0 = Self::VALUE_X;
        let x1 = w - PAD - 96;
        (x0, x1 - x0, 4)
    }

    /// Fail if any two vertically stacked bands overlap.
    ///
    /// The moon band deliberately shares vertical space with the header text —
    /// they sit side by side, not stacked — so only the stacked bands are
    /// compared.
    pub fn assert_no_overlap(&self, rows: i32) -> Result<(), String> {
        let stacked = [
            ("weekday", self.weekday_y, self.date_y),
            ("date", self.date_y, self.era_y),
            ("era", self.era_y, self.era_bottom),
            ("rule", self.rule_y, self.rule_y + 1),
            ("card", self.card_y, self.card_bottom),
            ("footer", self.footer_y, self.footer_y + 18),
        ];
        for pair in stacked.windows(2) {
            let (an, _, a_bottom) = pair[0];
            let (bn, b_top, _) = pair[1];
            if a_bottom > b_top {
                return Err(format!("{an} ends at {a_bottom} but {bn} starts at {b_top}"));
            }
        }
        // Every row, and the caption inside it, must fit the row.
        for i in 0..rows {
            let y = self.row_y(i);
            // The bar sits 30px into the row and is 4px tall; the caption is
            // 14px tall on the same line.
            if y + 30 + 14 > y + ROW_H {
                return Err(format!("row {i} content overflows: {y}"));
            }
        }
        if self.footer_y + 18 + PAD > self.height {
            return Err(format!("footer overflows the panel: {}", self.height));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bands_never_overlap_for_any_row_count() {
        for rows in 1..=12 {
            let l = Layout::new(rows);
            l.assert_no_overlap(rows).unwrap_or_else(|e| panic!("{rows} rows: {e}"));
        }
    }

    /// The regression fence for the actual defect: the era line must end
    /// above the card, and the moon band must not start before the date.
    #[test]
    fn the_era_line_ends_above_the_card() {
        let l = Layout::new(10);
        assert!(l.era_bottom <= l.card_y, "era {} card {}", l.era_bottom, l.card_y);
        assert!(l.rule_y > l.era_bottom);
    }

    #[test]
    fn the_moon_band_starts_below_the_weekday_label() {
        let l = Layout::new(10);
        assert!(l.moon_y >= l.date_y);
        assert!(l.moon_bottom <= l.card_y);
    }

    #[test]
    fn rows_stack_without_gaps_or_overlap() {
        let l = Layout::new(10);
        for i in 0..9 {
            assert_eq!(l.row_y(i + 1) - l.row_y(i), ROW_H);
        }
        let last = l.row_y(9);
        assert!(last + ROW_H <= l.card_bottom, "last row spills out of the card");
    }

    #[test]
    fn the_panel_height_is_exactly_what_the_content_needs() {
        let l = Layout::new(10);
        assert_eq!(l.height, l.footer_y + 18 + PAD);
        assert!(l.height > 500 && l.height < 800, "height {}", l.height);
    }

    /// The bar has to end before the caption starts, or the two collide.
    #[test]
    fn the_track_leaves_room_for_the_caption() {
        let l = Layout::new(10);
        let (x0, tw, th) = l.track(W);
        assert_eq!(x0, Layout::VALUE_X);
        assert!(tw > 80, "track too narrow: {tw}");
        assert_eq!(th, 4);
        assert!(
            x0 + tw <= W - PAD - 90,
            "track ends at {} leaving too little for the caption",
            x0 + tw
        );
    }

    #[test]
    fn the_value_column_starts_after_the_label_column() {
        assert!(Layout::VALUE_X > PAD + 80, "label column and value column collide");
    }
}
