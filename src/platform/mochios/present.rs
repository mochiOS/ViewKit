use super::*;

use super::connection::{ipc_call_raw, put_u32_raw, put_u64_raw, status_from_raw, zero_raw};
use super::renderer::valid_scale_factor;

pub(super) fn physical_dirty_rect(viewport: Viewport, dirty_bounds: Rect) -> PhysicalDirtyRect {
    let viewport_bounds = viewport.logical_bounds();
    let dirty = dirty_bounds
        .intersection(viewport_bounds)
        .unwrap_or(viewport_bounds);
    let scale = valid_scale_factor(viewport.scale_factor);
    let x = (dirty.origin.x * scale).floor().max(0.0);
    let y = (dirty.origin.y * scale).floor().max(0.0);
    let right = ((dirty.origin.x + dirty.size.width) * scale)
        .ceil()
        .min(viewport.physical_width as f32);
    let bottom = ((dirty.origin.y + dirty.size.height) * scale)
        .ceil()
        .min(viewport.physical_height as f32);
    let width = (right - x).max(1.0);
    let height = (bottom - y).max(1.0);

    PhysicalDirtyRect {
        x: x as usize,
        y: y as usize,
        width: width as usize,
        height: height as usize,
    }
}

pub(super) fn damage_token_request(
    compositor: u64,
    token: u64,
    viewport: Viewport,
    dirty_bounds: Rect,
) -> Result<(), MochiOsBackendError> {
    let dirty = physical_dirty_rect(viewport, dirty_bounds);

    let request = core::ptr::addr_of_mut!(DAMAGE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 28);
        put_u32_raw(request, 0, OP_DAMAGE);
        put_u64_raw(request, 4, token);
        put_u32_raw(request, 12, dirty.x as u32);
        put_u32_raw(request, 16, dirty.y as u32);
        put_u32_raw(request, 20, dirty.width as u32);
        put_u32_raw(request, 24, dirty.height as u32);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
    status_from_raw(reply, len)
}
