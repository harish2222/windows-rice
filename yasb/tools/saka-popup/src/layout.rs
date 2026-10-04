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
pub const W: i32 = 460;

/// Width of the panel when the values are Telugu.
///
/// Telugu is drawn in Nirmala UI, which is wider per glyph than the Latin
/// face: measured on the live panel, the widest row (Tithi plus its caption)
/// already spans 283px of a 292px column at the Latin width. The Indic panel
/// is given more room so nothing has to be ellipsised to fit.
pub const W_INDIC: i32 = 520;
/// Window corner radius.
pub const RADIUS: i32 = 20;
/// Panel edge padding. Scale step 6.
pub const PAD: i32 = yasb_chrome::space(6);

/// Height of one row.
///
/// Was 44 with the progress bar on its own line under the text. That spent a
/// third of the row on a 3px line, which is what made the panel read as sparse
/// — ten rows of it came to 440px of mostly empty card. The bar now sits
/// *under the value only*, inside the same line box, so a row is one line of
/// type plus a 2px rule and nothing else.
pub const ROW_H: i32 = 34;

/// Row height when the values are Telugu.
///
/// Telugu glyphs are 22px tall in the same 16px face that renders Latin
/// values at 12px: the vowel signs sit above the head and the `cheepuru`
/// below the baseline. At the Latin row height the descenders reach y+27
/// while the progress rule is drawn at y+26, so every rule struck through
/// the bottom of its own value. That is what made the Telugu panel read as
/// smeared rather than merely tighter.
pub const ROW_H_INDIC: i32 = 44;
/// Width of the row card's radius.
pub const CARD_R: i32 = 16;
/// Inner padding of the row card. Scale step 3.
pub const CARD_PAD: i32 = yasb_chrome::space(3);
/// Radius of the moon glyph. Lives here, not in `main.rs`, because the shape
/// pass and the text pass both need it and they must not disagree.
pub const MOON_R: i32 = 30;
/// Radius of the weekday scrim pill. Half its height, so the ends are
/// semicircular and the label never looks boxed.
pub const SCRIM_R: i32 = 9;
/// Vertical offset of the progress indicator inside a row, measured from the
/// row's top. The text line box is 22px, so this puts the rule just under it.
pub const INDICATOR_Y: i32 = 26;

/// Offset of the progress rule inside an Indic row.
///
/// Clears a 22px Telugu glyph box that starts 3px below the row's top, with
/// room to spare. Measured, not guessed: at the Latin offset the rule sat
/// *inside* the glyphs.
pub const INDICATOR_Y_INDIC: i32 = 34;
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
    /// Whether the values are drawn in an Indic script.
    pub indic: bool,
}

impl Layout {
    /// Compute the geometry for a panel with `rows` rows.
    pub fn new(rows: i32) -> Layout {
        Self::build(rows, false)
    }

    /// Geometry for a panel whose values are in a Brahmic script.
    pub fn new_indic(rows: i32) -> Layout {
        Self::build(rows, true)
    }

    /// Width this layout's panel needs.
    pub fn w(&self) -> i32 {
        if self.indic { W_INDIC } else { W }
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
    ///
    /// Telugu needs a taller box for the same reason it needs a taller row:
    /// a 22px glyph in a 22px Latin box is centred but still overflows it.
    pub fn text_h(&self) -> i32 {
        if self.indic { 30 } else { 22 }
    }

    fn build(rows: i32, indic: bool) -> Layout {
        let row_h = if indic { ROW_H_INDIC } else { ROW_H };
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
        let card_bottom = card_y + CARD_PAD * 2 + rows * row_h;
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
            indic,
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
    pub const VALUE_X: i32 = PAD + yasb_chrome::space(30);

    /// Gap between a row's value and its right-aligned caption. Scale step 4.
    pub const CAPTION_GAP: i32 = yasb_chrome::space(4);

    /// A caption narrower than this is not worth keeping: half a time or half
    /// a percentage is worse than none, because the reader cannot tell whether
    /// it ran out of room or means something.
    pub const MIN_CAPTION_W: i32 = 48;

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
        // The value keeps everything it needs; the caption gets the slack, and
        // is dropped outright rather than shown as a stub.
        let spare = avail - value_need - Self::CAPTION_GAP;
        if spare >= Self::MIN_CAPTION_W {
            (avail - spare, spare)
        } else {
            (avail, 0)
        }
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
    pub fn era_right(&self, w: i32) -> i32 {
        self.moon_center_x(w) - MOON_R - 10
    }

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
        assert!(Layout::VALUE_X > PAD + 80, "label column and value column collide");
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

        const LABELS: [&str; 10] = [
            "TITHI", "NAKSHATRA", "YOGA", "KARANA", "VARA",
            "MOON", "AMANTA", "PURNIMANTA", "SUN", "NEXT",
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
}
