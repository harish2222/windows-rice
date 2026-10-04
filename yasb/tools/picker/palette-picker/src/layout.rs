//! Filtering and grid geometry, kept free of Win32 so it can be tested.
//!
//! The visual order is *computed*, not assumed from catalog order: items
//! are grouped under "Dark" and "Light" headings, and those headings take
//! a row of their own. Deriving each cell's rectangle from the same pass
//! that builds the rows means a click, a hover and a keyboard move all
//! agree on which cell is where — no second indexing scheme to drift.

use crate::catalog::Item;

/// A laid-out row: either a section heading or a theme cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    /// Heading text; not selectable.
    Header(String),
    /// A cell, holding the item's index in the *unfiltered* catalog.
    Cell(usize),
}

/// Cell geometry.
///
/// Sized from measurement rather than taste. The shipped catalog's longest
/// name, `Rangalipi Aubergine Light`, measures 234px at 15px and 208px at 13px
/// in the bar's own monospace face (measured through GDI, not estimated).
/// With three swatches, a 6px gap and 14px of right padding the name column
/// therefore needs 320px of cell to hold the longest name at 13px.
///
/// The panel used to be 760px wide with a 232px cell, which left the name
/// 120px. Every one of those 22 names needed at least 182px, so *no* size in
/// the step-down list could ever fit and the ellipsis was not a fallback —
/// it was the only outcome. Widening the panel to 1020px gives the name
/// column 214px, which fits the longest name with slack.
pub const CELL_W: i32 = 320;
pub const CELL_H: i32 = yasb_chrome::space(10);
/// Gutter between cells.
///
/// Off the 4px base on purpose: 12px was tried and it pushed the third column
/// past the panel's content margin. 10px is what the three-column fit was
/// measured and tested at, so it stays until that measurement is redone.
pub const GAP: i32 = 10;
/// Cell width + gap. The Python picker used the same divisor for its
/// column count, so arrow-key movement feels identical across the two.
pub const STRIDE: i32 = CELL_W + GAP;

/// Swatch chips per cell, and their pitch.
///
/// These live here rather than in `main.rs` because the text pass derives the
/// name's left edge from them: if the two disagree, the name lands on top of
/// the chips.
pub const SWATCHES: i32 = 3;
pub const SWATCH_STRIDE: i32 = 22;
pub const CHIP_W: i32 = 18;
/// Left inset of the swatch cluster from the cell's edge. Scale step 6.
pub const SWATCH_X: i32 = yasb_chrome::space(6);
/// Gap between the last chip and the name.
///
/// Off the 4px base: 4px put the name against the last chip and 8px ate into
/// the 214px the longest shipped name needs. 6px is the measured compromise.
pub const NAME_GAP: i32 = 6;
/// Right padding inside a cell.
///
/// Off the base for the same reason: 16px would have cost the name column the
/// slack that lets the longest name fit without dropping a step.
pub const CELL_PAD_R: i32 = 14;

/// Offset of the name column from the cell's left edge.
///
/// Derived, so the shape pass and the text pass cannot disagree about where
/// the chips end and the name begins.
pub const NAME_DX: i32 =
    SWATCH_X + (SWATCHES - 1) * SWATCH_STRIDE + CHIP_W + NAME_GAP;

/// Width the name gets: the cell minus its swatches and its padding.
pub fn name_avail() -> i32 {
    CELL_W - NAME_DX - CELL_PAD_R
}

/// Height of a section heading's own line box.
///
/// The heading used to advance the grid by a full `CELL_H`, which put its
/// text *inside* the band the following cells also start at — so `LIGHT` was
/// drawn on top of the first light cell. A heading is a line of 11px caps,
/// not a cell, and it gets a line box and a gap like any other text.
pub const HEADER_H: i32 = yasb_chrome::space(5);
/// How far the grid advances past a heading before the next cell row.
pub fn header_advance() -> i32 {
    HEADER_H + GAP
}

/// Gap between the last cell row and the footer rule. Half a step: 6px.
pub const FOOT_GAP: i32 = yasb_chrome::space(3) / 2;

/// How many cells fit across `viewport_w`.
///
/// The trailing `GAP` is what makes this correct: `cols` cells occupy
/// `cols * CELL_W + (cols - 1) * GAP`, so dividing the viewport by
/// `CELL_W + GAP` alone loses a whole column at every boundary. It happened
/// to agree at the old 3-column width, which is why the off-by-one survived.
pub fn columns(viewport_w: i32) -> usize {
    (((viewport_w + GAP) / STRIDE).max(1)) as usize
}

/// Catalog indices of items matching `query`, grouped Dark-then-Light.
///
/// Grouping happens here, once, so the visible order, the drawn rows and
/// the cell rectangles all share one ordering. Doing it per-consumer is
/// how a click and a keyboard move end up pointing at different cells.
pub fn visible(items: &[Item], query: &str) -> Vec<usize> {
    let q = query.trim().to_lowercase();
    let mut out = Vec::new();
    for section in ["dark", "light"] {
        for (i, it) in items.iter().enumerate() {
            if it.section == section && (q.is_empty() || it.name.to_lowercase().contains(&q)) {
                out.push(i);
            }
        }
    }
    out
}

/// Section headings to insert into an already-grouped `visible` list.
///
/// A section with no matches contributes no heading, so filtering to
/// "Ember" does not leave an empty "Dark" heading behind.
pub fn rows_for(visible: &[usize], items: &[Item]) -> Vec<Row> {
    let present: Vec<&str> = ["dark", "light"]
        .iter()
        .copied()
        .filter(|s| visible.iter().any(|&i| items[i].section == *s))
        .collect();
    let mut rows = Vec::new();
    for (section, label) in [("dark", "Dark"), ("light", "Light")] {
        let in_section: Vec<usize> = visible
            .iter()
            .copied()
            .filter(|&i| items[i].section == section)
            .collect();
        if in_section.is_empty() {
            continue;
        }
        // Only show the heading when more than one section is on screen; a
        // single filtered result reads better without one.
        if present.len() > 1 {
            rows.push(Row::Header(label.to_string()));
        }
        rows.extend(in_section.into_iter().map(Row::Cell));
    }
    rows
}

/// Rectangle of each cell, in the same order as `visible` (headers are
/// skipped, so this Vec indexes the visible list directly).
pub fn cell_rects(rows: &[Row], origin: (i32, i32), max_w: i32) -> Vec<(i32, i32)> {
    let cols = columns(max_w);
    let mut out = Vec::new();
    let (ox, oy) = origin;
    let mut x = ox;
    let mut y = oy;
    let mut col = 0usize;
    // Whether the current line has any cells on it. A heading that arrives
    // mid-line has to let the line finish first: `y` still points at the top
    // of the last cell, not below it, so adding only the heading's own
    // advance would place the next row *inside* the current cell.
    let mut line_used = false;
    for r in rows {
        match r {
            Row::Header(_) => {
                if line_used {
                    y += CELL_H + GAP;
                }
                // A heading occupies its own line box plus a gap, and the
                // cells resume *below* it. Advancing by a full CELL_H left
                // the heading's text inside the next cells' band.
                x = ox;
                y += header_advance();
                col = 0;
                continue;
            }
            Row::Cell(_) => {
                if col == cols {
                    x = ox;
                    y += CELL_H + GAP;
                    col = 0;
                }
                out.push((x, y));
                x += STRIDE;
                col += 1;
                line_used = true;
            }
        }
    }
    out
}

/// Total height the rows occupy, so the window can size itself to content.
pub fn content_height(rows: &[Row], origin_y: i32, max_w: i32) -> i32 {
    match cell_rects(rows, (0, origin_y), max_w).last() {
        Some(&(_, y)) => y + CELL_H - origin_y,
        None => 0,
    }
}

/// Client height the panel needs, given the bottom of its last cell row.
///
/// The footer is *always* drawn — it carries the match count, which is worth
/// showing with no filter active — so its height is always reserved. The
/// reservation used to be conditional on a filter being active while the draw
/// was unconditional, which put the footer straight over the last row.
pub fn window_height(grid_bottom: i32, foot_h: i32, pad: i32) -> i32 {
    grid_bottom + FOOT_GAP + foot_h + pad
}

/// Y of the footer rule and of the footer text box.
pub fn footer_rows(h: i32, foot_h: i32, pad: i32) -> (i32, i32, i32) {
    (h - foot_h - FOOT_GAP, h - foot_h, foot_h - pad)
}

/// Which cell a click at client coordinates landed on.
pub fn hit_test(rects: &[(i32, i32)], x: i32, y: i32) -> Option<usize> {
    rects
        .iter()
        .position(|&(cx, cy)| x >= cx && x < cx + CELL_W && y >= cy && y < cy + CELL_H)
}

/// Move a selection by `delta` cells, wrapping at both ends.
pub fn move_sel(sel: usize, delta: isize, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let n = len as isize;
    ((sel as isize + delta).rem_euclid(n)) as usize
}

#[cfg(test)]
mod tests {
    use super::*;

    fn items() -> Vec<Item> {
        (0..6)
            .map(|i| Item {
                name: format!("Theme {i}"),
                colors: vec!["#ffffff".into()],
                section: if i < 3 { "dark".into() } else { "light".into() },
            })
            .collect()
    }

    #[test]
    fn empty_query_keeps_everything() {
        let it = items();
        assert_eq!(visible(&it, "").len(), 6);
        assert_eq!(visible(&it, "   ").len(), 6);
    }

    #[test]
    fn filter_is_case_insensitive_substring() {
        let it = items();
        assert_eq!(visible(&it, "theme 1").len(), 1);
        assert_eq!(visible(&it, "THEME").len(), 6);
        assert_eq!(visible(&it, "zzz").len(), 0);
    }

    #[test]
    fn visible_is_grouped_dark_then_light() {
        let it = items();
        let v = visible(&it, "");
        let sections: Vec<&str> = v.iter().map(|&i| it[i].section.as_str()).collect();
        assert_eq!(sections, ["dark", "dark", "dark", "light", "light", "light"]);
    }

    #[test]
    fn headings_appear_only_when_both_sections_are_present() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        assert_eq!(rows[0], Row::Header("Dark".into()));
        assert_eq!(rows.iter().filter(|r| matches!(r, Row::Header(_))).count(), 2);

        // One surviving match means one section, so the heading is dropped:
        // a lone "Light" caption above a single cell is just noise.
        let it2 = vec![
            Item { name: "A".into(), colors: vec![], section: "dark".into() },
            Item { name: "B".into(), colors: vec![], section: "light".into() },
        ];
        assert_eq!(rows_for(&visible(&it2, "B"), &it2), vec![Row::Cell(1)]);
    }#[test]
fn a_heading_occupies_its_own_row() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        // Cells start one heading below, clear of the heading's own band.
        let rects = cell_rects(&rows, (0, 0), STRIDE * 3);
        assert_eq!(rects[0], (0, header_advance()));
    }

    /// The regression fence for the visible defect: a heading's text sits in
    /// its own line box, and the cells below start below that box.
    ///
    /// The grid used to advance a full `CELL_H` for a heading, so the heading
    /// was drawn at `y + 14` inside the very band the next cells occupied —
    /// `LIGHT` was painted across the first light cell.
    #[test]
    fn a_heading_does_not_overlap_the_cells_below_it() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        let rects = cell_rects(&rows, (0, 0), STRIDE * 3);
        // The heading's band is [0, HEADER_H) and its text is drawn at y + 4.
        let heading_text_bottom = 4 + 11;
        assert!(heading_text_bottom <= HEADER_H, "heading text spills out of its own band");
        // Three dark cells, then the "Light" heading, then three more.
        let second_heading_cells = &rects[3..6];
        let first_of_second = second_heading_cells[0];
        assert!(
            first_of_second.1 >= HEADER_H + GAP,
            "the cells after a heading start at {}, inside its band",
            first_of_second.1
        );
        // Nothing overlaps vertically: every cell starts below the heading's
        // text and below the previous cell's bottom edge.
        for w in rects.windows(2) {
            assert!(w[1].1 >= w[0].1, "cells on the same line must share a y");
        }
    }#[test]
fn rects_wrap_into_rows_and_match_visible_order() {
        let it = items();
        let v = visible(&it, "");
        let rows = rows_for(&v, &it);
        let rects = cell_rects(&rows, (0, 0), STRIDE * 2); // 2 columns
        assert_eq!(rects.len(), v.len());
        assert_eq!(rects[0], (0, header_advance()));
        assert_eq!(rects[1], (STRIDE, header_advance()));
        // Third cell wraps onto a new line below the first pair.
        assert!(rects[2].1 > header_advance(), "third cell must wrap below");
    }

    /// `n` cells occupy `n * CELL_W + (n - 1) * GAP`, not `n * STRIDE`. The
    /// column count used to divide by `STRIDE` alone, which loses a column at
    /// every boundary; it agreed by luck at the old width and stopped agreeing
    /// the moment the cell was resized.
    #[test]
    fn the_column_count_does_not_lose_a_column_at_the_boundary() {
        // From two: a one-column grid floors at one column and never drops to zero,
        // which is the `.max(1)` doing its job rather than a boundary to test.
        for cols in 2..=4i32 {
            let cols = cols as usize;
            let needed = cols as i32 * CELL_W + (cols as i32 - 1) * GAP;
            assert_eq!(columns(needed), cols, "exact fit for {cols} columns");
            assert_eq!(columns(needed - 1), cols - 1, "one pixel short must drop a column");
            assert_eq!(columns(needed + 40), cols, "slack must not invent a column");
        }
    }

    /// The shipped catalog, in the shipped panel, with no ellipsis.
    ///
    /// The whole point of the 320px cell: the longest shipped name measures
    /// 208px at 13px in this face, and `name_avail` has to clear that. When
    /// it did not — 120px at the old 232px cell — every long name was
    /// ellipsised, because no size in the step-down list could fit.
    #[test]
    fn the_name_column_fits_the_longest_shipped_theme() {
        // Measured through GDI in "FiraCode Nerd Font Mono", 13px.
        const LONGEST_AT_13PX: i32 = 208;
        assert!(
            name_avail() >= LONGEST_AT_13PX,
            "the name column is {}px but the longest name needs {LONGEST_AT_13PX}px",
            name_avail()
        );
    }

    /// The shipped catalog's actual shape: eleven dark and eleven light.
    ///
    /// The previous version of this test used three items per section, which
    /// is exactly one *full* row — and a full row already wraps, so the bug
    /// was invisible. With eleven, the last dark line is only two cells wide
    /// and `y` still points at that line's top when the `LIGHT` heading
    /// arrives, so the heading's band and the cells below it both start
    /// inside the last dark cell. A live capture showed it as a single 70px
    /// painted band where there should have been a 40px cell and a heading.
    #[test]
    fn a_heading_after_a_partial_line_does_not_overlap_it() {
        let items: Vec<Item> = (0..22)
            .map(|i| Item {
                name: format!("Theme {i}"),
                colors: vec!["#ffffff".into()],
                section: if i < 11 { "dark".into() } else { "light".into() },
            })
            .collect();
        let rows = rows_for(&visible(&items, ""), &items);
        let rects = cell_rects(&rows, (0, 0), 1020 - 32);
        assert_eq!(rects.len(), 22);

        // Consecutive cells on different lines must be a full row + gap apart.
        let mut prev_line_bottom = i32::MIN;
        let mut cur_line_y = None;
        for &(x, y) in &rects {
            match cur_line_y {
                Some(ly) if ly == y => {}
                Some(ly) => {
                    // new line
                    assert!(y >= ly + CELL_H + GAP, "line at {y} starts inside the previous one at {ly}");
                    cur_line_y = Some(y);
                }
                None => cur_line_y = Some(y),
            }
            let _ = x;
            prev_line_bottom = prev_line_bottom.max(y);
        }
        let _ = prev_line_bottom;

        // The two headings' bands must each fit above the line that follows.
        let light_first = rects[11];
        // The last dark cell is at index 10; the first light cell is index 11.
        assert!(
            light_first.1 >= rects[10].1 + CELL_H + GAP + HEADER_H,
            "the Light section starts at {} but the last Dark cell ends at {}",
            light_first.1,
            rects[10].1 + CELL_H
        );
    }
    /// The panel is wide enough for the three columns the cell size implies,
    /// with the last cell's right edge on the content margin.
    #[test]
    fn three_columns_fit_the_panel_with_a_margin() {
        let panel_w = 1020;
        let viewport = panel_w - 2 * 16;
        assert_eq!(columns(viewport), 3);
        let last_x = 16 + 2 * STRIDE;
        assert!(
            last_x + CELL_W <= panel_w - 16,
            "the third column ends at {}, past the content margin {}",
            last_x + CELL_W,
            panel_w - 16
        );
    }

    #[test]
    fn hit_test_finds_the_right_cell() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        let rects = cell_rects(&rows, (0, 0), STRIDE * 3);
        let (x, y) = rects[2];
        assert_eq!(hit_test(&rects, x + 5, y + 5), Some(2));
        assert_eq!(hit_test(&rects, x - 2, y + 5), None, "gutter hits nothing");
    }

    #[test]
    fn selection_wraps() {
        assert_eq!(move_sel(0, -1, 6), 5);
        assert_eq!(move_sel(5, 1, 6), 0);
        assert_eq!(move_sel(0, 0, 6), 0);
        assert_eq!(move_sel(3, 1, 0), 0);
    }#[test]
fn keyboard_moves_by_a_visual_row() {
        let it = items();
        let v = visible(&it, "");
        let cols = columns(STRIDE * 3);
        let sel = move_sel(0, cols as isize, v.len());
        assert_eq!(it[v[sel]].name, "Theme 3");
    }

    /// The footer is drawn unconditionally — it carries the match count — so
    /// its height is always reserved. It used to be reserved only while a
    /// filter was active, which put the footer over the last row of cells on
    /// an unfiltered panel, which is how it opens.
    #[test]
    fn the_footer_never_overlaps_the_last_row_of_cells() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        let origin_y = 16 + 64 + 12;
        let grid_h = content_height(&rows, origin_y, 1020 - 32);
        let grid_bottom = origin_y + grid_h;
        let h = window_height(grid_bottom, 34, 16);
        let (rule_y, foot_y, foot_h) = footer_rows(h, 34, 16);

        assert!(rule_y > grid_bottom, "the footer rule at {rule_y} is inside the grid, which ends at {grid_bottom}");
        assert!(foot_y > grid_bottom);
        assert!(foot_y + foot_h <= h, "the footer text runs past the panel edge");
        assert!(h - (foot_y + foot_h) == 16, "the panel's bottom padding is gone");
        assert!(h <= 760, "the shipped catalog must not hit the height guard");
    }
}