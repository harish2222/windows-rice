//! Random motif assignment from the SVG files already on disk.
//!
//! The generated mandala PNGs and their `--motif-mandala:` declarations have
//! both been removed; what the popups paint now is the hand-made SVG motifs
//! that ship in the yasb folder. So this discovers them by globbing
//! `motif-*.svg` rather than from a hard-coded list, which means dropping a
//! new file in is enough to make it eligible.
//!
//! Three are drawn at random and dealt out one per theme block, so switching
//! themes changes the artwork without every theme looking identical. The draw
//! is seeded from the system clock, which is what makes it actually random
//! between runs rather than fixed at compile time.
//!
//! Only the exact `--motif:` line of each block is touched. That exactness is
//! not incidental: `--motif-mandala:` used to share the `--motif` prefix, so a
//! prefix match would have rewritten the wrong variable and pointed a panel at
//! art it was never meant to draw.

use std::path::Path;

/// One discovered motif.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Motif {
    /// File stem, e.g. `motif-paisley`.
    pub stem: String,
    /// Full path, as written into the stylesheet.
    pub path: String,
}

/// Find every `motif-*.svg` in `dir`, sorted by name.
///
/// Sorting is not cosmetic: the shuffle below is a Fisher-Yates over this
/// slice, and a stable order means the same clock seed always deals the same
/// three, so an unsorted read from the filesystem would make the result depend
/// on directory iteration order.
pub fn discover(dir: &Path) -> Vec<Motif> {
    let mut out = Vec::new();
    let Ok(rd) = std::fs::read_dir(dir) else { return out };
    for entry in rd.flatten() {
        let p = entry.path();
        if p.extension().and_then(|e| e.to_str()) != Some("svg") {
            continue;
        }
        let Some(stem) = p.file_stem().and_then(|s| s.to_str()) else { continue };
        if !stem.starts_with("motif-") {
            continue;
        }
        out.push(Motif {
            stem: stem.to_string(),
            // Forward slashes regardless of platform: Qt's stylesheet parser
            // treats a backslash as an escape, so a Windows path written
            // verbatim would silently fail to resolve.
            path: p.to_string_lossy().replace('\\', "/"),
        });
    }
    out.sort_by(|a, b| a.stem.cmp(&b.stem));
    out
}

/// Fisher-Yates over indices, using `seed`.
///
/// Written out rather than pulled from a crate because the whole crate exists
/// to keep the dependency list at `windows` and `serde_json`, and a shuffle is
/// about fifteen lines.
fn shuffle(n: usize, mut seed: u64) -> Vec<usize> {
    let mut v: Vec<usize> = (0..n).collect();
    // xorshift64*: any seed of 0 would be a fixed point, so force it non-zero.
    if seed == 0 {
        seed = 0x9E3779B97F4A7C15;
    }
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for i in (1..n).rev() {
        let j = (next() % (i as u64 + 1)) as usize;
        v.swap(i, j);
    }
    v
}

/// Pick up to `count` motifs at random, without replacement.
pub fn pick(motifs: &[Motif], count: usize, seed: u64) -> Vec<Motif> {
    let order = shuffle(motifs.len(), seed);
    order.into_iter().take(count.min(motifs.len())).map(|i| motifs[i].clone()).collect()
}

/// Rewrite each theme block's `--motif:` to one of `chosen`, cycling when
/// there are fewer motifs than themes.
///
/// Returns how many declarations were rewritten. Anything that is not a
/// `--motif:` declaration is left byte-identical, which is what keeps this
/// safe to run against a hand-edited stylesheet.
pub fn assign(lines: &mut [String], regions: &[crate::Region], chosen: &[Motif]) -> usize {
    if chosen.is_empty() {
        return 0;
    }
    let mut n = 0usize;
    for (i, region) in regions.iter().enumerate() {
        let motif = &chosen[i % chosen.len()];
        for &v in &region.vars {
            let Some(orig) = lines.get(v) else { continue };
            let trimmed = orig.trim();
            // Only the `--motif:` key. `--motif-mandala:` starts with the same
            // characters, so the exact key match is what keeps the two apart.
            let rest = match trimmed.strip_prefix("--motif:") {
                Some(r) => r,
                None => continue,
            };
            if rest.trim_start().starts_with('-') {
                continue;
            }
            lines[v] = format!("    --motif: url(\"{}\");", motif.path);
            n += 1;
        }
    }
    n
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fake(n: usize) -> Vec<Motif> {
        (0..n)
            .map(|i| Motif { stem: format!("motif-{i:02}"), path: format!("C:/m/motif-{i:02}.svg") })
            .collect()
    }

    #[test]
    fn pick_never_repeats_a_motif() {
        let m = fake(12);
        let p = pick(&m, 3, 12345);
        assert_eq!(p.len(), 3);
        let mut stems: Vec<&str> = p.iter().map(|x| x.stem.as_str()).collect();
        stems.sort_unstable();
        stems.dedup();
        assert_eq!(stems.len(), 3, "draw was {stems:?}");
    }

    #[test]
    fn pick_handles_fewer_motifs_than_requested() {
        let m = fake(2);
        assert_eq!(pick(&m, 3, 1).len(), 2);
        assert_eq!(pick(&[], 3, 1).len(), 0);
    }

    #[test]
    fn different_seeds_actually_differ() {
        let m = fake(12);
        let a: Vec<String> = pick(&m, 3, 1).iter().map(|x| x.stem.clone()).collect();
        let b: Vec<String> = pick(&m, 3, 2).iter().map(|x| x.stem.clone()).collect();
        assert_ne!(a, b, "the draw is not varying with the seed");
    }

    #[test]
    fn a_zero_seed_does_not_hang_or_repeat() {
        // xorshift is a fixed point at 0, which would make every draw identical.
        let p = pick(&fake(12), 3, 0);
        assert_eq!(p.len(), 3);
        let mut stems: Vec<&str> = p.iter().map(|x| x.stem.as_str()).collect();
        stems.sort_unstable();
        stems.dedup();
        assert_eq!(stems.len(), 3);
    }

    fn region(name: &str, vars: Vec<usize>) -> crate::Region {
        crate::Region {
            name: name.to_string(),
            header_idx: 0,
            marked_active: false,
            vars,
        }
    }

    #[test]
    fn assign_rewrites_motif_but_never_motif_mandala() {
        let mut lines: Vec<String> = vec![
            "/* c */".into(),
            "    --motif: url(\"C:/old/one.svg\");".into(),
            "    --motif-mandala: url(\"C:/old/one-mandala.png\");".into(),
        ];
        let regions = vec![region("A", vec![1, 2])];
        let n = assign(&mut lines, &regions, &fake(2));
        assert_eq!(n, 1, "only the --motif line counts");
        assert!(lines[1].contains("--motif: url(\"C:/m/motif-"));
        assert_eq!(
            lines[2], "    --motif-mandala: url(\"C:/old/one-mandala.png\");",
            "the mandala declaration must survive untouched"
        );
    }

    #[test]
    fn assign_cycles_when_there_are_fewer_motifs_than_themes() {
        let mut lines: Vec<String> = vec![
            "    --motif: url(\"a.svg\");".into(),
            "    --motif: url(\"b.svg\");".into(),
            "    --motif: url(\"c.svg\");".into(),
        ];
        let regions = vec![region("A", vec![0]), region("B", vec![1]), region("C", vec![2])];
        assign(&mut lines, &regions, &fake(1));
        // One motif, three themes: all three end up identical rather than
        // leaving the last two themes on stale art.
        assert_eq!(lines[0], lines[1]);
        assert_eq!(lines[1], lines[2]);
    }

    #[test]
    fn assign_with_no_motifs_changes_nothing() {
        let mut lines: Vec<String> = vec!["    --motif: url(\"a.svg\");".into()];
        let before = lines.clone();
        assert_eq!(assign(&mut lines, &[region("A", vec![0])], &[]), 0);
        assert_eq!(lines, before);
    }

    #[test]
    fn discovery_is_sorted_and_svg_only() {
        let dir = std::env::temp_dir().join("motif-discover-test");
        let _ = std::fs::create_dir_all(&dir);
        for n in ["motif-zebra.svg", "motif-alpha.svg", "runner-x.svg", "motif-a.png"] {
            std::fs::write(dir.join(n), b"x").unwrap();
        }
        let got = discover(&dir);
        let names: Vec<&str> = got.iter().map(|m| m.stem.as_str()).collect();
        assert_eq!(names, vec!["motif-alpha", "motif-zebra"]);
        assert!(got.iter().all(|m| !m.path.contains('\\')), "paths must use forward slashes");
        let _ = std::fs::remove_dir_all(&dir);
    }
}