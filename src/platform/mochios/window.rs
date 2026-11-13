use super::*;

pub(super) struct MochiOsWindow {
    id: crate::app::WindowId,
    viewport: Cell<Viewport>,
    redraw_requested: Cell<bool>,
    compositor: u64,
    surface: u64,
    input_regions: std::cell::RefCell<Vec<Rect>>,
    pub(super) input_serial: Cell<u64>,
    pub(super) decoration_layout: Cell<crate::decoration::DecorationLayout>,
}

impl MochiOsWindow {
    pub(super) fn new(
        id: crate::app::WindowId,
        viewport: Viewport,
        compositor: u64,
        surface: u64,
    ) -> Self {
        Self {
            id,
            viewport: Cell::new(viewport),
            redraw_requested: Cell::new(false),
            compositor,
            surface,
            input_regions: std::cell::RefCell::new(Vec::new()),
            input_serial: Cell::new(0),
            decoration_layout: Cell::new(crate::decoration::DecorationLayout::default()),
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

    pub(super) fn set_compositor_title(&self, title: &str) -> Result<(), MochiOsBackendError> {
        let mut title_len = title.len().min(MAX_WINDOW_TITLE_BYTES);
        while !title.is_char_boundary(title_len) {
            title_len -= 1;
        }
        let title = &title.as_bytes()[..title_len];
        let mut request = Vec::with_capacity(16 + title.len());
        request.extend_from_slice(&OP_SET_TITLE.to_le_bytes());
        request.extend_from_slice(&self.surface.to_le_bytes());
        request.extend_from_slice(&(title.len() as u32).to_le_bytes());
        request.extend_from_slice(title);
        let mut reply = [0u8; 16];
        let len = ipc_call_raw(
            self.compositor,
            request.as_ptr(),
            request.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )?;
        status_from_raw(reply.as_ptr(), len)
    }

    pub(super) fn set_input_regions(&self, regions: &[Rect]) -> Result<(), MochiOsBackendError> {
        let scale = renderer::valid_scale_factor(self.viewport.get().scale_factor);
        let regions: Vec<_> = regions
            .iter()
            .map(|rect| {
                Rect::new(
                    (rect.origin.x * scale).floor(),
                    (rect.origin.y * scale).floor(),
                    ((rect.origin.x + rect.size.width) * scale).ceil()
                        - (rect.origin.x * scale).floor(),
                    ((rect.origin.y + rect.size.height) * scale).ceil()
                        - (rect.origin.y * scale).floor(),
                )
            })
            .collect();
        if *self.input_regions.borrow() == regions {
            return Ok(());
        }
        let mut request = Vec::with_capacity(16 + regions.len() * 16);
        request.extend_from_slice(&OP_SET_INPUT_REGIONS.to_le_bytes());
        request.extend_from_slice(&self.surface.to_le_bytes());
        request.extend_from_slice(&(regions.len() as u32).to_le_bytes());
        for rect in &regions {
            request.extend_from_slice(&(rect.origin.x as i32).to_le_bytes());
            request.extend_from_slice(&(rect.origin.y as i32).to_le_bytes());
            request.extend_from_slice(&(rect.size.width as u32).to_le_bytes());
            request.extend_from_slice(&(rect.size.height as u32).to_le_bytes());
        }
        let mut reply = [0u8; 16];
        let len = ipc_call_raw(
            self.compositor,
            request.as_ptr(),
            request.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )?;
        status_from_raw(reply.as_ptr(), len)?;
        *self.input_regions.borrow_mut() = regions;
        Ok(())
    }
}

impl PlatformWindow for MochiOsWindow {
    fn decoration_layout(&self) -> Option<crate::decoration::DecorationLayout> {
        Some(self.decoration_layout.get())
    }
    fn input_serial(&self) -> u64 {
        self.input_serial.get()
    }
    fn configure_decoration(
        &self,
        config: crate::decoration::DecorationConfig,
    ) -> Result<crate::decoration::DecorationLayout, crate::decoration::DecorationError> {
        use crate::decoration::*;
        let scale = self.viewport.get().scale_factor as f32;
        let mut request = [0u8; 32];
        let mut reply = [0u8; 16];
        request[..4].copy_from_slice(&14u32.to_le_bytes());
        request[4..12].copy_from_slice(&self.surface.to_le_bytes());
        request[12..16]
            .copy_from_slice(&(u32::from(config.mode == DecorationMode::Integrated)).to_le_bytes());
        request[16..20].copy_from_slice(&((config.height * scale).ceil() as u32).to_le_bytes());
        let c = config.background;
        let packed = (u32::from(c.red) << 24)
            | (u32::from(c.green) << 16)
            | (u32::from(c.blue) << 8)
            | u32::from(c.alpha);
        request[20..24].copy_from_slice(&packed.to_le_bytes());
        request[24..28]
            .copy_from_slice(&((scale * metrics::SCALE_UNIT as f32).round() as u32).to_le_bytes());
        request[28..32].copy_from_slice(
            &(u32::from(config.separator) | (u32::from(config.resizable) << 1)).to_le_bytes(),
        );
        let len = ipc_call_raw(
            self.compositor,
            request.as_ptr(),
            request.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )
        .map_err(|_| DecorationError::Rejected)?;
        status_from_raw(reply.as_ptr(), len).map_err(|_| DecorationError::Rejected)?;
        if len != 16 {
            return Err(DecorationError::Rejected);
        }
        let layout = DecorationLayout {
            height: u32::from_le_bytes(reply[4..8].try_into().unwrap()) as f32 / scale,
            controls_width: u32::from_le_bytes(reply[8..12].try_into().unwrap()) as f32 / scale,
            state: WindowState::from_raw(u32::from_le_bytes(reply[12..16].try_into().unwrap()))
                .ok_or(DecorationError::Rejected)?,
        };
        self.decoration_layout.set(layout);
        Ok(layout)
    }
    fn begin_window_interaction(
        &self,
        serial: u64,
        operation: crate::decoration::WindowInteraction,
    ) -> bool {
        if serial == 0 || self.input_serial.replace(0) != serial {
            return false;
        }
        let mut request = [0u8; 28];
        let mut reply = [0u8; 16];
        let (opcode, edge) = match operation {
            crate::decoration::WindowInteraction::Move => (105u32, 0u32),
            crate::decoration::WindowInteraction::Resize(edge) => (106, edge as u32),
        };
        request[..4].copy_from_slice(&opcode.to_le_bytes());
        request[4..12].copy_from_slice(&self.surface.to_le_bytes());
        request[12..20].copy_from_slice(&serial.to_le_bytes());
        request[20..24].copy_from_slice(&edge.to_le_bytes());
        ipc_call_raw(
            self.compositor,
            request.as_ptr(),
            request.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )
        .is_ok_and(|len| status_from_raw(reply.as_ptr(), len).is_ok())
    }

    fn id(&self) -> crate::app::WindowId {
        self.id
    }

    fn request_redraw(&self) {
        self.redraw_requested.set(true);
    }

    fn set_title(&self, title: &str) {
        let _ = self.set_compositor_title(title);
    }

    fn activate(&self) {
        let _ = simple_token_request(self.compositor, OP_ACTIVATE_SURFACE, self.surface);
    }

    fn set_keyboard_capture(&self, captured: bool) -> bool {
        let mut request = [0u8; 16];
        request[..4].copy_from_slice(&OP_SET_KEYBOARD_CAPTURE.to_le_bytes());
        request[4..12].copy_from_slice(&self.surface.to_le_bytes());
        request[12..].copy_from_slice(&u32::from(captured).to_le_bytes());
        let mut reply = [0u8; 16];
        ipc_call_raw(
            self.compositor,
            request.as_ptr(),
            request.len(),
            reply.as_mut_ptr(),
            reply.len(),
        )
        .and_then(|len| status_from_raw(reply.as_ptr(), len))
        .is_ok()
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
        let scale = self.viewport.get().scale_factor as f32;
        wire[0..4].copy_from_slice(&OP_CONTEXT_MENU_SHOW.to_le_bytes());
        wire[4..12].copy_from_slice(&self.surface.to_le_bytes());
        wire[12..20].copy_from_slice(&request.request_id.to_le_bytes());
        wire[20..24].copy_from_slice(&((request.position.x * scale).round() as i32).to_le_bytes());
        wire[24..28].copy_from_slice(&((request.position.y * scale).round() as i32).to_le_bytes());
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
