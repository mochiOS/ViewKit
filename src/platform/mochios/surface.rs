use super::*;

use super::connection::{
    ipc_call_raw, put_u32_raw, put_u64_raw, read_u64_raw, status_from_raw, zero_raw,
};
use super::present::physical_dirty_rect;

pub(super) struct CompositorSurface {
    compositor: u64,
    token: u64,
}

impl CompositorSurface {
    pub(super) fn create(
        compositor: u64,
        event_endpoint: u64,
        role: u32,
        width: u32,
        height: u32,
    ) -> Result<Self, MochiOsBackendError> {
        let token = create_surface(compositor, event_endpoint, role, width, height)?;
        Ok(Self { compositor, token })
    }

    pub(super) const fn token(&self) -> u64 {
        self.token
    }
}

impl Drop for CompositorSurface {
    fn drop(&mut self) {
        let _ = simple_token_request(self.compositor, OP_DESTROY_SURFACE, self.token);
    }
}

fn create_surface(
    compositor: u64,
    event_endpoint: u64,
    role: u32,
    width: u32,
    height: u32,
) -> Result<u64, MochiOsBackendError> {
    let request = core::ptr::addr_of_mut!(CREATE_SURFACE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 24);
        put_u32_raw(request, 0, OP_CREATE_SURFACE);
        put_u32_raw(request, 4, role);
        put_u32_raw(request, 8, width);
        put_u32_raw(request, 12, height);
        put_u64_raw(request, 16, event_endpoint);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 24, reply, 16)?;
    if len < 12 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    status_from_raw(reply, len)?;
    Ok(unsafe { read_u64_raw(reply, 4) })
}

pub(super) fn attach_buffer(
    compositor: u64,
    token: u64,
    width: usize,
    height: usize,
    pixmap: &Pixmap,
    background: Color,
    shared_buffer: &mut SharedBuffer,
    viewport: Viewport,
    dirty_bounds: Rect,
    format: u32,
) -> Result<(), MochiOsBackendError> {
    let pixel_count = width
        .checked_mul(height)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    let pixmap_pixel_count = (pixmap.width() as usize)
        .checked_mul(pixmap.height() as usize)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    if pixmap.width() as usize != width || pixmap.height() as usize != height {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }
    if pixmap_pixel_count < pixel_count {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }
    if !shared_buffer.is_attached() {
        let request = core::ptr::addr_of_mut!(ATTACH_BUFFER_REQ).cast::<u8>();
        let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
        unsafe {
            zero_raw(request, 28);
            put_u32_raw(request, 0, OP_ATTACH_BUFFER);
            put_u64_raw(request, 4, token);
            put_u32_raw(request, 12, width as u32);
            put_u32_raw(request, 16, height as u32);
            put_u32_raw(request, 20, width as u32);
            put_u32_raw(request, 24, format);
            zero_raw(reply, 16);
        }
        let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
        status_from_raw(reply, len)?;
        shared_buffer.mark_attached();
    }
    let dirty_rect = physical_dirty_rect(viewport, dirty_bounds);
    shared_buffer.send_pixmap_to(compositor, pixmap, background, dirty_rect, format)
}

pub(super) fn attach_gpu_scene(
    compositor: u64,
    token: u64,
    width: usize,
    height: usize,
    scene: &[u8],
    shared_buffer: &mut SharedBuffer,
) -> Result<(), MochiOsBackendError> {
    if !shared_buffer.is_attached() {
        let request = core::ptr::addr_of_mut!(ATTACH_BUFFER_REQ).cast::<u8>();
        let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
        unsafe {
            zero_raw(request, 28);
            put_u32_raw(request, 0, OP_ATTACH_BUFFER);
            put_u64_raw(request, 4, token);
            put_u32_raw(request, 12, width as u32);
            put_u32_raw(request, 16, height as u32);
            put_u32_raw(request, 20, width as u32);
            put_u32_raw(request, 24, PIXEL_FORMAT_GPU_SCENE);
            zero_raw(reply, 16);
        }
        let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
        status_from_raw(reply, len)?;
        shared_buffer.mark_attached();
    }
    shared_buffer.send_scene_to(compositor, scene)
}

pub(super) fn renderer_caps(compositor: u64) -> u32 {
    let mut request = [0u8; 4];
    request.copy_from_slice(&OP_GET_RENDERER_CAPS.to_le_bytes());
    let mut reply = [0u8; 16];
    let Ok(length) = ipc_call_raw(
        compositor,
        request.as_ptr(),
        request.len(),
        reply.as_mut_ptr(),
        reply.len(),
    ) else {
        return 0;
    };
    if length < 8 || u32::from_le_bytes([reply[0], reply[1], reply[2], reply[3]]) != 0 {
        return 0;
    }
    u32::from_le_bytes([reply[4], reply[5], reply[6], reply[7]])
}

pub(super) fn simple_token_request(
    compositor: u64,
    opcode: u32,
    token: u64,
) -> Result<(), MochiOsBackendError> {
    let request = core::ptr::addr_of_mut!(TOKEN_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 12);
        put_u32_raw(request, 0, opcode);
        put_u64_raw(request, 4, token);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 12, reply, 16)?;
    status_from_raw(reply, len)
}

pub(super) fn set_cursor_position(
    compositor: u64,
    x: f32,
    y: f32,
    visible: bool,
) -> Result<(), MochiOsBackendError> {
    let mut request = [0u8; 16];
    request[0..4].copy_from_slice(&OP_SET_CURSOR_POSITION.to_le_bytes());
    request[4..8].copy_from_slice(&(x.round() as i32).to_le_bytes());
    request[8..12].copy_from_slice(&(y.round() as i32).to_le_bytes());
    request[12..16].copy_from_slice(&u32::from(visible).to_le_bytes());
    let mut reply = [0u8; 16];
    let len = ipc_call_raw(
        compositor,
        request.as_ptr(),
        request.len(),
        reply.as_mut_ptr(),
        reply.len(),
    )?;
    if len < 4 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    let status = u32::from_le_bytes([reply[0], reply[1], reply[2], reply[3]]);
    if status == 0 {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(u64::from(status)))
    }
}

pub(super) fn set_cursor_image(
    compositor: u64,
    image: &ImageData,
) -> Result<(), MochiOsBackendError> {
    let pixels = image.premultiplied_rgba8();
    let mut request = Vec::new();
    request
        .try_reserve_exact(20usize.saturating_add(pixels.len()))
        .map_err(|_| MochiOsBackendError::ArithmeticOverflow)?;
    request.extend_from_slice(&OP_SET_CURSOR_IMAGE.to_le_bytes());
    request.extend_from_slice(&image.width().to_le_bytes());
    request.extend_from_slice(&image.height().to_le_bytes());
    request.extend_from_slice(&(CURSOR_HOTSPOT_X.round() as i32).to_le_bytes());
    request.extend_from_slice(&(CURSOR_HOTSPOT_Y.round() as i32).to_le_bytes());
    request.extend_from_slice(pixels);
    let mut reply = [0u8; 16];
    let len = ipc_call_raw(
        compositor,
        request.as_ptr(),
        request.len(),
        reply.as_mut_ptr(),
        reply.len(),
    )?;
    if len < 4 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    let status = u32::from_le_bytes([reply[0], reply[1], reply[2], reply[3]]);
    if status == 0 {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(u64::from(status)))
    }
}
