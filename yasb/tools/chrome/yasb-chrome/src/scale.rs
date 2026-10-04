//! One spacing and type scale for every panel.
//!
//! The panels are not a web page, so there is no utility framework to lean
//! on: `styles.css` is Qt QSS read by the bar's own widgets, and the popup
//! panels draw straight into a GDI device context. Nothing in this tree can
//! consume a compiled stylesheet, so the discipline has to live in the code
//! instead — and it did not, which is why the two panels had drifted into
//! unrelated numbers for what is visibly the same idea.
//!
//! This module is that discipline. Two scales, both small:
//!
//! * **Space** — a 4px base. `space(6)` is 24, `space(3)` is 12. Panels
//!   declare `const PAD: i32 = space(6);` instead of `24`, so "the panel's
//!   edge padding" and "the gap inside a card" are visibly the same decision
//!   at two different call sites.
//! * **Type** — the sizes that actually get used, named for their job rather
//!   than for their pixel value, so a heading can be resized without hunting
//!   for every `font(-11, ...)` that was meant to match it.
//!
//! Deliberately *not* here: colour, radius and shadow. Those are per-theme
//! and already live in `Theme`; putting them in a shared scale would imply a
//! single visual identity that the 22 palettes explicitly do not share.
//!
//! ## The honest caveat
//!
//! A handful of values are deliberately off the 4px base — the picker's 10px
//! gutter, the saka panel's 34px row. They are marked where they appear. They
//! are off-scale because the measurements that fixed the truncation defects
//! landed on those numbers, and snapping them to the scale afterwards would
//! mean re-deriving geometry that has been verified against real text widths.
//! Rounding a verified layout back to a prettier number is how a panel starts
//! clipping a string again.

/// The spacing base, in pixels.
///
/// 4px divides every vertical rhythm the panels use and keeps a 2px rule and
/// a 1px border expressible as half-steps without a separate token.
pub const SPACE_BASE: i32 = 4;

/// `step` steps of the spacing scale.
///
/// Steps, not pixels: `space(6)` reads as "the sixth step" and survives the
/// base changing, where a literal `24` silently becomes wrong.
#[inline]
pub const fn space(step: i32) -> i32 {
    step * SPACE_BASE
}

/// The type scale, named for the job each size does.
///
/// Sizes are negative at the `font()` call site because that is GDI's
/// convention (a negative `lfHeight` means "character height", positive means
/// "cell height"), so the values here are the magnitudes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Type {
    /// Tracked uppercase section headings and row labels.
    Label,
    /// Footer hints, match counts, captions.
    Foot,
    /// Supporting metadata: the era line, progress captions.
    Meta,
    /// Cell and row values.
    Body,
    /// Search fields and row values that need a little more presence.
    Lead,
    /// The date. One per panel.
    Title,
}

impl Type {
    /// The magnitude to hand to `font()`.
    pub const fn px(self) -> i32 {
        match self {
            Type::Label => 11,
            Type::Foot => 12,
            Type::Meta => 13,
            Type::Body => 15,
            Type::Lead => 16,
            Type::Title => 28,
        }
    }

    /// The size as `font()` wants it.
    #[inline]
    pub const fn gdi(self) -> i32 {
        -self.px()
    }

    /// Line box for this size: the size plus the leading the scale assumes.
    ///
    /// Panels position text from the top of a line box, not from the top of
    /// the glyphs, so a row height has to be derived from something. Using
    /// `size * 4 / 3` is the same approximation GDI's `lfHeight` already
    /// makes and keeps one number for the whole scale.
    pub const fn line(self) -> i32 {
        self.px() * 4 / 3
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn space_is_the_base_times_the_step() {
        assert_eq!(space(0), 0);
        assert_eq!(space(1), 4);
        assert_eq!(space(3), 12);
        assert_eq!(space(6), 24);
    }

    /// The type scale ascends, with no two steps the same size — a scale that
    /// repeats a value is two names for one thing.
    #[test]
    fn the_type_scale_ascends_without_duplicates() {
        let all = [
            Type::Label.px(),
            Type::Foot.px(),
            Type::Meta.px(),
            Type::Body.px(),
            Type::Lead.px(),
            Type::Title.px(),
        ];
        for w in all.windows(2) {
            assert!(w[0] < w[1], "{:?} and {:?} are not distinct", w[0], w[1]);
        }
    }

    #[test]
    fn gdi_sizes_are_negative_magnitudes() {
        for t in [Type::Label, Type::Foot, Type::Meta, Type::Body, Type::Lead, Type::Title] {
            assert_eq!(t.gdi(), -t.px());
            assert!(t.gdi() < 0, "a positive lfHeight selects cell height, not character height");
        }
    }

    #[test]
    fn every_line_box_leaves_room_under_its_glyphs() {
        for t in [Type::Label, Type::Foot, Type::Meta, Type::Body, Type::Lead, Type::Title] {
            assert!(t.line() > t.px(), "{:?} has no leading", t);
        }
    }
}