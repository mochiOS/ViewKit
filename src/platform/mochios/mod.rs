use std::cell::Cell;
use std::collections::HashMap;
use std::time::Instant;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use mochi_user_syscall as syscall;
use tiny_skia::{
    Color as SkiaColor, FillRule, Mask, Paint, Path, PathBuilder, Pixmap, Rect as SkiaRect, Stroke,
    Transform,
};

use crate::draw_command::{DisplayList, DrawCommand, TextCommand};
use crate::font::create_font_system;
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
const MAX_SURFACE_EXTENT: u32 = 16_384;
const ERRNO_EAGAIN: u64 = 11;
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
const KEY_DELETE: u16 = 79;
const KEY_HOME: u16 = 80;
const KEY_END: u16 = 81;
const KEY_LEFT: u16 = 82;
const KEY_RIGHT: u16 = 83;
const KEY_PAGE_UP: u16 = 86;
const KEY_PAGE_DOWN: u16 = 87;
const INPUT_FLAG_PRESS: u32 = 1 << 0;
const INPUT_FLAG_RELEASE: u32 = 1 << 1;
const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct TextLayoutKey {
    text: String,
    font_family: String,
    font_size_bits: u32,
    line_height_bits: u32,
    width_bits: u32,
    height_bits: u32,
    scale_bits: u32,
    weight: u16,
    alignment: u8,
}

impl TextLayoutKey {
    fn new(command: &TextCommand, scale: f32) -> Self {
        Self {
            text: command.text.clone(),
            font_family: command.font_family.clone(),
            font_size_bits: canonical_f32_bits(command.font_size),
            line_height_bits: canonical_f32_bits(command.line_height),
            width_bits: canonical_f32_bits(command.bounds.size.width),
            height_bits: canonical_f32_bits(command.bounds.size.height),
            scale_bits: canonical_f32_bits(scale),
            weight: command.weight.clamp(1, 1000),
            alignment: alignment_key(command.alignment),
        }
    }
}

fn canonical_f32_bits(value: f32) -> u32 {
    if value == 0.0 {
        0.0_f32.to_bits()
    } else {
        value.to_bits()
    }
}

const fn alignment_key(alignment: crate::typography::TextAlignment) -> u8 {
    match alignment {
        crate::typography::TextAlignment::Start => 0,
        crate::typography::TextAlignment::Center => 1,
        crate::typography::TextAlignment::End => 2,
        crate::typography::TextAlignment::Justified => 3,
    }
}

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
    font_system: FontSystem,
    swash_cache: SwashCache,
    text_layout_cache: HashMap<TextLayoutKey, Buffer>,
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
            font_system: create_font_system(),
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
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

            while let Some(event) = try_recv_event()? {
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
                let buffer = render_display_list(
                    window.viewport(),
                    &display_list,
                    &mut self.font_system,
                    &mut self.swash_cache,
                    &mut self.text_layout_cache,
                )?;
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
                let button_id = (c & 0xffff) as u16;
                let flags = c >> 16;
                let button = match button_id {
                    1 => PointerButton::Primary,
                    2 => PointerButton::Secondary,
                    3 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                };
                let state = if flags & INPUT_FLAG_PRESS != 0 {
                    if !self.pressed_buttons.contains(&button_id) {
                        self.pressed_buttons.push(button_id);
                    }
                    ButtonState::Pressed
                } else if flags & INPUT_FLAG_RELEASE != 0 {
                    if let Some(pos) = self
                        .pressed_buttons
                        .iter()
                        .position(|pressed| *pressed == button_id)
                    {
                        self.pressed_buttons.swap_remove(pos);
                    }
                    ButtonState::Released
                } else {
                    self.toggle_button_state(button_id)
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

    fn toggle_button_state(&mut self, button_id: u16) -> ButtonState {
        if let Some(pos) = self
            .pressed_buttons
            .iter()
            .position(|pressed| *pressed == button_id)
        {
            self.pressed_buttons.swap_remove(pos);
            ButtonState::Released
        } else {
            self.pressed_buttons.push(button_id);
            ButtonState::Pressed
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
        let page_count = page_count.max(1);
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
        if bytes_len > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };
        dst[..bytes_len].copy_from_slice(src);
        let page_count = bytes_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        send_pages(compositor, page_count, self.virt)?;
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
        Err(MochiOsBackendError::Syscall(ERRNO_EAGAIN)) => return Ok(None),
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
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text_layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
) -> Result<Vec<u32>, MochiOsBackendError> {
    let width = viewport.physical_width;
    let height = viewport.physical_height;
    let mut pixmap = Pixmap::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
    pixmap.fill(SkiaColor::from_rgba8(0, 0, 0, 0));
    let mut clear_color = Color::BLACK;

    let scale = valid_scale_factor(viewport.scale_factor);
    let transform = Transform::from_scale(scale, scale);
    let bounds = viewport.logical_bounds();
    let mut clip_stack = vec![create_clip_mask(bounds, None, width, height, transform)?];

    for command in display_list.commands() {
        match command {
            DrawCommand::Clear { color } => {
                clear_color = *color;
                pixmap.fill(SkiaColor::from_rgba8(
                    color.red,
                    color.green,
                    color.blue,
                    color.alpha,
                ));
            }
            DrawCommand::FillRect { rect, color } => {
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let paint = solid_paint(*color);
                pixmap.fill_rect(rect, &paint, transform, clip_stack.last());
            }
            DrawCommand::FillRoundedRect {
                rect,
                radius,
                color,
            } => {
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = rounded_rect_path(rect, *radius);
                let paint = solid_paint(*color);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    transform,
                    clip_stack.last(),
                );
            }
            DrawCommand::FillEllipse { rect, color } => {
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = ellipse_path(rect);
                let paint = solid_paint(*color);
                pixmap.fill_path(
                    &path,
                    &paint,
                    FillRule::Winding,
                    transform,
                    clip_stack.last(),
                );
            }
            DrawCommand::StrokeRect {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = PathBuilder::from_rect(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::StrokeRoundedRect {
                rect,
                radius,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = rounded_rect_path(rect, *radius);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::StrokeEllipse {
                rect,
                color,
                width: stroke_width,
            } => {
                if !stroke_width.is_finite() || *stroke_width <= 0.0 {
                    continue;
                }
                let Some(rect) = to_skia_rect(*rect) else {
                    continue;
                };
                let path = ellipse_path(rect);
                let paint = solid_paint(*color);
                let stroke = Stroke {
                    width: *stroke_width,
                    ..Stroke::default()
                };
                pixmap.stroke_path(&path, &paint, &stroke, transform, clip_stack.last());
            }
            DrawCommand::PushClip { rect } => {
                let mask = create_clip_mask(*rect, clip_stack.last(), width, height, transform)?;
                clip_stack.push(mask);
            }
            DrawCommand::PushRoundedClip { rect, radius } => {
                let mask = create_rounded_clip_mask(
                    *rect,
                    *radius,
                    clip_stack.last(),
                    width,
                    height,
                    transform,
                )?;
                clip_stack.push(mask);
            }
            DrawCommand::PopClip => {
                if clip_stack.len() > 1 {
                    clip_stack.pop();
                }
            }
            DrawCommand::DrawText { command } => {
                if command.bounds.intersection(bounds).is_none() {
                    continue;
                }
                draw_text_command(
                    &mut pixmap,
                    font_system,
                    swash_cache,
                    text_layout_cache,
                    command,
                    scale,
                    clip_stack.last(),
                );
            }
            DrawCommand::DrawImage { .. } | DrawCommand::DrawSvg { .. } => {}
        }
    }

    Ok(pixmap_to_xrgb(&pixmap, clear_color))
}

fn valid_scale_factor(scale_factor: f64) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor as f32
    } else {
        1.0
    }
}

fn draw_text_command(
    pixmap: &mut Pixmap,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    command: &TextCommand,
    scale: f32,
    clip: Option<&Mask>,
) {
    if command.text.is_empty()
        || command.bounds.size.width <= 0.0
        || command.bounds.size.height <= 0.0
    {
        return;
    }

    let scale = if scale.is_finite() && scale > 0.0 {
        scale
    } else {
        1.0
    };

    let font_size = (command.font_size * scale).max(1.0);
    let line_height = (command.line_height * scale).max(font_size);
    let width = (command.bounds.size.width * scale).max(0.0);
    let height = (command.bounds.size.height * scale).max(0.0);
    let origin_x = (command.bounds.origin.x * scale).round();
    let origin_y = command.bounds.origin.y * scale;
    let key = TextLayoutKey::new(command, scale);

    if !layout_cache.contains_key(&key) {
        if layout_cache.len() >= TEXT_LAYOUT_CACHE_CAPACITY {
            layout_cache.clear();
        }

        let metrics = Metrics::new(font_size, line_height);
        let mut buffer = Buffer::new(font_system, metrics);
        {
            let mut buffer_with_font_system = buffer.borrow_with(font_system);
            buffer_with_font_system.set_size(Some(width), Some(height));

            let attrs = Attrs::new()
                .family(Family::Name(command.font_family.as_str()))
                .weight(Weight(command.weight.clamp(1, 1000)));

            buffer_with_font_system.set_text(
                command.text.as_str(),
                &attrs,
                Shaping::Advanced,
                command.alignment.to_cosmic(),
            );
        }

        layout_cache.insert(key.clone(), buffer);
    }

    let Some(buffer) = layout_cache.get_mut(&key) else {
        return;
    };
    let mut buffer = buffer.borrow_with(font_system);
    let text_color = CosmicColor::rgba(
        command.color.red,
        command.color.green,
        command.color.blue,
        command.color.alpha,
    );
    let Some(text_clip) = SkiaRect::from_xywh(origin_x, origin_y, width, height) else {
        return;
    };

    let mut physical_glyphs = Vec::new();
    for run in buffer.layout_runs() {
        let baseline_y = (origin_y + run.line_y).round();
        for glyph in run.glyphs {
            physical_glyphs.push(glyph.physical((origin_x, baseline_y), 1.0));
        }
    }
    drop(buffer);

    for physical_glyph in physical_glyphs {
        swash_cache.with_pixels(
            font_system,
            physical_glyph.cache_key,
            text_color,
            |x, y, color| {
                let draw_x = physical_glyph.x + x;
                let draw_y = physical_glyph.y + y;
                let Some(pixel_rect) = SkiaRect::from_xywh(draw_x as f32, draw_y as f32, 1.0, 1.0)
                else {
                    return;
                };
                let Some(rect) = intersect_rect(pixel_rect, text_clip) else {
                    return;
                };
                let (red, green, blue, alpha) = color.as_rgba_tuple();
                if alpha == 0 {
                    return;
                }
                let mut paint = Paint::default();
                paint.set_color_rgba8(red, green, blue, alpha);
                paint.anti_alias = false;
                pixmap.fill_rect(rect, &paint, Transform::identity(), clip);
            },
        );
    }
}

fn intersect_rect(first: SkiaRect, second: SkiaRect) -> Option<SkiaRect> {
    let left = first.left().max(second.left());
    let top = first.top().max(second.top());
    let right = first.right().min(second.right());
    let bottom = first.bottom().min(second.bottom());
    if right <= left || bottom <= top {
        return None;
    }
    SkiaRect::from_xywh(left, top, right - left, bottom - top)
}

fn to_skia_rect(rect: Rect) -> Option<SkiaRect> {
    if !rect.origin.x.is_finite()
        || !rect.origin.y.is_finite()
        || !rect.size.width.is_finite()
        || !rect.size.height.is_finite()
        || rect.size.width < 0.0
        || rect.size.height < 0.0
    {
        return None;
    }
    SkiaRect::from_xywh(
        rect.origin.x,
        rect.origin.y,
        rect.size.width,
        rect.size.height,
    )
}

fn rounded_rect_path(rect: SkiaRect, radius: f32) -> Path {
    let radius = if radius.is_finite() {
        radius.max(0.0).min(rect.width().min(rect.height()) / 2.0)
    } else {
        0.0
    };
    if radius == 0.0 {
        return PathBuilder::from_rect(rect);
    }

    let left = rect.left();
    let top = rect.top();
    let right = rect.right();
    let bottom = rect.bottom();
    let mut builder = PathBuilder::new();
    builder.move_to(left + radius, top);
    builder.line_to(right - radius, top);
    builder.quad_to(right, top, right, top + radius);
    builder.line_to(right, bottom - radius);
    builder.quad_to(right, bottom, right - radius, bottom);
    builder.line_to(left + radius, bottom);
    builder.quad_to(left, bottom, left, bottom - radius);
    builder.line_to(left, top + radius);
    builder.quad_to(left, top, left + radius, top);
    builder.close();
    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn ellipse_path(rect: SkiaRect) -> Path {
    const KAPPA: f32 = 0.552_284_8;

    let center_x = (rect.left() + rect.right()) / 2.0;
    let center_y = (rect.top() + rect.bottom()) / 2.0;
    let radius_x = rect.width() / 2.0;
    let radius_y = rect.height() / 2.0;
    let control_x = radius_x * KAPPA;
    let control_y = radius_y * KAPPA;

    let mut builder = PathBuilder::new();
    builder.move_to(center_x + radius_x, center_y);
    builder.cubic_to(
        center_x + radius_x,
        center_y + control_y,
        center_x + control_x,
        center_y + radius_y,
        center_x,
        center_y + radius_y,
    );
    builder.cubic_to(
        center_x - control_x,
        center_y + radius_y,
        center_x - radius_x,
        center_y + control_y,
        center_x - radius_x,
        center_y,
    );
    builder.cubic_to(
        center_x - radius_x,
        center_y - control_y,
        center_x - control_x,
        center_y - radius_y,
        center_x,
        center_y - radius_y,
    );
    builder.cubic_to(
        center_x + control_x,
        center_y - radius_y,
        center_x + radius_x,
        center_y - control_y,
        center_x + radius_x,
        center_y,
    );
    builder.close();
    builder
        .finish()
        .unwrap_or_else(|| PathBuilder::from_rect(rect))
}

fn create_clip_mask(
    rect: Rect,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let path = to_skia_rect(rect).map(PathBuilder::from_rect);
    create_path_clip_mask(path, previous, width, height, transform)
}

fn create_rounded_clip_mask(
    rect: Rect,
    radius: f32,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let path = to_skia_rect(rect).map(|rect| rounded_rect_path(rect, radius));
    create_path_clip_mask(path, previous, width, height, transform)
}

fn create_path_clip_mask(
    path: Option<Path>,
    previous: Option<&Mask>,
    width: u32,
    height: u32,
    transform: Transform,
) -> Result<Mask, MochiOsBackendError> {
    let has_previous = previous.is_some();
    let mut mask = match previous {
        Some(previous) => previous.clone(),
        None => Mask::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?,
    };

    let Some(path) = path else {
        mask.clear();
        return Ok(mask);
    };

    if has_previous {
        mask.intersect_path(&path, FillRule::Winding, true, transform);
    } else {
        mask.clear();
        mask.fill_path(&path, FillRule::Winding, true, transform);
    }

    Ok(mask)
}

fn solid_paint(color: Color) -> Paint<'static> {
    let mut paint = Paint::default();
    paint.set_color_rgba8(color.red, color.green, color.blue, color.alpha);
    paint.anti_alias = true;
    paint
}

fn pixmap_to_xrgb(pixmap: &Pixmap, background: Color) -> Vec<u32> {
    let mut output = Vec::with_capacity(pixmap.width() as usize * pixmap.height() as usize);
    for pixel in pixmap.data().chunks_exact(4) {
        let alpha = pixel[3] as u32;
        let inv_alpha = 255_u32.saturating_sub(alpha);

        // tiny-skia stores premultiplied RGBA. The compositor surface is XRGB,
        // so each pixel must be flattened explicitly instead of dropping alpha.
        let red = pixel[0] as u32 + (background.red as u32 * inv_alpha + 127) / 255;
        let green = pixel[1] as u32 + (background.green as u32 * inv_alpha + 127) / 255;
        let blue = pixel[2] as u32 + (background.blue as u32 * inv_alpha + 127) / 255;

        output.push(0xff00_0000 | (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255));
    }
    output
}
