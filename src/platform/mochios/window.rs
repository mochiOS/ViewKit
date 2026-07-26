use super::*;

pub(super) struct MochiOsWindow {
    viewport: Cell<Viewport>,
    redraw_requested: Cell<bool>,
}

impl MochiOsWindow {
    pub(super) fn new(viewport: Viewport) -> Self {
        Self {
            viewport: Cell::new(viewport),
            redraw_requested: Cell::new(false),
        }
    }

    pub(super) const fn width(&self) -> u32 {
        self.viewport.get().physical_width
    }

    pub(super) const fn height(&self) -> u32 {
        self.viewport.get().physical_height
    }

    pub(super) fn take_redraw_requested(&self) -> bool {
        self.redraw_requested.replace(false)
    }

    pub(super) fn set_viewport(&self, viewport: Viewport) {
        self.viewport.set(viewport);
    }
}

impl PlatformWindow for MochiOsWindow {
    fn request_redraw(&self) {
        self.redraw_requested.set(true);
    }

    fn set_title(&self, title: &str) {
        let _ = title;
    }

    fn viewport(&self) -> Viewport {
        self.viewport.get()
    }

    fn set_cursor(&self, cursor: CursorIcon) {
        let _ = cursor;
    }
}

pub(super) fn checked_surface_size(
    size: crate::geometry::Size,
) -> Result<(u32, u32), MochiOsBackendError> {
    if !size.width.is_finite() || !size.height.is_finite() {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }

    let width = size.width.round();
    let height = size.height.round();

    if width < 1.0
        || height < 1.0
        || width > MAX_SURFACE_EXTENT as f32
        || height > MAX_SURFACE_EXTENT as f32
    {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }

    Ok((width as u32, height as u32))
}
