use std::cell::Cell;
use std::time::Instant;

use mochi_user_syscall as syscall;

use crate::draw_command::{DisplayList, DrawCommand};
use crate::geometry::Rect;
use crate::platform::{
    ButtonState, CursorIcon, PlatformApplication, PlatformEvent, PlatformWindow, PointerButton,
    WindowConfig,
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
const MAX_IPC_PAGE_CHUNK: usize = 128;
const MAX_SURFACE_EXTENT: u32 = 4096;
const EVENT_POINTER_ENTER: u32 = 2;
const EVENT_POINTER_LEAVE: u32 = 3;
const EVENT_POINTER_MOTION: u32 = 4;
const EVENT_POINTER_BUTTON: u32 = 5;
const EVENT_KEY: u32 = 6;
const EVENT_FOCUS_GAINED: u32 = 8;
const EVENT_FOCUS_LOST: u32 = 9;
const EVENT_FRAME_DONE: u32 = 10;
const KEY_BACKSPACE: u16 = 2;
const KEY_TAB: u16 = 3;
const KEY_ENTER: u16 = 4;
const KEY_SPACE: u16 = 5;
const KEY_DELETE: u16 = 76;
const KEY_HOME: u16 = 71;
const KEY_END: u16 = 79;
const KEY_LEFT: u16 = 75;
const KEY_RIGHT: u16 = 77;
const KEY_PAGE_UP: u16 = 73;
const KEY_PAGE_DOWN: u16 = 81;

static mut CREATE_SURFACE_REQ: [u8; 24] = [0; 24];
static mut ATTACH_BUFFER_REQ: [u8; 28] = [0; 28];
static mut TOKEN_REQ: [u8; 12] = [0; 12];
static mut IPC_REPLY: [u8; 16] = [0; 16];
static mut EVENT_BUF: [u8; 32] = [0; 32];

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

    #[error("invalid compositor event")]
    InvalidEvent,
}

pub struct MochiOsBackend<A>
where
    A: PlatformApplication,
{
    app: A,
    config: WindowConfig,
    pressed_buttons: Vec<u16>,
}

impl<A> MochiOsBackend<A>
where
    A: PlatformApplication,
{
    pub fn new(app: A, config: WindowConfig) -> Self {
        Self {
            app,
            config,
            pressed_buttons: Vec::new(),
        }
    }

    pub fn run(mut self) -> Result<(), MochiOsBackendError> {
        let compositor = find_compositor()?;
        let event_endpoint = create_event_endpoint()?;
        let size = checked_surface_size(self.config.size)?;
        let viewport = Viewport::new(self.config.size, size.0, size.1, 1.0);
        let window = MochiOsWindow::new(viewport);
        let token = create_surface(compositor, event_endpoint, size.0, size.1)?;
        let mut shared_buffer = SharedBuffer::new(size.0 as usize, size.1 as usize)?;

        self.app
            .handle_event(PlatformEvent::Resumed { viewport }, &window);
        window.request_redraw();

        let mut display_list = DisplayList::new();

        loop {
            let mut handled_work = false;

            while let Some(event) = try_recv_event(event_endpoint)? {
                self.handle_compositor_event(event, &window)?;
                handled_work = true;
            }

            let redraw_due = self
                .app
                .next_redraw_at()
                .is_some_and(|deadline| deadline <= Instant::now());

            if window.take_redraw_requested() || redraw_due {
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
                handled_work = true;
            }

            if !handled_work {
                if let Some(deadline) = self.app.next_redraw_at() {
                    if wait_until_deadline(deadline, &window, &mut self)? {
                        continue;
                    }
                } else {
                    wait_for_event(event_endpoint, &window, &mut self)?;
                }
            }
        }
    }

    fn handle_compositor_event(
        &mut self,
        event: [u8; 32],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        let kind = unsafe { read_u32_raw(event.as_ptr(), 0) };
        let a = unsafe { read_i32_raw(event.as_ptr(), 4) };
        let b = unsafe { read_i32_raw(event.as_ptr(), 8) };
        let c = unsafe { read_u32_raw(event.as_ptr(), 12) };

        match kind {
            EVENT_POINTER_ENTER | EVENT_POINTER_MOTION => {
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: a as f32,
                        y: b as f32,
                    },
                    window,
                );
            }
            EVENT_POINTER_LEAVE => {
                self.app.handle_event(PlatformEvent::PointerLeft, window);
            }
            EVENT_POINTER_BUTTON => {
                let button = match c as u16 {
                    1 => PointerButton::Primary,
                    2 => PointerButton::Secondary,
                    3 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                };
                let button_id = c as u16;
                let state = if let Some(pos) = self
                    .pressed_buttons
                    .iter()
                    .position(|pressed| *pressed == button_id)
                {
                    self.pressed_buttons.swap_remove(pos);
                    ButtonState::Released
                } else {
                    self.pressed_buttons.push(button_id);
                    ButtonState::Pressed
                };
                self.app
                    .handle_event(PlatformEvent::PointerButton { button, state }, window);
            }
            EVENT_KEY => {
                if c & 1 != 0 {
                    if let Some(event) = self.key_event(a as u16, b as u32) {
                        self.app.handle_event(event, window);
                    }
                }
            }
            EVENT_FOCUS_GAINED => {
                self.app.handle_event(PlatformEvent::Focused(true), window);
            }
            EVENT_FOCUS_LOST => {
                self.app.handle_event(PlatformEvent::Focused(false), window);
            }
            EVENT_FRAME_DONE => {
                window.request_redraw();
            }
            _ => {}
        }

        Ok(())
    }

    fn key_event(&self, keycode: u16, codepoint: u32) -> Option<PlatformEvent> {
        if let Some(text) = char::from_u32(codepoint)
            && !text.is_control()
        {
            return Some(PlatformEvent::TextInput {
                text: text.to_string(),
            });
        }
        Some(match keycode {
            KEY_BACKSPACE => PlatformEvent::Backspace,
            KEY_TAB => PlatformEvent::TextInput {
                text: String::from("\t"),
            },
            KEY_ENTER => PlatformEvent::TextInput {
                text: String::from("\n"),
            },
            KEY_SPACE => PlatformEvent::TextInput {
                text: String::from(" "),
            },
            KEY_DELETE => PlatformEvent::Delete,
            KEY_HOME => PlatformEvent::Home,
            KEY_END => PlatformEvent::End,
            KEY_LEFT => PlatformEvent::ArrowLeft,
            KEY_RIGHT => PlatformEvent::ArrowRight,
            KEY_PAGE_UP => PlatformEvent::SelectHome,
            KEY_PAGE_DOWN => PlatformEvent::SelectEnd,
            _ => return None,
        })
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

fn create_event_endpoint() -> Result<u64, MochiOsBackendError> {
    syscall_result(syscall::call2(syscall::SyscallNumber::IpcCreate, 0, 0))
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

fn ipc_wait_raw(
    endpoint: u64,
    buf_ptr: *mut u8,
    buf_len: usize,
) -> Result<usize, MochiOsBackendError> {
    let msg = syscall_result(syscall::call3(
        syscall::SyscallNumber::IpcWait,
        buf_ptr as u64,
        buf_len as u64,
        endpoint,
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
    byte_capacity: usize,
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
        let page_count = page_count.min(MAX_IPC_PAGE_CHUNK).max(1);
        let byte_capacity = page_count
            .checked_mul(PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let virt = alloc_shared_page_count(page_count)?;

        Ok(Self {
            virt,
            byte_capacity,
        })
    }

    fn send_to(&mut self, compositor: u64, buffer: &[u32]) -> Result<(), MochiOsBackendError> {
        let bytes_len = buffer
            .len()
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let src = unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), bytes_len) };
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };
        let mut offset = 0usize;
        while offset < src.len() {
            let remaining = src.len() - offset;
            let chunk_len = remaining.min(self.byte_capacity);
            dst[..chunk_len].copy_from_slice(&src[offset..offset + chunk_len]);
            let page_count = chunk_len
                .checked_add(PAGE_SIZE - 1)
                .map(|len| len / PAGE_SIZE)
                .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
            send_pages(compositor, page_count, self.virt)?;
            offset += chunk_len;
        }
        Ok(())
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

unsafe fn read_i32_raw(ptr: *const u8, offset: usize) -> i32 {
    let mut bytes = [0u8; 4];
    unsafe {
        core::ptr::copy_nonoverlapping(ptr.add(offset), bytes.as_mut_ptr(), 4);
    }
    i32::from_le_bytes(bytes)
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

fn try_recv_event() -> Result<Option<[u8; 32]>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(0, event, 32) {
        Ok(len) => len,
        Err(MochiOsBackendError::Syscall(err)) if err == syscall::EAGAIN => return Ok(None),
        Err(err) => return Err(err),
    };
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; 32];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some(out))
}

fn wait_for_event<A: PlatformApplication>(
    endpoint: u64,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<(), MochiOsBackendError> {
    if let Some(event) = read_event_blocking(endpoint)? {
        backend.handle_compositor_event(event, window)?;
    }
    Ok(())
}

fn wait_until_deadline<A: PlatformApplication>(
    deadline: Instant,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<bool, MochiOsBackendError> {
    loop {
        if let Some(event) = try_recv_event()? {
            backend.handle_compositor_event(event, window)?;
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
}

fn read_event_blocking(endpoint: u64) -> Result<Option<[u8; 32]>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = ipc_wait_raw(endpoint, event, 32)?;
    if len < 16 {
        return Err(MochiOsBackendError::InvalidEvent);
    }
    let mut out = [0u8; 32];
    let copy_len = len.min(out.len());
    unsafe {
        core::ptr::copy_nonoverlapping(event, out.as_mut_ptr(), copy_len);
    }
    Ok(Some(out))
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
    shared_buffer.send_to(compositor, &buffer[..pixel_count])
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
