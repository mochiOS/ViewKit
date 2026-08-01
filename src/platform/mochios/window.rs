use super::*;

pub(super) struct MochiOsWindow {
    viewport: Cell<Viewport>,
    redraw_requested: Cell<bool>,
    compositor: u64,
    surface: u64,
}

impl MochiOsWindow {
    pub(super) fn new(viewport: Viewport, compositor: u64, surface: u64) -> Self {
        Self {
            viewport: Cell::new(viewport),
            redraw_requested: Cell::new(false),
            compositor,
            surface,
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

    fn show_context_menu(&self, request: &crate::event::ContextMenuRequest) -> bool {
        if request.request_id == 0
            || !request.position.x.is_finite()
            || !request.position.y.is_finite()
            || request.items.is_empty()
            || request.items.len() > 32
        {
            return false;
        }
        let mut wire = Vec::with_capacity(256);
        wire.resize(32, 0);
        wire[0..4].copy_from_slice(&OP_CONTEXT_MENU_SHOW.to_le_bytes());
        wire[4..12].copy_from_slice(&self.surface.to_le_bytes());
        wire[12..20].copy_from_slice(&request.request_id.to_le_bytes());
        wire[20..24].copy_from_slice(&(request.position.x.round() as i32).to_le_bytes());
        wire[24..28].copy_from_slice(&(request.position.y.round() as i32).to_le_bytes());
        wire[28..32].copy_from_slice(&(request.items.len() as u32).to_le_bytes());
        for item in &request.items {
            let label = item.label.as_bytes();
            if label.len() > 128 || (!item.separator && (item.command_id == 0 || label.is_empty()))
            {
                return false;
            }
            let mut flags = 0u16;
            if item.separator {
                flags |= 1 << 0;
            }
            if item.enabled {
                flags |= 1 << 1;
            }
            if item.checked {
                flags |= 1 << 2;
            }
            if item.destructive {
                flags |= 1 << 3;
            }
            wire.extend_from_slice(&item.command_id.to_le_bytes());
            wire.extend_from_slice(&flags.to_le_bytes());
            wire.extend_from_slice(&(label.len() as u16).to_le_bytes());
            wire.extend_from_slice(label);
        }
        if wire.len() > EVENT_BUFFER_SIZE {
            return false;
        }
        let mut reply = [0u8; 16];
        ipc_call_raw(
            self.compositor,
            wire.as_ptr(),
            wire.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )
        .ok()
        .and_then(|len| status_from_raw(reply.as_ptr(), len).ok())
        .is_some()
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
