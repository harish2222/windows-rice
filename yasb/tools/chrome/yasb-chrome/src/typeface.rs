//! One type scale, shared by every panel, derived from the bar's own font.
//!
//! # The problem this exists to solve
//!
//! The bar renders its text in whatever `--system-font` the active theme names.
//! The panels used to hardcode `Segoe UI` / `Segoe UI Variable Display`, so on
//! a theme whose bar font is a Nerd Font the bar and its popup were visibly
//! two different typefaces sitting a few centimetres apart. That was the
//! single most obvious thing wrong with the panels, and it was invisible in
//! the source because both constants looked perfectly reasonable on their own.
//!
//! So the family is *read from the stylesheet* rather than hardcoded, the same
//! way every colour in these panels already is. Switch the bar's font and the
//! panels follow it.
//!
//! # Why the fallback chain matters
//!
//! `--system-font` names a font that may not be installed — the stylesheet is
//! portable and this machine is not necessarily the machine it was written on.
//! GDI does not do CSS-style fallback lists, so a missing family silently
//! becomes GDI's default face, which is what produced the mismatch in the
//! first place. [`Typeface::resolve`] therefore checks each candidate with
//! `EnumFontFamiliesExW` and picks the first one that actually exists, so a
//! theme naming an absent font degrades to Segoe UI on purpose rather than by
//! accident.
//!
//! # Telugu
//!
//! Nerd Fonts carry no Brahmic glyphs, so a Telugu label in the bar's family
//! renders as tofu. Windows ships Nirmala UI for exactly this. Telugu text
//! therefore always uses [`INDIC_FAMILY`]; only the Latin portions of the
//! panel follow `--system-font`. Mixing the two mid-string is what produced
//! the half-rendered Telugu in an earlier build.

use windows::Win32::Graphics::Gdi::{
    CreateCompatibleDC, CreateFontW, DeleteDC, DeleteObject, GetDC, GetTextFaceW, HFONT,
    OUT_DEFAULT_PRECIS, ReleaseDC, SelectObject,
};
use windows::core::PCWSTR;

/// Windows ships Nirmala UI for Brahmic scripts; Telugu text uses it.
pub const INDIC_FAMILY: &str = "Nirmala UI";
/// Used when the bar's family is not installed on this machine.
pub const FALLBACK_FAMILY: &str = "Segoe UI";
/// Headings, when the bar's family has no display cut.
pub const FALLBACK_DISPLAY: &str = "Segoe UI Variable Display";

/// The bar's own family for the active theme, plus a resolved fallback.
///
/// Plain data, so it can cross the worker thread boundary into the paint pass
/// without dragging anything thread-affine along.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Typeface {
    /// Latin body and heading text, already resolved to something installed.
    pub family: String,
    /// Latin headings. Same as `family` unless a display cut was asked for.
    pub display: String,
    /// Brahmic text. Always Nirmala UI, since Nerd Fonts have no Telugu.
    pub indic: String,
}

impl Default for Typeface {
    fn default() -> Self {
        Typeface {
            family: FALLBACK_FAMILY.to_string(),
            display: FALLBACK_DISPLAY.to_string(),
            indic: INDIC_FAMILY.to_string(),
        }
    }
}

impl Typeface {
    /// Resolve the bar's `--system-font` into a family that exists here.
    ///
    /// `system_font` is the raw `--system-font` value from `styles.css`,
    /// already stripped of quotes by the caller. An empty or uninstallable
    /// value falls back to Segoe UI rather than to GDI's default.
    pub fn resolve(system_font: Option<&str>) -> Typeface {
        let Some(raw) = system_font.map(str::trim).filter(|s| !s.is_empty()) else {
            return Typeface::default();
        };
        // A CSS font stack may list fallbacks separated by commas. Take the
        // first installed one, which is what the browser would do.
        for candidate in raw.split(',').map(str::trim).filter(|s| !s.is_empty()) {
            if font_is_installed(candidate) {
                return Typeface {
                    family: candidate.to_string(),
                    // Nerd Fonts ship a display cut under a suffixed name
                    // ("FiraCode Nerd Font" vs "FiraCode Nerd Font Display").
                    // Asking for a name that does not exist silently falls back
                    // inside GDI, so headings just use the same family as the
                    // body — one typeface, which was the goal.
                    display: candidate.to_string(),
                    indic: INDIC_FAMILY.to_string(),
                };
            }
        }
        Typeface::default()
    }

    /// The family for a script, given whether the panel is in Telugu.
    ///
    /// Telugu switches the whole panel to Nirmala, because a row's label and
    /// value are read as a pair and setting one in a Latin face and the other
    /// in a Brahmic one looks like a bug even when both are correct.
    pub fn body(&self, telugu: bool) -> &str {
        if telugu { &self.indic } else { &self.family }
    }

    /// The heading family for a script.
    pub fn heading(&self, telugu: bool) -> &str {
        if telugu { &self.indic } else { &self.display }
    }
}

/// Is `family` installed?
///
/// `EnumFontFamiliesExW` with a null family enumerates everything installed,
/// which is a few hundred entries on a typical machine. This runs once per
/// theme load on the worker thread, not per frame, so the cost does not matter;
/// caching would be more code than the problem deserves.
fn font_is_installed(family: &str) -> bool {
    // The obvious implementation is `EnumFontFamiliesExW` with the face name
    // in a LOGFONTW, and it does not work: that function ignores the requested
    // face and enumerates *every* installed family, so the "is it installed?"
    // question answers yes unconditionally. Two tests caught it, which is why
    // this is written the roundabout way.
    //
    // Instead: ask GDI to build the font, then ask GDI which face it actually
    // selected. If the request was satisfiable, `GetTextFaceW` reports the
    // family back. If not, GDI silently substitutes and reports the substitute
    // — which is precisely the behaviour that produced the original mismatch,
    // so detecting it here is the whole point.
    let hf = ui_font(-16, 400, family);
    let mut buf = [0u16; 64];
    let selected = unsafe {
        let dc = GetDC(None);
        let mem = CreateCompatibleDC(dc);
        if mem.0.is_null() {
            let _ = ReleaseDC(None, dc);
            let _ = DeleteObject(HFONT(hf.0));
            return false;
        }
        let old = SelectObject(mem, HFONT(hf.0));
        let n = GetTextFaceW(mem, Some(&mut buf));
        let face = if n > 0 {
            let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
            String::from_utf16_lossy(&buf[..end])
        } else {
            String::new()
        };
        SelectObject(mem, old);
        let _ = DeleteDC(mem);
        let _ = ReleaseDC(None, dc);
        let _ = DeleteObject(HFONT(hf.0));
        face
    };

    // GDI reports the face as the user wrote it, but compares
    // case-insensitively, so do the same.
    selected.eq_ignore_ascii_case(family)
}

/// Build a UI font at a given pixel height, weight and family.
///
/// `CLEARTYPE_QUALITY` is deliberate. The panels blit a DIB they composed
/// themselves rather than letting GDI draw into the window, and ClearType's
/// subpixel rendering assumes an opaque white backdrop — over the panel's own
/// dark acrylic it fringes. So the panels ask for `ANTIALIASED_QUALITY`, which
/// is what a compositor does when text lands on a surface it does not control.
///
/// GDI has no per-font DPI here because these windows are created without a
/// DPI-awareness manifest, so they render in the system scale's virtual pixels;
/// the pixel heights below are therefore already in the right units.
pub fn ui_font(size: i32, weight: i32, family: &str) -> windows::Win32::Graphics::Gdi::HFONT {
    let wide: Vec<u16> = family.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        CreateFontW(
            size, 0, 0, 0, weight, 0, 0, 0,
            1u32, // ANSI_CHARSET
            OUT_DEFAULT_PRECIS.0 as u32,
            windows::Win32::Graphics::Gdi::CLIP_DEFAULT_PRECIS.0 as u32,
            windows::Win32::Graphics::Gdi::ANTIALIASED_QUALITY.0 as u32,
            windows::Win32::Graphics::Gdi::DEFAULT_PITCH.0 as u32,
            PCWSTR(wide.as_ptr()),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Segoe UI is on every Windows install, so a resolved family that is not
    /// Segoe must have come from the stylesheet rather than the fallback.
    #[test]
    fn a_missing_family_falls_back_rather_than_being_taken_on_trust() {
        let t = Typeface::resolve(Some("Definitely Not Installed Nerd Font"));
        assert_eq!(t.family, FALLBACK_FAMILY);
        assert_eq!(t.display, FALLBACK_DISPLAY);
    }

    #[test]
    fn an_absent_value_falls_back() {
        for input in [None, Some(""), Some("   ")] {
            assert_eq!(Typeface::resolve(input).family, FALLBACK_FAMILY);
        }
    }

    /// The whole point of the module: the bar's font wins when it is
    /// installed, and Nerd Fonts are installed per-user on this machine.
    #[test]
    fn the_bars_own_family_is_used_when_it_is_installed() {
        let t = Typeface::resolve(Some("Segoe UI"));
        assert_eq!(t.family, "Segoe UI");
        // One typeface, not a stack: headings do not silently switch cut.
        assert_eq!(t.display, t.family, "headings must not use a second family");
    }

    /// A CSS stack is comma-separated; the first *installed* entry wins,
    /// which is what a browser would do. Falling back on the first entry
    /// regardless of whether it exists is the bug this replaces.
    #[test]
    fn a_font_stack_takes_the_first_installed_entry() {
        let t = Typeface::resolve(Some("No Such Font, Segoe UI, Also Missing"));
        assert_eq!(t.family, "Segoe UI");
    }

    /// Telugu must never fall back to a Nerd Font, which has no Brahmic
    /// glyphs and would render the script as tofu.
    #[test]
    fn telugu_always_uses_a_script_capable_face() {
        let t = Typeface::resolve(Some("Segoe UI"));
        assert_eq!(t.body(true), INDIC_FAMILY);
        assert_eq!(t.heading(true), INDIC_FAMILY);
        // ...while Latin keeps following the bar.
        assert_eq!(t.body(false), "Segoe UI");
    }

    /// The families this machine can actually render, checked against the real
    /// font table rather than assumed.
    ///
    /// These were measured with `GetTextFaceW`, which reports the substitute
    /// GDI picked when a family is absent. The result was not what the
    /// filenames suggested: `JetBrainsMono Nerd Font` and
    /// `CaskaydiaCove Nerd Font` are both present on disk and registered, yet
    /// GDI substitutes **Arial** for them, while `FiraCode Nerd Font Mono` —
    /// the family the active theme actually asks for — resolves correctly.
    ///
    /// That gap is the original bug in one line: the panels used to render in
    /// Segoe UI while the bar drew in a family GDI could not supply either, so
    /// the two were mismatched for a reason nobody could see in the source.
    #[test]
    fn the_families_this_machine_can_render_resolve_to_themselves() {
        for family in [
            "FiraCode Nerd Font Mono",
            "FiraCode Nerd Font",
            "Mononoki Nerd Font",
            "Hack Nerd Font",
            "Segoe UI",
        ] {
            let t = Typeface::resolve(Some(family));
            assert_eq!(
                t.family, family,
                "{family} is installed but did not resolve to itself"
            );
        }
    }

    /// A family GDI cannot supply must not be used, because GDI would quietly
    /// substitute Arial and the panel would render in a third typeface.
    #[test]
    fn a_family_gdi_would_substitute_is_rejected() {
        // Pinned to what `GetTextFaceW` actually reports for these: Arial.
        for family in ["JetBrainsMono Nerd Font", "CaskaydiaCove Nerd Font"] {
            let t = Typeface::resolve(Some(family));
            assert_eq!(
                t.family, FALLBACK_FAMILY,
                "{family} is not renderable by GDI and must fall back, not \
                 silently become Arial"
            );
        }
    }
}