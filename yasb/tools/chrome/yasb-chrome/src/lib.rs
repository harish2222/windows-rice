//! Shared window chrome for the YASB bar's popups.
//!
//! Both `saka-popup` and `palette-picker` are the same kind of program: one
//! frameless, always-on-top window, drawn by hand, with no widget toolkit. The
//! two things they both need — anti-aliased shapes and themed text on top of
//! them — used to be copy-pasted between them, which is how a rounding radius
//! or a shadow radius ends up subtly different in two panels. They live here
//! instead.
//!
//! The split is deliberate:
//!
//! * [`Canvas`] is pure buffer maths over a 32-bit BGRA pixel buffer. No GDI,
//!   no window, no display. Every primitive is unit-tested by checking the
//!   pixels it wrote.
//! * [`gdi`] owns the display-side plumbing: the DIB section, the HDC that
//!   text is drawn through, the backdrop snapshot that gives the panels their
//!   acrylic look, and the blit onto the window.
//!
//! Both popups composite the same way, and the reason is worth recording:
//! `UpdateLayeredWindow` on a `WS_EX_LAYERED` window *reports* success and puts
//! nothing on screen on this machine (see `saka-popup`'s module docs), so
//! translucency is faked the way a compositor would do it — snapshot the
//! desktop behind the window once, alpha-blend the theme over it, and blit the
//! result. See [`gdi::capture_backdrop`].

pub mod canvas;
pub mod gdi;
pub mod singleton;
pub mod typeface;

pub use canvas::{Canvas, moon_lit_mask};
pub use gdi::{
    Backdrop, Dib, apply_round_region, blit_to_window, colorref, draw_text, font, rect, styles_path,
    text_width,
};
pub use singleton::{Action, acquire, notify_window_of_class, raise_window_of_class};
pub use typeface::{INDIC_FAMILY, Typeface};
