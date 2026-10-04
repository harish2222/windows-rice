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

/// Cell geometry. Wider and taller than a compact dmenu grid on purpose: the
/// panel is read at a glance, and a 178x28 box crams 22 themes into a strip
/// that needs no looking at but is hard to *choose* from. 232x40 fits the
/// longest shipped name ("Rangalipi Mossfern Light") at body size next to its
/// swatches, so the name never has to be shrunk to fit.
pub const CELL_W: i32 = 232;
pub const CELL_H: i32 = 40;
pub const GAP: i32 = 10;
/// Cell width + gap. The Python picker used the same divisor for its
/// column count, so arrow-key movement feels identical across the two.
pub const STRIDE: i32 = CELL_W + GAP;

/// How many cells fit across `viewport_w`.
pub fn columns(viewport_w: i32) -> usize {
    ((viewport_w / STRIDE).max(1)) as usize
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
    for r in rows {
        match r {
            Row::Header(_) => {
                // A heading occupies its own full-width row.
                x = ox;
                y += CELL_H;
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
    }

    #[test]
    fn a_heading_occupies_its_own_row() {
        let it = items();
        let rows = rows_for(&visible(&it, ""), &it);
        // Cells start one row down, below the "Dark" heading.
        let rects = cell_rects(&rows, (0, 0), STRIDE * 3);
        assert_eq!(rects[0], (0, CELL_H));
    }

    #[test]
    fn rects_wrap_into_rows_and_match_visible_order() {
        let it = items();
        let v = visible(&it, "");
        let rows = rows_for(&v, &it);
        let rects = cell_rects(&rows, (0, 0), STRIDE * 2); // 2 columns
        assert_eq!(rects.len(), v.len());
        assert_eq!(rects[0], (0, CELL_H));
        assert_eq!(rects[1], (STRIDE, CELL_H));
        // Third cell wraps onto a new line below the first pair.
        assert!(rects[2].1 > CELL_H, "third cell must wrap below");
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
    }

    #[test]
    fn keyboard_moves_by_a_visual_row() {
        let it = items();
        let v = visible(&it, "");
        let cols = columns(STRIDE * 3);
        let sel = move_sel(0, cols as isize, v.len());
        assert_eq!(it[v[sel]].name, "Theme 3");
    }
}