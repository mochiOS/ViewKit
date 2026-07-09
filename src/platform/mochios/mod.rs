use std::cell::Cell;

use mochi_user_syscall as syscall;

use crate::draw_command::{DisplayList, DrawCommand};
use crate::geometry::Rect;
use crate::platform::{
    CursorIcon, PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig,
};
use crate::renderer::Viewport;
use crate::theme::Color;

const COMPOSITOR_SERVICE_NAME: &str = "compositor.service";
const OP_CREATE_SURFACE: u32 = 1;
const OP_ATTACH_BUFFER: u32 = 2;
const OP_DAMAGE: u32 = 3;
const OP_COMMIT: u32 = 4;
const ROLE_TOPLEVEL: u32 = 1;
const PIXEL_FORMAT_XRGB8888: u32 = 1;
const PAGE_SIZE: usize = 4096;
const MAX_SURFACE_EXTENT: u32 = 4096;
const EVENT_LOOP_IDLE_YIELDS: usize = 128;

static mut CREATE_SURFACE_REQ: [u8; 24] = [0; 24];
static mut ATTACH_BUFFER_REQ: [u8; 28] = [0; 28];
static mut TOKEN_REQ: [u8; 12] = [0; 12];
static mut IPC_REPLY: [u8; 16] = [0; 16];

#[derive(Debug, thiserror::Error)]
pub enum MochiOsBackendError {
    #[error("mochiOS syscall failed: {0}")]
    Syscall(u64),

    #[error("compositor.service was not found")]
    CompositorNotFound,

    #[error("invalid compositor reply")]
    InvalidReply,

    #[error("invalid window size")]
    InvalidWindowSize,

    #[error("arithmetic overflow")]
    ArithmeticOverflow,
}

pub struct MochiOsBackend<A>
where
    A: PlatformApplication,
{
    app: A,
    config: WindowConfig,
}

impl<A> MochiOsBackend<A>
where
    A: PlatformApplication,
{
    pub fn new(app: A, config: WindowConfig) -> Self {
        Self { app, config }
    }

    pub fn run(mut self) -> Result<(), MochiOsBackendError> {
        let compositor = find_compositor()?;
        let size = checked_surface_size(self.config.size)?;
        let viewport = Viewport::new(self.config.size, size.0, size.1, 1.0);
        let window = MochiOsWindow::new(viewport);
        let token = create_surface(compositor, 0, size.0, size.1)?;
        let mut shared_buffer = SharedBuffer::new(size.0 as usize, size.1 as usize)?;

        self.app
            .handle_event(PlatformEvent::Resumed { viewport }, &window);
        window.request_redraw();

        let mut display_list = DisplayList::new();

        loop {
            if window.take_redraw_requested() {
                display_list.clear();
                let _ = self.app.draw(window.viewport(), &mut display_list);
                let buffer = render_display_list(window.viewport(), &display_list)?;
                attach_buffer(
                    compositor,
                    token,
                    window.width() as usize,
                    window.height() as usize,
                    &buffer,
                    &mut shared_buffer,
                )?;
                simple_token_request(compositor, OP_DAMAGE, token)?;
                simple_token_request(compositor, OP_COMMIT, token)?;
            }

            for _ in 0..EVENT_LOOP_IDLE_YIELDS {
                let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
            }
        }
    }
}

struct MochiOsWindow {
    viewport: Viewport,
    redraw_requested: Cell<bool>,
}

impl MochiOsWindow {
    fn new(viewport: Viewport) -> Self {
        Self {
            viewport,
            redraw_requested: Cell::new(false),
        }
    }

    const fn width(&self) -> u32 {
        self.viewport.physical_width
    }

    const fn height(&self) -> u32 {
        self.viewport.physical_height
    }

    fn take_redraw_requested(&self) -> bool {
        self.redraw_requested.replace(false)
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
        self.viewport
    }

    fn set_cursor(&self, cursor: CursorIcon) {
        let _ = cursor;
    }
}

fn checked_surface_size(size: crate::geometry::Size) -> Result<(u32, u32), MochiOsBackendError> {
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

fn syscall_result<T>(result: syscall::SysResult<T>) -> Result<T, MochiOsBackendError> {
    result.map_err(|err| MochiOsBackendError::Syscall(err.errno().unwrap_or(5)))
}

fn find_compositor() -> Result<u64, MochiOsBackendError> {
    let name = COMPOSITOR_SERVICE_NAME.as_bytes();
    for _ in 0..64 {
        let tid = syscall_result(syscall::call2(
            syscall::SyscallNumber::FindProcessByName,
            name.as_ptr() as u64,
            name.len() as u64,
        ))?;
        if tid != 0 {
            return Ok(tid);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
    Err(MochiOsBackendError::CompositorNotFound)
}

fn ipc_call_raw(
    dest: u64,
    req_ptr: *const u8,
    req_len: usize,
    reply_ptr: *mut u8,
    reply_len: usize,
) -> Result<usize, MochiOsBackendError> {
    let msg = syscall_result(syscall::call5(
        syscall::SyscallNumber::IpcCall,
        dest,
        req_ptr as u64,
        req_len as u64,
        reply_ptr as u64,
        reply_len as u64,
    ))?;
    Ok((msg & 0xffff_ffff) as usize)
}

fn alloc_shared_page_count(page_count: usize) -> Result<u64, MochiOsBackendError> {
    let virt = syscall_result(syscall::call4(
        syscall::SyscallNumber::AllocSharedPages,
        page_count as u64,
        0,
        0,
        0,
    ))?;
    if virt == 0 || (virt & (PAGE_SIZE as u64 - 1)) != 0 {
        return Err(MochiOsBackendError::Syscall(5));
    }
    Ok(virt)
}

fn send_pages(dest: u64, page_count: usize, local_base: u64) -> Result<(), MochiOsBackendError> {
    syscall_result(syscall::call4(
        syscall::SyscallNumber::IpcSendPages,
        dest,
        0,
        page_count as u64,
        local_base,
    ))?;
    Ok(())
}

struct SharedBuffer {
    virt: u64,
    page_count: usize,
    pixel_count: usize,
}

impl SharedBuffer {
    fn new(width: usize, height: usize) -> Result<Self, MochiOsBackendError> {
        let pixel_count = width
            .checked_mul(height)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let byte_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let page_count = byte_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let virt = alloc_shared_page_count(page_count)?;

        Ok(Self {
            virt,
            page_count,
            pixel_count,
        })
    }

    fn copy_from(&mut self, buffer: &[u32]) -> Result<(), MochiOsBackendError> {
        if buffer.len() < self.pixel_count {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }

        let pixels =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u32, self.pixel_count) };
        pixels.copy_from_slice(&buffer[..self.pixel_count]);
        Ok(())
    }

    fn send_to(&self, compositor: u64) -> Result<(), MochiOsBackendError> {
        send_pages(compositor, self.page_count, self.virt)
    }
}

unsafe fn zero_raw(ptr: *mut u8, len: usize) {
    unsafe {
        core::ptr::write_bytes(ptr, 0, len);
    }
}

unsafe fn put_u32_raw(ptr: *mut u8, offset: usize, value: u32) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 4);
    }
}

unsafe fn put_u64_raw(ptr: *mut u8, offset: usize, value: u64) {
    unsafe {
        core::ptr::copy_nonoverlapping(value.to_le_bytes().as_ptr(), ptr.add(offset), 8);
    }
}

unsafe fn read_u32_raw(ptr: *const u8, offset: usize) -> u32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    u32::from_le_bytes(bytes)
}

unsafe fn read_u64_raw(ptr: *const u8, offset: usize) -> u64 {
    let mut bytes = [0u8; 8];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 8);
    }
    u64::from_le_bytes(bytes)
}

fn status_from_raw(ptr: *const u8, len: usize) -> Result<(), MochiOsBackendError> {
    if len < 4 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    let status = unsafe { read_u32_raw(ptr, 0) };
    if status == 0 {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(status as u64))
    }
}

fn create_surface(
    compositor: u64,
    event_endpoint: u64,
    width: u32,
    height: u32,
) -> Result<u64, MochiOsBackendError> {
    let request = core::ptr::addr_of_mut!(CREATE_SURFACE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 24);
        put_u32_raw(request, 0, OP_CREATE_SURFACE);
        put_u32_raw(request, 4, ROLE_TOPLEVEL);
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

fn attach_buffer(
    compositor: u64,
    token: u64,
    width: usize,
    height: usize,
    buffer: &[u32],
    shared_buffer: &mut SharedBuffer,
) -> Result<(), MochiOsBackendError> {
    let pixel_count = width
        .checked_mul(height)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    if buffer.len() < pixel_count {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }
    shared_buffer.copy_from(buffer)?;

    let request = core::ptr::addr_of_mut!(ATTACH_BUFFER_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(IPC_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 28);
        put_u32_raw(request, 0, OP_ATTACH_BUFFER);
        put_u64_raw(request, 4, token);
        put_u32_raw(request, 12, width as u32);
        put_u32_raw(request, 16, height as u32);
        put_u32_raw(request, 20, width as u32);
        put_u32_raw(request, 24, PIXEL_FORMAT_XRGB8888);
        zero_raw(reply, 16);
    }
    let len = ipc_call_raw(compositor, request, 28, reply, 16)?;
    status_from_raw(reply, len)?;
    shared_buffer.send_to(compositor)
}

fn simple_token_request(
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

fn render_display_list(
    viewport: Viewport,
    display_list: &DisplayList,
) -> Result<Vec<u32>, MochiOsBackendError> {
    let width = viewport.physical_width as usize;
    let height = viewport.physical_height as usize;
    let pixel_count = width
        .checked_mul(height)
        .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
    let mut buffer = vec![0xff00_0000; pixel_count];
    let bounds = Rect::new(0.0, 0.0, width as f32, height as f32);
    let mut clips = vec![bounds];

    for command in display_list.commands() {
        match command {
            DrawCommand::Clear { color } => {
                fill_pixels(&mut buffer, *color);
            }
            DrawCommand::FillRect { rect, color }
            | DrawCommand::FillRoundedRect { rect, color, .. }
            | DrawCommand::FillEllipse { rect, color } => {
                fill_rect(
                    &mut buffer,
                    width,
                    height,
                    *rect,
                    *color,
                    *clips.last().unwrap_or(&bounds),
                );
            }
            DrawCommand::StrokeRect {
                rect,
                color,
                width: stroke_width,
            }
            | DrawCommand::StrokeRoundedRect {
                rect,
                color,
                width: stroke_width,
                ..
            }
            | DrawCommand::StrokeEllipse {
                rect,
                color,
                width: stroke_width,
            } => {
                stroke_rect(
                    &mut buffer,
                    width,
                    height,
                    *rect,
                    *stroke_width,
                    *color,
                    *clips.last().unwrap_or(&bounds),
                );
            }
            DrawCommand::PushClip { rect } | DrawCommand::PushRoundedClip { rect, .. } => {
                let current = *clips.last().unwrap_or(&bounds);
                clips.push(
                    rect.intersection(current)
                        .unwrap_or(Rect::new(0.0, 0.0, 0.0, 0.0)),
                );
            }
            DrawCommand::PopClip => {
                if clips.len() > 1 {
                    clips.pop();
                }
            }
            DrawCommand::DrawText { .. }
            | DrawCommand::DrawImage { .. }
            | DrawCommand::DrawSvg { .. } => {}
        }
    }

    Ok(buffer)
}

fn fill_pixels(buffer: &mut [u32], color: Color) {
    let pixel = color_to_xrgb(color);
    for dst in buffer {
        *dst = pixel;
    }
}

fn fill_rect(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    color: Color,
    clip: Rect,
) {
    let Some(rect) = sanitize_rect(rect, width, height).and_then(|r| r.intersection(clip)) else {
        return;
    };
    let (left, top, right, bottom) = rect_to_pixels(rect, width, height);
    for y in top..bottom {
        let row = y * width;
        for x in left..right {
            blend_pixel(&mut buffer[row + x], color);
        }
    }
}

fn stroke_rect(
    buffer: &mut [u32],
    width: usize,
    height: usize,
    rect: Rect,
    stroke_width: f32,
    color: Color,
    clip: Rect,
) {
    if stroke_width <= 0.0 || !stroke_width.is_finite() {
        return;
    }
    let w = stroke_width.ceil();
    fill_rect(
        buffer,
        width,
        height,
        Rect::new(rect.origin.x, rect.origin.y, rect.size.width, w),
        color,
        clip,
    );
    fill_rect(
        buffer,
        width,
        height,
        Rect::new(
            rect.origin.x,
            rect.origin.y + rect.size.height - w,
            rect.size.width,
            w,
        ),
        color,
        clip,
    );
    fill_rect(
        buffer,
        width,
        height,
        Rect::new(rect.origin.x, rect.origin.y, w, rect.size.height),
        color,
        clip,
    );
    fill_rect(
        buffer,
        width,
        height,
        Rect::new(
            rect.origin.x + rect.size.width - w,
            rect.origin.y,
            w,
            rect.size.height,
        ),
        color,
        clip,
    );
}

fn sanitize_rect(rect: Rect, width: usize, height: usize) -> Option<Rect> {
    if !rect.origin.x.is_finite()
        || !rect.origin.y.is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width <= 0.0
        || rect.size.height <= 0.0
    {
        return None;
    }
    rect.intersection(Rect::new(0.0, 0.0, width as f32, height as f32))
}

fn rect_to_pixels(rect: Rect, width: usize, height: usize) -> (usize, usize, usize, usize) {
    let left = rect.origin.x.floor().max(0.0).min(width as f32) as usize;
    let top = rect.origin.y.floor().max(0.0).min(height as f32) as usize;
    let right = (rect.origin.x + rect.size.width)
        .ceil()
        .max(0.0)
        .min(width as f32) as usize;
    let bottom = (rect.origin.y + rect.size.height)
        .ceil()
        .max(0.0)
        .min(height as f32) as usize;
    (left, top, right, bottom)
}

fn color_to_xrgb(color: Color) -> u32 {
    0xff00_0000 | ((color.red as u32) << 16) | ((color.green as u32) << 8) | color.blue as u32
}

fn blend_pixel(dst: &mut u32, src: Color) {
    if src.alpha == 255 {
        *dst = color_to_xrgb(src);
        return;
    }
    if src.alpha == 0 {
        return;
    }

    let alpha = src.alpha as u32;
    let inv = 255 - alpha;
    let dr = (*dst >> 16) & 0xff;
    let dg = (*dst >> 8) & 0xff;
    let db = *dst & 0xff;
    let r = (src.red as u32 * alpha + dr * inv) / 255;
    let g = (src.green as u32 * alpha + dg * inv) / 255;
    let b = (src.blue as u32 * alpha + db * inv) / 255;
    *dst = 0xff00_0000 | (r << 16) | (g << 8) | b;
}
