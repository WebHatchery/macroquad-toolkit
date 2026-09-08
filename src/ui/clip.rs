//! Clip scrolling content without changing the UI or world camera.
use super::VirtualUi;
use macroquad::prelude::*;
use std::cell::Cell;

thread_local! {
    static CLIP: Cell<Option<Rect>> = const { Cell::new(None) };
}

struct RestoreClip(Option<Rect>);
impl Drop for RestoreClip {
    fn drop(&mut self) {
        apply(self.0);
    }
}

fn apply(rect: Option<Rect>) {
    CLIP.with(|clip| clip.set(rect));
    // Macroquad's scissor takes framebuffer pixels with a top-left origin.
    unsafe {
        get_internal_gl()
            .quad_gl
            .scissor(rect.map(|r| (r.x as i32, r.y as i32, r.w as i32, r.h as i32)));
    }
}

impl VirtualUi {
    /// Draw within a UI rectangle, preserving the enclosing toolkit clip.
    /// Input must still be restricted to the same rectangle by the caller.
    pub fn with_clip<R>(&self, rect: Rect, draw: impl FnOnce() -> R) -> R {
        let dpi = screen_dpi_scale();
        let point = self.ui_to_screen(rect.point()) * dpi;
        let size = rect.size() * self.scale * dpi;
        let rect = Rect::new(
            point.x.ceil(),
            point.y.ceil(),
            size.x.floor(),
            size.y.floor(),
        );
        let previous = CLIP.with(Cell::get);
        let clipped = previous.map_or(rect, |parent| parent.intersect(rect).unwrap_or_default());
        apply(Some(clipped));
        let _restore = RestoreClip(previous);
        draw()
    }
}
