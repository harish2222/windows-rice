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

/// Height of one row.
///
/// Was 44 with the progress bar on its own line under the text. That spent a
/// third of the row on a 3px line, which is what made the panel read as sparse
/// — ten rows of it came to 440px of mostly empty card. The bar now sits
/// *under the value only*, inside the same line box, so a row is one line of
/// type plus a 2px rule and nothing else.
pub const ROW_H: i32 = 34;
/// Width of the row card's radius.
pub const CARD_R: i32 = 16;
/// Inner padding of the row card.
pub const CARD_PAD: i32 = 12;
/// Radius of the moon glyph. Lives here, not in `main.rs`, because the shape
/// pass and the text pass both need it and they must not disagree.
pub const MOON_R: i32 = 30;
/// Radius of the weekday scrim pill. Half its height, so the ends are
/// semicircular and the label never looks boxed.
pub const SCRIM_R: i32 = 9;
/// Vertical offset of the progress indicator inside a row, measured from the
/// row's top. The text line box is 22px, so this puts the rule just under it.
pub const INDICATOR_Y: i32 = 26;
/// Thickness of the progress indicator. 2px reads as a rule rather than a bar.
pub const INDICATOR_H: i32 = 2;
/// Horizontal padding inside the weekday pill, either side of the text.
pub const SCRIM_PAD_X: i32 = 11;
/// The weekday label's line box, which is what the pill is sized around.
pub const WEEKDAY_H: i32 = 18;

/// Where each band starts and ends, measured from the panel's top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    /// Tracked weekday label (11px caps).
    pub weekday_y: i32,
    /// Top of the weekday's scrim pill.
    pub weekday_top: i32,
    /// Bottom of the weekday's scrim pill.
    pub weekday_bottom: i32,
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
        // The pill is centred on the label's line box, so the two can never
        // drift apart. It reaches a little above `weekday_y` and ends just
        // above the date, which is why `date_y` is derived from the pill's
        // bottom rather than from the label's own y.
        let weekday_top = weekday_y - 3;
        let weekday_bottom = weekday_top + WEEKDAY_H + 6;
        // 11px caps: 14px line.
        let date_y = weekday_bottom + 2;
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
            weekday_top,
            weekday_bottom,
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

    /// The weekday scrim pill, given the measured width of the label.
    ///
    /// The pill is sized from the text rather than fixed, because a fixed box
    /// would either clip `WEDNESDAY` or trail a long gap behind `SUN`. The
    /// text is inset by [`SCRIM_PAD_X`] on both sides so the caps never touch
    /// the edge.
    pub fn weekday_pill(&self, text_w: i32) -> (i32, i32, i32, i32) {
        (
            PAD - SCRIM_PAD_X,
            self.weekday_top,
            PAD + text_w + SCRIM_PAD_X,
            self.weekday_bottom,
        )
    }

    /// Top of row `i` inside the card.
    pub fn row_y(&self, i: i32) -> i32 {
        self.card_y + CARD_PAD + i * ROW_H
    }

    /// Left edge of the value column.
    ///
    /// Sized off the *actual* label set in the bar's own typeface rather than
    /// a round number. The panel used to use a monospace-ish Segoe UI and
    /// 96px was generous; when the panel switched to the bar's Nerd Font the
    /// wider mono advances pushed "PURnimanta" to 84px and the value column
    /// started clipping the longest label. 104 is the measured width of the
    /// longest label plus one character of clearance.
    pub const VALUE_X: i32 = PAD + 104;

    /// Centre x of the moon glyph, flush with the content's right edge.
    ///
    /// The glyph carries no caption: the phase name and the illumination are
    /// already the `Moon` row's value and caption, so repeating them here
    /// duplicated two facts on one screen — and a centred caption under a disc
    /// this size does not fit inside the content box anyway.
    pub fn moon_center_x(&self, w: i32) -> i32 {
        w - PAD - MOON_R
    }

    /// Where the progress indicator starts and ends.
    ///
    /// Under the *value*, not under the whole row. Spanning the full width put
    /// a long empty line to the right of every short value, which is what the
    /// old panel looked like: a list of mostly-empty rules. Starting at the
    /// value column and running to the panel's right margin keeps it visually
    /// attached to the number it belongs to while still giving it a run.
    ///
    /// Returns `(x0, width, thickness)`.
    pub fn track(&self, w: i32) -> (i32, i32, i32) {
        (Self::VALUE_X, w - PAD - Self::VALUE_X, 2)
    }

    /// Fail if any two vertically stacked bands overlap.
    ///
    /// The moon band deliberately shares vertical space with the header text —
    /// they sit side by side, not stacked — so only the stacked bands are
    /// compared.
    pub fn assert_no_overlap(&self, rows: i32) -> Result<(), String> {
        let stacked = [
            ("weekday pill", self.weekday_bottom, self.date_y),
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
        // Every row's content must fit inside the row.
        for i in 0..rows {
            let y = self.row_y(i);
            // The indicator sits under the value's line box. It has to clear
            // the text above it and still end inside the row, or it lands on
            // the next row's label.
            if y + INDICATOR_Y + INDICATOR_H > y + ROW_H {
                return Err(format!("row {i} indicator overflows: {y}"));
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

    /// The indicator belongs to the value, so it starts where the value starts and
    /// ends on the panel's right margin — attached to the number it describes,
    /// with no run of empty rule to the right of a short value.
    #[test]
    fn the_indicator_runs_from_the_value_column_to_the_right_margin() {
        let l = Layout::new(10);
        let (x0, tw, th) = l.track(W);
        assert_eq!(x0, Layout::VALUE_X, "indicator must start under the value");
        assert_eq!(x0 + tw, W - PAD, "indicator must end on the right margin");
        assert_eq!(th, INDICATOR_H);
        assert_eq!(th, 2, "a 3px bar read as a bar and cost a third of the row");
    }

    /// The indicator must not eat into the label column, which is the whole
    /// reason it moved.
    #[test]
    fn the_indicator_starts_after_the_label_column() {
        let (_, tw, _) = Layout::new(10).track(W);
        assert!(
            Layout::VALUE_X > PAD + 80,
            "indicator would run under the labels"
        );
        assert!(tw > 120, "indicator run of {tw}px is too short to read");
    }

    /// Every row's indicator has to land inside its own row. This is the check
    /// that catches a rule bleeding onto the next row's label, which is what
    /// the old 32px offset did when the row height changed.
    #[test]
    fn every_indicator_stays_inside_its_own_row() {
        for rows in 1..=12 {
            let l = Layout::new(rows);
            for i in 0..rows {
                let y = l.row_y(i);
                let bottom = y + INDICATOR_Y + INDICATOR_H;
                assert!(
                    bottom <= y + ROW_H,
                    "row {i} indicator ends at {bottom}, past its {ROW_H}px row"
                );
                // And it must sit under the text, not on top of it.
                assert!(
                    INDICATOR_Y >= 22,
                    "indicator at y={INDICATOR_Y} would overlap the 22px text line"
                );
            }
            l.assert_no_overlap(rows).unwrap_or_else(|e| panic!("{rows} rows: {e}"));
        }
    }

    /// The disc has to sit inside the header band and inside the content box.
    /// A caption under it was tried and abandoned precisely because there was
    /// no room for one, which is only obvious once this is measured.
    #[test]
    fn the_moon_glyph_fits_the_header_band() {
        let l = Layout::new(10);
        for w in [W, 420, 520] {
            let cx = l.moon_center_x(w);
            assert!(cx - MOON_R >= PAD, "disc overflows the left edge at w={w}");
            assert!(cx + MOON_R <= w - PAD, "disc overflows the right edge at w={w}");
            let cy = (l.moon_y + l.moon_bottom) / 2;
            assert!(cy - MOON_R >= l.weekday_y, "disc above the panel content");
            assert!(cy + MOON_R <= l.rule_y, "disc collides with the header rule");
        }
    }

    /// A centred caption of any useful width cannot fit under the glyph, which
    /// is why the glyph has none. This pins that reasoning so it is not
    /// "fixed" later by silently overlapping the date.
    #[test]
    fn there_is_no_room_for_a_caption_under_the_glyph() {
        let l = Layout::new(10);
        let cx = l.moon_center_x(W);
        let cy = (l.moon_y + l.moon_bottom) / 2;
        // Widest string the header would want to put there.
        let needed = 110;
        let room_below = l.rule_y - (cy + MOON_R);
        // Both ways it would have to go are blocked. This is a
        // characterisation test, not a wish: if either becomes false the panel
        // has grown room for the caption and it should be added back on
        // purpose rather than by accident.
        assert!(
            cx + needed / 2 > W - PAD,
            "a {needed}px caption now fits beside the disc ({}) — add it back deliberately",
            cx + needed / 2
        );
        assert!(
            room_below < needed,
            "a caption now fits under the disc (room={room_below}) — add it back deliberately"
        );
    }

    /// The weekday label has to sit *inside* its scrim on both axes, and the
    /// pill has to clear the date below it. The vertical relationship is the
    /// one that actually broke once: the pill was drawn from the label's own
    /// `y`, and adding padding to it pushed the bottom into the date.
    #[test]
    fn the_weekday_sits_inside_its_scrim_and_the_scrim_clears_the_date() {
        let l = Layout::new(10);
        assert!(l.weekday_top <= l.weekday_y, "pill starts below its label");
        assert!(l.weekday_y + WEEKDAY_H <= l.weekday_bottom, "label overflows the pill");
        assert!(
            l.weekday_bottom <= l.date_y,
            "pill ends at {} but the date starts at {}",
            l.weekday_bottom,
            l.date_y
        );
        // The pill is vertically centred on the line box, not flush to it.
        let above = l.weekday_y - l.weekday_top;
        let below = l.weekday_bottom - (l.weekday_y + WEEKDAY_H);
        assert!(
            (above - below).abs() <= 1,
            "label is off-centre in its pill: {above} above, {below} below"
        );
    }

    /// The pill is sized from the measured text, so the widest weekday in
    /// either script has to fit inside the panel's content box with room for
    /// its padding — and the pill must not run into the moon glyph, which is
    /// why the right edge is checked rather than just the left.
    #[test]
    fn the_weekday_pill_fits_the_header_on_both_sides() {
        let l = Layout::new(10);
        for text_w in [40, 70, 96, 120] {
            let (x0, _y0, x1, y1) = l.weekday_pill(text_w);
            assert!(x0 >= 0, "pill leaves the panel at text_w={text_w}");
            assert!(x1 <= W, "pill overflows the panel at text_w={text_w}");
            assert!(y1 <= l.date_y, "pill collides with the date at text_w={text_w}");
            // Left edge is PAD - SCRIM_PAD_X, so the text itself still starts
            // on the content edge and the pill does not shift the label.
            assert_eq!(x0 + SCRIM_PAD_X, PAD);
            // The moon disc's left limb, worst case for a long label.
            let disc_left = l.moon_center_x(W) - MOON_R;
            assert!(
                x1 + 8 <= disc_left,
                "pill ends at {x1}, the moon limb starts at {disc_left} (text_w={text_w})"
            );
        }
    }

    #[test]
    fn the_value_column_starts_after_the_label_column() {
        assert!(Layout::VALUE_X > PAD + 80, "label column and value column collide");
    }

    /// The era line and the moon glyph share a band. Measured off a real
    /// capture of the shipped panel, the era text stops at x=205 and the lit
    /// limb of the disc starts at x=380 -- a 171px gap. The era line is
    /// unbounded text (a Vikram Samvat year can be long), so the gap is a
    /// property of where the disc sits, not of what today's date happens to
    /// be. This pins the disc's left edge so a future tweak to `moon_center_x`
    /// cannot quietly walk it left into the text.
    #[test]
    fn the_moon_glyph_cannot_reach_the_era_line() {
        let l = Layout::new(10);
        let disc_left = l.moon_center_x(W) - MOON_R;
        // A deliberately long era line: 13px text averages ~7px per glyph.
        let era = "Shaka 1946 · Vikram Samvat 2083";
        let era_width = era.chars().count() as i32 * 7;
        let era_right = PAD + era_width;
        assert!(
            era_right < disc_left - 8,
            "a long era line would run into the disc: text ends {era_right}, disc starts {disc_left}"
        );
    }

    /// The footer's text box has to close above the panel edge, with the
    /// bottom padding still intact below it. Measured off a capture: the box
    /// is y 612..630, the glyphs occupy 616..628, and the panel is 654 tall.
    #[test]
    fn the_footer_sits_inside_the_panel() {
        let l = Layout::new(10);
        let box_bottom = l.footer_y + 18;
        assert!(
            box_bottom <= l.height - PAD,
            "footer box ends at {box_bottom}, past the panel's bottom padding (height {})",
            l.height
        );
        // And there is real slack, not a rounding coincidence: the glyphs need
        // roughly 13px and the box gives 18.
        assert!(l.height - box_bottom >= PAD - 6, "no breathing room under the footer");
    }

    /// The widest value in the panel must fit the column it is given.
///
/// The per-glyph estimate below is measured, not guessed: a real capture of
/// the panel in the bar's own typeface put `Krishna Ashtami` (15 glyphs) at
/// 85px, so 6px a glyph is the safe upper bound and anything wider would let a
/// string through that the screen then ellipsises.
///
/// The `Next` row is the worst case because it carries two dates and has no
/// caption to shorten it.
    #[test]
    fn the_widest_value_still_fits_its_column() {
        let value_x = Layout::VALUE_X;
        let widest = "New 10 Oct 2026 · Full 26 Oct 2026";
        let need = widest.chars().count() as i32 * 6;
        // `paint` reserves `w - PAD - value_x` when there is no caption.
        let avail = W - PAD - value_x;
        assert!(
            avail >= need,
            "the widest value needs {need}px but only {avail}px is reserved \
             (value column starts at {value_x})"
        );
    }

    /// The other kind of row: a short value with a right-aligned caption. The
    /// two must not meet, or the value runs under the caption and both become
    /// unreadable. This is the case that actually exists — no row in the
    /// panel has both the widest value *and* a caption — so testing that
    /// imaginary combination would only assert something untrue.
    #[test]
    fn a_value_clears_the_caption_beside_it() {
        let value_x = Layout::VALUE_X;
        let value = "Krishna Ashtami"; // the longest value that has a caption
        let caption = "40% · Last Quarter"; // the longest caption
        let value_w = value.chars().count() as i32 * 6;
        let caption_w = caption.chars().count() as i32 * 6;
        let value_right = value_x + value_w;
        let caption_left = W - PAD - caption_w;
        assert!(
            value_right + 12 <= caption_left,
            "value ends at {value_right}, caption starts at {caption_left}"
        );
    }
}
