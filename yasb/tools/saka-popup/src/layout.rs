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

/// Width of the panel, Latin.
pub const W: i32 = 660;

/// Width of the panel when the values are Telugu.
pub const W_INDIC: i32 = 720;
/// Width in landscape mode (Latin).
pub const W_LANDSCAPE: i32 = 1040;
/// Width in landscape mode (Indic).
pub const W_LANDSCAPE_INDIC: i32 = 1080;
/// Width in compact glance mode.
pub const W_COMPACT: i32 = 580;
/// Width in compact glance mode (Indic).
pub const W_COMPACT_INDIC: i32 = 620;
/// Window corner radius.
pub const RADIUS: i32 = 20;
/// Panel edge padding. Scale step 6 (24px).
pub const PAD: i32 = yasb_chrome::space(6);

/// Height of one row.
pub const ROW_H: i32 = 50;

/// Row height when the values are Telugu.
pub const ROW_H_INDIC: i32 = 58;
/// Width of the row card's radius.
pub const CARD_R: i32 = 16;
/// Inner padding of the row card. Scale step 3.
pub const CARD_PAD: i32 = yasb_chrome::space(3);
/// Radius of the moon glyphs. Dual large hero spheres.
pub const MOON_R: i32 = 42;
/// Radius of the weekday scrim pill. Half its height, so the ends are
/// semicircular and the label never looks boxed.
pub const SCRIM_R: i32 = 10;
/// Vertical offset of the progress indicator inside a row, measured from the
/// row's top. The text line box is 24px (Latin) / 30px (Indic) starting at y+4,
/// so this leaves at least 10px downward padding before the indicator rule.
pub const INDICATOR_Y: i32 = 38;

/// Offset of the progress rule inside an Indic row.
pub const INDICATOR_Y_INDIC: i32 = 44;
/// Thickness of the progress indicator. 2px reads as a rule rather than a bar.
pub const INDICATOR_H: i32 = 2;
/// Horizontal padding inside the weekday pill, either side of the text.
pub const SCRIM_PAD_X: i32 = 12;
/// The weekday label's line box, which is what the pill is sized around.
pub const WEEKDAY_H: i32 = 18;

/// Layout presentation mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum LayoutMode {
    #[default]
    Portrait,
    Landscape,
    Compact,
}

/// Where each band starts and ends, measured from the panel's top.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Layout {
    pub mode: LayoutMode,
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
    /// Udaya Tithi and Day Length band.
    pub udaya_y: i32,
    pub udaya_bottom: i32,
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
    /// Whether the values are drawn in an Indic script.
    pub indic: bool,
    /// Sunrise / Sunset timeline band.
    pub timeline_y: i32,
    pub timeline_bottom: i32,
    /// Today's Panchang primary card.
    pub panchang_card_y: i32,
    pub panchang_card_bottom: i32,
    /// Moon & Calendar 2x2 / horizontal tile grid.
    pub grid_y: i32,
    pub grid_bottom: i32,
    /// 3D Rotating Moon sphere center.
    pub moon_cx: i32,
    pub moon_cy: i32,
    /// Astronomical Sky Phase Moon center.
    pub sky_moon_cx: i32,
    pub moon_r: i32,
}

impl Layout {
    /// Compute the geometry for a panel with `rows` rows.
    pub fn new(rows: i32) -> Layout {
        Self::build(rows, false)
    }

    /// Geometry for a panel whose values are in a Brahmic script.
    #[allow(dead_code)]
    pub fn new_indic(rows: i32) -> Layout {
        Self::build(rows, true)
    }

    /// Compute layout for a specific mode.
    #[allow(dead_code)]
    pub fn new_mode(mode: LayoutMode, indic: bool) -> Layout {
        Self::build_mode(mode, 10, indic)
    }

    /// Geometry for tile `idx` (0..4) in the Moon & Calendar section.
    pub fn tile_rect(&self, idx: usize) -> (i32, i32, i32, i32) {
        if self.mode == LayoutMode::Landscape {
            let w = self.w();
            let gutter = 14;
            let pad = PAD;
            let avail = w - pad * 2 - gutter * 3;
            let tile_w = avail / 4;
            let tile_h = if self.indic { 112 } else { 104 };
            let x0 = pad + (idx as i32) * (tile_w + gutter);
            let x1 = x0 + tile_w;
            let y0 = self.grid_y;
            let y1 = y0 + tile_h;
            (x0, y0, x1, y1)
        } else {
            let w = self.w();
            let col = (idx % 2) as i32;
            let row = (idx / 2) as i32;
            let gutter = 14;
            let pad = PAD;
            let avail = w - pad * 2 - gutter;
            let tile_w = avail / 2;
            let tile_h = if self.indic { 112 } else { 104 };
            let row_gap = 12;
            let x0 = pad + col * (tile_w + gutter);
            let x1 = x0 + tile_w;
            let y0 = self.grid_y + row * (tile_h + row_gap);
            let y1 = y0 + tile_h;
            (x0, y0, x1, y1)
        }
    }

    /// Width this layout's panel needs.
    pub fn w(&self) -> i32 {
        match self.mode {
            LayoutMode::Landscape => if self.indic { W_LANDSCAPE_INDIC } else { W_LANDSCAPE },
            LayoutMode::Compact => if self.indic { W_COMPACT_INDIC } else { W_COMPACT },
            LayoutMode::Portrait => if self.indic { W_INDIC } else { W },
        }
    }

    /// Height of one row.
    pub fn row_h(&self) -> i32 {
        if self.indic { ROW_H_INDIC } else { ROW_H }
    }

    /// Offset of the progress rule inside a row.
    pub fn indicator_y(&self) -> i32 {
        if self.indic { INDICATOR_Y_INDIC } else { INDICATOR_Y }
    }

    /// Height of a row's text line box.
    pub fn text_h(&self) -> i32 {
        if self.indic { 30 } else { 24 }
    }

    fn build(rows: i32, indic: bool) -> Layout {
        let row_h = if indic { ROW_H_INDIC } else { ROW_H };
        let weekday_y = PAD;
        let weekday_top = weekday_y - 3;
        let weekday_bottom = weekday_top + WEEKDAY_H + 6;
        let date_y = weekday_bottom + 12;
        let date_h = 42;
        let era_y = date_y + date_h + 12;
        let era_bottom = era_y + 22;
        let udaya_y = era_bottom + 12;
        let udaya_bottom = udaya_y + 22;
        let mode = LayoutMode::Portrait;
        let w = if indic { W_INDIC } else { W };

        let moon_r = MOON_R;
        let sky_moon_cx = w - PAD - moon_r;
        let moon_cx = sky_moon_cx - 96;
        let moon_cy = 100;
        let moon_y = moon_cy - moon_r;
        let moon_bottom = moon_cy + moon_r;

        let badge_bottom = moon_bottom + 10 + 22;
        let top_content_bottom = udaya_bottom.max(badge_bottom);
        let timeline_y = top_content_bottom + 16;
        let timeline_bottom = timeline_y + 24;
        let rule_y = timeline_bottom + 14;

        let panchang_card_y = rule_y + 42;
        let panchang_card_bottom = panchang_card_y + CARD_PAD * 2 + 5 * row_h;
        let grid_y = panchang_card_bottom + 44;
        let tile_h = if indic { 112 } else { 104 };
        let row_gap = 12;
        let grid_bottom = grid_y + tile_h * 2 + row_gap;

        let card_y = panchang_card_y;
        let card_bottom = panchang_card_y + CARD_PAD * 2 + rows.max(5) * row_h;

        let footer_y = grid_bottom + 18;
        let height = footer_y + 20 + PAD;
        Layout {
            mode,
            weekday_y,
            weekday_top,
            weekday_bottom,
            date_y,
            era_y,
            era_bottom,
            udaya_y,
            udaya_bottom,
            rule_y,
            moon_y,
            moon_bottom,
            card_y,
            card_bottom,
            footer_y,
            height,
            indic,
            timeline_y,
            timeline_bottom,
            panchang_card_y,
            panchang_card_bottom,
            grid_y,
            grid_bottom,
            moon_cx,
            moon_cy,
            sky_moon_cx,
            moon_r,
        }
    }

    /// Geometry for a specific presentation mode.
    pub fn build_mode(mode: LayoutMode, rows: i32, indic: bool) -> Layout {
        match mode {
            LayoutMode::Portrait => Self::build(rows, indic),
            LayoutMode::Landscape => {
                let _w = if indic { W_LANDSCAPE_INDIC } else { W_LANDSCAPE };
                let row_h = if indic { ROW_H_INDIC } else { ROW_H };
                let weekday_y = PAD;
                let weekday_top = weekday_y - 3;
                let weekday_bottom = weekday_top + WEEKDAY_H + 6;
                let date_y = weekday_bottom + 12;
                let date_h = 42;
                let era_y = date_y + date_h + 12;
                let era_bottom = era_y + 22;
                let udaya_y = era_bottom + 12;
                let udaya_bottom = udaya_y + 22;

                let moon_r = 44;
                let moon_cx = 120;
                let sky_moon_cx = 240;
                let moon_cy = 250;
                let moon_y = moon_cy - moon_r;
                let moon_bottom = moon_cy + moon_r;

                let badge_bottom = moon_bottom + 10 + 22;
                let timeline_y = badge_bottom + 16;
                let timeline_bottom = timeline_y + 24;

                let panchang_card_y = PAD + 32;
                let panchang_card_bottom = panchang_card_y + CARD_PAD * 2 + 5 * row_h;
                let deck_bottom = panchang_card_bottom.max(timeline_bottom);
                let rule_y = deck_bottom + 16;
                let grid_y = rule_y + 16;
                let tile_h = if indic { 112 } else { 104 };
                let grid_bottom = grid_y + tile_h;

                let card_y = panchang_card_y;
                let card_bottom = grid_bottom;
                let footer_y = grid_bottom + 18;
                let height = footer_y + 20 + PAD;
                Layout {
                    mode,
                    weekday_y,
                    weekday_top,
                    weekday_bottom,
                    date_y,
                    era_y,
                    era_bottom,
                    udaya_y,
                    udaya_bottom,
                    rule_y,
                    moon_y,
                    moon_bottom,
                    card_y,
                    card_bottom,
                    footer_y,
                    height,
                    indic,
                    timeline_y,
                    timeline_bottom,
                    panchang_card_y,
                    panchang_card_bottom,
                    grid_y,
                    grid_bottom,
                    moon_cx,
                    moon_cy,
                    sky_moon_cx,
                    moon_r,
                }
            }
            LayoutMode::Compact => {
                let w = if indic { W_COMPACT_INDIC } else { W_COMPACT };
                let weekday_y = PAD;
                let weekday_top = weekday_y - 3;
                let weekday_bottom = weekday_top + WEEKDAY_H + 6;
                let date_y = weekday_bottom + 10;
                let date_h = 36;
                let era_y = date_y + date_h + 10;
                let era_bottom = era_y + 22;
                let udaya_y = 0;
                let udaya_bottom = 0;

                let moon_r = 38;
                let sky_moon_cx = w - PAD - moon_r - 8;
                let moon_cx = sky_moon_cx;
                let moon_cy = 72;
                let moon_y = moon_cy - moon_r;
                let moon_bottom = moon_cy + moon_r;

                let rule_y = era_bottom + 14;
                let panchang_card_y = rule_y + 14;
                let panchang_card_bottom = panchang_card_y + if indic { 96 } else { 88 };

                let timeline_y = panchang_card_bottom + 12;
                let timeline_bottom = timeline_y + if indic { 84 } else { 78 };

                let grid_y = 0;
                let grid_bottom = 0;
                let card_y = panchang_card_y;
                let card_bottom = timeline_bottom;
                let footer_y = timeline_bottom + 16;
                let height = footer_y + 20 + PAD;
                Layout {
                    mode,
                    weekday_y,
                    weekday_top,
                    weekday_bottom,
                    date_y,
                    era_y,
                    era_bottom,
                    udaya_y,
                    udaya_bottom,
                    rule_y,
                    moon_y,
                    moon_bottom,
                    card_y,
                    card_bottom,
                    footer_y,
                    height,
                    indic,
                    timeline_y,
                    timeline_bottom,
                    panchang_card_y,
                    panchang_card_bottom,
                    grid_y,
                    grid_bottom,
                    moon_cx,
                    moon_cy,
                    sky_moon_cx,
                    moon_r,
                }
            }
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

    /// Top of the weekday label's glyph box, centred in its scrim.
    ///
    /// `tracked_text` anchors on the top of the line, so the label's position
    /// is not free: it has to be computed from the pill rather than guessed,
    /// or the text drifts out of its scrim the moment the type size or the
    /// pill's padding changes.
    pub fn weekday_text_top(&self) -> i32 {
        let pill_h = self.weekday_bottom - self.weekday_top;
        self.weekday_top + (pill_h - yasb_chrome::Type::Meta.px()) / 2
    }

    /// Top of row `i` inside the card.
    pub fn row_y(&self, i: i32) -> i32 {
        self.card_y + CARD_PAD + i * self.row_h()
    }

    /// Left edge of the value column.
    ///
    /// Sized off the *actual* label set in the bar's own typeface, at the size
    /// the labels are drawn, rather than off a round number. The panel used a
    /// monospace-ish Segoe UI and 96px was generous; switching to the bar's
    /// Nerd Font widened the advances, and then enlarging the labels from 11px
    /// to 13px widened them again. Measured through GDI, `PURNIMANTA` at 13px
    /// with its tracking is 108px, so the column is 120px: the label plus a
    /// full character of clearance.
    ///
    /// Enlarging the labels is therefore not free in general — it eats the
    /// value column — and `the_shipped_rows_never_need_an_ellipsis` is what
    /// proves the trade still fits the `Next` row's two dates.
    pub const VALUE_X: i32 = PAD + yasb_chrome::space(28);

    /// Gap between a row's value and its right-aligned caption. Scale step 4.
    pub const CAPTION_GAP: i32 = yasb_chrome::space(4);

    /// Split one row's line box between its value and its optional caption.
    ///
    /// Returns `(value_w, caption_w)`, with `caption_w == 0` when the row has
    /// no caption or when the caption lost its space.
    ///
    /// The panel used to reserve a flat 96px on *every* row for a caption that
    /// only five of the ten have. That cost the rows without one a quarter of
    /// their column, which is what ellipsised `New 10 Oct 2026 · Full 26 Oct
    /// 2026` down to `New 10 Oct 2026 · ...`, and it was simultaneously too
    /// *narrow* for the caption it was meant to protect: the `Moon` row's
    /// `39% % · waning` measures 102px, so it was clipped inside its own
    /// reservation. One fixed number was wrong in both directions at once.
    ///
    /// Sizing both from what the text actually measures fixes both at the same
    /// time. When the two genuinely cannot coexist the caption yields first:
    /// the value is the datum, the caption is a percentage and an end time.
    pub fn row_split(w: i32, value_need: i32, caption_need: Option<i32>) -> (i32, i32) {
        let avail = w - PAD - Self::VALUE_X;
        let Some(cap_need) = caption_need else { return (avail, 0) };
        if value_need + Self::CAPTION_GAP + cap_need <= avail {
            return (avail - Self::CAPTION_GAP - cap_need, cap_need);
        }
        // They cannot coexist, and the caption is what yields: the value is the
        // datum, the caption is a percentage and an end time.
        //
        // It is dropped *whole*. The previous version handed it whatever was
        // left over after the value took what it needed and called that
        // "dropped outright rather than shown as a stub" — but the leftover is
        // by construction less than the caption needs (the branch above would
        // otherwise have fired), so `paint` measured it, found it narrower than
        // its own text and drew `DT_END_ELLIPSIS`: the tithi row showed `...`
        // where the end time belongs. Half a time is worse than none, because
        // the reader cannot tell whether it ran out of room or means something.
        (avail, 0)
    }

    /// Right edge of the era line's text box.
    ///
    /// The era line shares its band with the moon glyph, so an unbounded text
    /// box would run the year straight under the disc. The box stops short of
    /// the disc's left limb instead.
    ///
    /// This used to be a test with a 7px-per-glyph *estimate* in it, which is
    /// how the Telugu panel came to clip its era line: the estimate happened
    /// to hold for Latin and the Indic face is a different width again. The
    /// bound is computed here so the paint pass and the test agree by
    /// construction rather than by coincidence.
    #[allow(dead_code)]
    pub fn era_right(&self, w: i32) -> i32 {
        self.moon_center_x(w) - MOON_R - 10
    }

    /// Centre x of the rightmost moon glyph (flush with the content's right edge).
    #[allow(dead_code)]
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
        if self.mode == LayoutMode::Compact {
            let stacked = [
                ("weekday pill", self.weekday_top, self.weekday_bottom),
                ("date", self.date_y, self.date_y + 36),
                ("era", self.era_y, self.era_bottom),
                ("rule", self.rule_y, self.rule_y + 2),
                ("panchang_card", self.panchang_card_y, self.panchang_card_bottom),
                ("timeline_card", self.timeline_y, self.timeline_bottom),
                ("footer", self.footer_y, self.footer_y + 20),
            ];
            for pair in stacked.windows(2) {
                let (an, _, a_bottom) = pair[0];
                let (bn, b_top, _) = pair[1];
                if a_bottom > b_top {
                    return Err(format!("{an} ends at {a_bottom} but {bn} starts at {b_top}"));
                }
            }
            if self.footer_y + 20 + PAD > self.height {
                return Err(format!("footer overflows the compact panel: {}", self.height));
            }
            return Ok(());
        }

        if self.mode == LayoutMode::Landscape {
            let stacked = [
                ("weekday pill", self.weekday_top, self.weekday_bottom),
                ("date", self.date_y, self.date_y + 42),
                ("era", self.era_y, self.era_bottom),
                ("udaya", self.udaya_y, self.udaya_bottom),
                ("moons", self.moon_y - 20, self.moon_bottom),
                ("timeline", self.timeline_y, self.timeline_bottom),
                ("rule", self.rule_y, self.rule_y + 2),
                ("grid", self.grid_y, self.grid_bottom),
                ("footer", self.footer_y, self.footer_y + 20),
            ];
            for pair in stacked.windows(2) {
                let (an, _, a_bottom) = pair[0];
                let (bn, b_top, _) = pair[1];
                if a_bottom > b_top {
                    return Err(format!("{an} ends at {a_bottom} but {bn} starts at {b_top}"));
                }
            }
            if self.footer_y + 20 + PAD > self.height {
                return Err(format!("footer overflows the landscape panel: {}", self.height));
            }
            return Ok(());
        }

        let stacked = [
            ("weekday pill", self.weekday_top, self.weekday_bottom),
            ("date", self.date_y, self.date_y + 42),
            ("era", self.era_y, self.era_bottom),
            ("udaya", self.udaya_y, self.udaya_bottom),
            ("timeline", self.timeline_y, self.timeline_bottom),
            ("rule", self.rule_y, self.rule_y + 2),
            ("card", self.panchang_card_y, self.panchang_card_bottom),
            ("grid", self.grid_y, self.grid_bottom),
            ("footer", self.footer_y, self.footer_y + 20),
        ];
        for pair in stacked.windows(2) {
            let (an, _, a_bottom) = pair[0];
            let (bn, b_top, _) = pair[1];
            if a_bottom > b_top {
                return Err(format!("{an} ends at {a_bottom} but {bn} starts at {b_top}"));
            }
        }
        // Every row's content must fit inside the row.
        for i in 0..rows.min(5) {
            let y = self.row_y(i);
            // The indicator sits under the value's line box. It has to clear
            // the text above it and still end inside the row, or it lands on
            // the next row's label.
            if y + self.indicator_y() + INDICATOR_H > y + self.row_h() {
                return Err(format!("row {i} indicator overflows: {y}"));
            }
        }
        if self.footer_y + 20 + PAD > self.height {
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
        assert_eq!(l.height, l.footer_y + 20 + PAD);
        assert!(l.height > 600 && l.height < 1050, "height {}", l.height);
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
        const {
            assert!(Layout::VALUE_X > PAD + 80, "indicator would run under the labels");
        }
        assert!(tw > 120, "indicator run of {tw}px is too short to read");
    }

    /// Every row's indicator has to land inside its own row. This is the check
    /// that catches a rule bleeding onto the next row's label, which is what
    /// the old 32px offset did when the row height changed.
    #[test]
    fn every_indicator_stays_inside_its_own_row() {
        // Row-invariant: hoisted out of the loops and made compile-time, so a
        // future ROW_H tweak cannot silently reintroduce the overlap.
        const {
            assert!(INDICATOR_Y >= 22, "indicator would overlap the 22px text line");
        }
        for rows in 1..=12 {
            let l = Layout::new(rows);
            for i in 0..rows {
                let y = l.row_y(i);
                let bottom = y + INDICATOR_Y + INDICATOR_H;
                assert!(
                    bottom <= y + ROW_H,
                    "row {i} indicator ends at {bottom}, past its {ROW_H}px row"
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

    /// The weekday label is drawn top-anchored, so the glyph box has to land
    /// inside the pill it sits on.
    #[test]
    fn the_weekday_glyph_box_sits_inside_its_scrim() {
        let l = Layout::new(10);
        let top = l.weekday_text_top();
        let bottom = top + yasb_chrome::Type::Meta.px();
        assert!(top >= l.weekday_top, "label starts above its pill: {top} < {}", l.weekday_top);
        assert!(bottom <= l.weekday_bottom, "label ends below its pill: {bottom} > {}", l.weekday_bottom);
        let above = top - l.weekday_top;
        let below = l.weekday_bottom - bottom;
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
    }    #[test]
    fn the_indic_rule_clears_an_indic_glyph() {
        // Measured on the live panel: a Telugu value occupies 22px starting
        // 9px below its row top, so it reaches y+27. The Latin rule is drawn
        // at y+26 -- inside the glyph. Every progress bar was striking through
        // the bottom of the value it described, which is what made the Telugu
        // panel read as smeared rather than merely tighter.
        const GLYPH_TOP: i32 = 9;
        const GLYPH_H: i32 = 22;
        let glyph_bottom = GLYPH_TOP + GLYPH_H;
        let l = Layout::new_indic(10);
        let rule = l.indicator_y();
        assert!(
            rule >= glyph_bottom,
            "the rule at y+{rule} cuts through a Telugu glyph reaching y+{glyph_bottom}"
        );
        assert!(
            rule + INDICATOR_H <= l.row_h(),
            "the rule at y+{rule} runs past its {}-px row",
            l.row_h()
        );
        // Latin is deliberately left alone: its values are 12px tall and end
        // around y+21, so its rule at y+26 already clears them. An earlier
        // version of this test asserted the Indic clearance against the Latin
        // layout too and failed there for no reason -- the two scripts have
        // genuinely different metrics and pretending otherwise would have
        // forced the Latin row to be as tall as the Telugu one.
        assert_eq!(Layout::new(10).indicator_y(), INDICATOR_Y);
    }

    #[test]
    fn the_indic_panel_is_wider_and_its_rows_are_taller() {
        let latin = Layout::new(10);
        let indic = Layout::new_indic(10);
        assert!(indic.w() > latin.w(), "indic panel is not wider");
        assert!(indic.row_h() > latin.row_h(), "indic rows are not taller");
        assert!(indic.text_h() > latin.text_h(), "indic text box is not taller");
        latin.assert_no_overlap(10).unwrap();
        indic.assert_no_overlap(10).unwrap();
    }

    #[test]
    fn the_value_column_starts_after_the_label_column() {
        const {
            assert!(Layout::VALUE_X > PAD + 80, "label column and value column collide");
        }
    }

    /// The label column, measured against the largest label at the size it is
    /// actually drawn at.
    ///
    /// The labels were 11px against 16px values, a ratio of 1.45 that left the
    /// left column reading as an afterthought. They are now
    /// [`yasb_chrome::Type::Meta`] — 13px — which closes the gap to 1.23 and
    /// still fits: the longest label, `PURNIMANTA`, is 80px of glyphs plus 10px
    /// of tracking, and the column is 104px. That is why enlarging the labels
    /// cost the value column nothing.
    #[test]
    fn the_longest_label_fits_the_label_column_at_its_drawn_size() {
        use yasb_chrome::gdi::{font, text_width};
        use windows::Win32::Graphics::Gdi::{
            DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
        };

        const LABELS: [&str; 12] = [
            "TITHI", "NAKSHATRA", "YOGA", "KARANA", "VARA",
            "MOON", "PHASE", "NEXT PHASE", "AMANTA", "PURNIMANTA", "SUN", "NEXT",
        ];
        const TRACKING: i32 = 1;
        let avail = Layout::VALUE_X - PAD;

        unsafe {
            let dc = GetDC(None);
            let f = font(yasb_chrome::Type::Meta.gdi(), 600, "FiraCode Nerd Font Mono");
            let old = SelectObject(dc, HGDIOBJ(f.0));
            let mut worst = 0;
            let mut who = "";
            for l in LABELS {
                let w = text_width(dc, l) + TRACKING * (l.chars().count() as i32 - 1);
                if w > worst {
                    worst = w;
                    who = l;
                }
            }
            let _ = SelectObject(dc, old);
            let _ = ReleaseDC(None, dc);
            let _ = DeleteObject(HGDIOBJ(f.0));
            assert!(
                worst <= avail,
                "{who} is {worst}px but the label column is only {avail}px"
            );
        }
        assert_eq!(
            yasb_chrome::Type::Lead.px() as f32 / yasb_chrome::Type::Meta.px() as f32,
            16.0 / 13.0
        );
    }

    /// The era line and the moon glyph share a band, so the era line's box has
    /// to stop short of the disc. The previous version of this test
    /// multiplied the string's character count by a guessed 7px per glyph,
    /// which held for Latin and silently stopped holding for the Indic face —
    /// which is how the Telugu panel came to clip its era line.
    #[test]
    fn the_era_line_cannot_reach_the_moon_glyph() {
        let l = Layout::new(10);
        let disc_left = l.moon_center_x(W) - MOON_R;
        assert_eq!(l.era_right(W), disc_left - 10);
        assert!(
            l.era_right(W) > PAD + 100,
            "the era line's box has collapsed to {}px",
            l.era_right(W) - PAD
        );
    }

    /// The footer's text box has to close above the panel edge, with the
    /// bottom padding still intact below it. Measured off a capture: the box
    /// is y 612..630, the glyphs occupy 616..628, and the panel is 654 tall.
    #[test]
    fn the_footer_sits_inside_the_panel() {
        let l = Layout::new(10);
        let box_bottom = l.footer_y + 20;
        assert!(
            box_bottom <= l.height - PAD,
            "footer box ends at {box_bottom}, past the panel's bottom padding (height {})",
            l.height
        );
        // And there is real slack, not a rounding coincidence: the glyphs need
        // roughly 13px and the box gives 18.
        assert!(l.height - box_bottom >= PAD - 6, "no breathing room under the footer");
    }

    /// The row split is the fix for the truncating captions, so it gets tested
    /// against the strings that actually broke, measured through GDI in the
    /// bar's own typeface.
    ///
    /// These assertions used to multiply a character count by 6px, which is
    /// how the fixed 96px caption zone survived so long: the estimate said
    /// `39% · waning` was 72px and it quietly fit, while GDI says 102px and
    /// it did not.
    #[test]
    fn the_shipped_rows_never_need_an_ellipsis() {
        use windows::Win32::Graphics::Gdi::{
            DeleteObject, GetDC, HGDIOBJ, ReleaseDC, SelectObject,
        };
        use yasb_chrome::gdi::{font, text_width};

        // (value, caption) exactly as `App::rows` builds them.
        let rows: &[(&str, Option<&str>)] = &[
            // The worst row in the panel: the longest tithi name against the
            // longest caption it can carry. This is the pair the old 460px
            // panel could not hold, and the reason `W` is 496.
            ("Krishna Chaturdashi", Some("100% · 03:56")),
            ("Krishna Navami", Some("58% · 03:56")),
            ("Punarvasu", Some("75% · 00:15")),
            ("Shiva", Some("29% · 09:51")),
            ("Kaulava", Some("second half")),
            ("Ravivara", None),
            // The two that were visibly clipped.
            ("Krishna Ashtami", Some("39% · waning")),
            ("Bhadra · Day 23", None),
            ("Asvina", None),
            ("06:07 · 18:03", None),
            ("New 10 Oct · Full 26 Oct", None),
            // The `Phase` row: current phase name with its end caption.
            ("Krishna Panchami", Some("ends 28 Oct")),
            // The `Next Phase` row: upcoming phase name and start time.
            ("Krishna Panchami · starts 28 Oct", None),
        ];

        unsafe {
            let dc = GetDC(None);
            let body = font(-16, 500, "FiraCode Nerd Font Mono");
            let small = font(-13, 400, "FiraCode Nerd Font Mono");
            for (value, caption) in rows {
                SelectObject(dc, HGDIOBJ(body.0));
                let value_need = text_width(dc, value);
                let cap_need = caption.map(|c| {
                    SelectObject(dc, HGDIOBJ(small.0));
                    text_width(dc, c)
                });
                let (value_w, cap_w) = Layout::row_split(W, value_need, cap_need);
                assert!(
                    value_w >= value_need,
                    "{value:?} needs {value_need}px, was given {value_w}px"
                );
                if let Some(c) = caption {
                    assert!(
                        cap_w >= cap_need.unwrap_or(0),
                        "the caption {c:?} was given {cap_w}px of the \
                         {}px it needs",
                        cap_need.unwrap_or(0)
                    );
                }
            }
            let _ = ReleaseDC(None, dc);
            let _ = DeleteObject(HGDIOBJ(body.0));
            let _ = DeleteObject(HGDIOBJ(small.0));
        }
    }

    /// When the value and the caption genuinely cannot both fit, the caption
    /// is the one that yields. The value is the datum; the caption is a
    /// percentage and an end time.
    #[test]
    fn the_caption_yields_before_the_value_does() {
        let avail = W - PAD - Layout::VALUE_X;
        // A value that eats the whole column.
        let (value_w, cap_w) = Layout::row_split(W, avail, Some(100));
        assert_eq!(value_w, avail, "the value must keep its width");
        assert_eq!(cap_w, 0, "a caption with no room must be dropped, not stubbed");
    }

    /// A caption narrower than its own text is not a shorter caption, it is
    /// `...` — `paint` hands it `DT_END_ELLIPSIS`. So `row_split` must return
    /// either the caption's full width or none of it, for any pair of needs.
    ///
    /// This is the regression fence for the tithi row's `...`: the old
    /// implementation handed the caption whatever was left over, which was
    /// always short by construction.
    #[test]
    fn a_caption_is_either_whole_or_absent_never_a_stub() {
        for w in [W, W_INDIC, 460, 420, 640] {
            for value_need in (0..=400).step_by(20) {
                for cap_need in 0..=140 {
                    let (_, cap_w) = Layout::row_split(w, value_need, Some(cap_need));
                    assert!(
                        cap_w == 0 || cap_w >= cap_need,
                        "at w={w}, value={value_need}, need={cap_need}: caption got {cap_w}px — \
                         that is an ellipsis, not a caption"
                    );
                }
            }
        }
    }

    /// A row with no caption gets the whole column. This is the case the flat
    /// 96px reservation broke: `New 10 Oct · Full 26 Oct` measures 279px and
    /// was being handed 212px.
    #[test]
    fn a_row_without_a_caption_gets_the_whole_column() {
        let (value_w, cap_w) = Layout::row_split(W, 279, None);
        assert_eq!(cap_w, 0);
        assert_eq!(value_w, W - PAD - Layout::VALUE_X);
        assert!(value_w >= 279, "the Next row's value still does not fit");
    }

    #[test]
    fn layout_modes_have_deliberate_proportions() {
        let p_lat = Layout::new_mode(LayoutMode::Portrait, false);
        let p_ind = Layout::new_mode(LayoutMode::Portrait, true);
        let l_lat = Layout::new_mode(LayoutMode::Landscape, false);
        let l_ind = Layout::new_mode(LayoutMode::Landscape, true);
        let c_lat = Layout::new_mode(LayoutMode::Compact, false);

        assert_eq!(p_lat.w(), W);
        assert_eq!(p_ind.w(), W_INDIC);
        assert_eq!(l_lat.w(), W_LANDSCAPE);
        assert_eq!(l_ind.w(), W_LANDSCAPE_INDIC);
        assert_eq!(c_lat.w(), W_COMPACT);

        assert!(l_lat.w() > p_lat.w());
        assert!(p_lat.w() > c_lat.w());
        assert!(c_lat.height < p_lat.height);
    }

    #[test]
    fn tile_rect_stays_within_bounds_for_all_modes() {
        for mode in [LayoutMode::Portrait, LayoutMode::Landscape] {
            for indic in [false, true] {
                let l = Layout::new_mode(mode, indic);
                let w = l.w();
                for i in 0..4 {
                    let (x0, y0, x1, y1) = l.tile_rect(i);
                    assert!(x0 >= PAD - 4, "tile {i} left limb out of bounds: {x0}");
                    assert!(x1 <= w - PAD + 4, "tile {i} right limb out of bounds: {x1} > {w}");
                    assert!(y0 >= l.grid_y, "tile {i} top out of bounds: {y0}");
                    assert!(y1 <= l.grid_bottom, "tile {i} bottom out of bounds: {y1} > {}", l.grid_bottom);
                    assert!(x1 > x0, "tile {i} width is non-positive");
                    assert!(y1 > y0, "tile {i} height is non-positive");
                }
            }
        }
    }
}
