use std::cell::Cell;
use std::collections::HashMap;
use std::env;
use std::time::Instant;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use mochi_user_syscall as syscall;
use tiny_skia::{
    Color as SkiaColor, FillRule, FilterQuality, Mask, Paint, Path, PathBuilder, Pixmap,
    PixmapPaint, Rect as SkiaRect, Stroke, Transform,
};

use crate::draw_command::{DisplayList, DrawCommand, SvgCommand, TextCommand};
use crate::font::create_font_system;
use crate::geometry::Rect;
use crate::platform::{
    ButtonState, CursorIcon, PlatformApplication, PlatformEvent, PlatformWindow, PointerButton,
    WindowConfig,
};
use crate::renderer::Viewport;
use crate::theme::Color;

const COMPOSITOR_SERVICE_NAME: &str = "compositor.service";
const DISPLAY_SERVICE_NAME: &str = "display.driver";
const INPUT_SERVICE_NAME: &str = "input.service";
const WINDOW_OVERLAY_CAPABILITY: &str = "window.overlay";
const CAPABILITY_PROMPT_OPCODE: u32 = 0x4350_5251;
const DISPLAY_GET_INFO_OPCODE: u32 = 1;
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
const INPUT_SUBSCRIBE_OPCODE: u32 = 0x5355_4253;
const INPUT_EVENT_SIZE: usize = 32;
const INPUT_EVENT_KIND_POINTER_MOVE: u16 = 2;
const INPUT_EVENT_KIND_POINTER_BUTTON: u16 = 3;
const INPUT_EVENT_KIND_POINTER_ABSOLUTE: u16 = 5;
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
const INPUT_FLAG_PRESS: u16 = 1 << 0;
const INPUT_FLAG_RELEASE: u16 = 1 << 1;
const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;
const SVG_SMALL_RENDER_LIMIT: f32 = 256.0;
const SVG_SMALL_RENDER_SUPERSAMPLE: f32 = 2.0;

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
static mut DAMAGE_REQ: [u8; 28] = [0; 28];
static mut IPC_REPLY: [u8; 16] = [0; 16];
static mut EVENT_BUF: [u8; 32] = [0; 32];
static mut DISPLAY_REQ: [u8; 20] = [0; 20];
static mut DISPLAY_REPLY: [u8; 32] = [0; 32];
static mut INPUT_SUBSCRIBE_REQ: [u8; 16] = [0; 16];
static mut INPUT_SUBSCRIBE_REPLY: [u8; 1] = [0; 1];

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

#[repr(u32)]
#[derive(Clone, Copy)]
enum CapabilityClass {
    UserGrantable = 1,
}

#[repr(u32)]
#[derive(Clone, Copy)]
enum CapabilityDecision {
    AllowOnce = 1,
    AllowForProcess = 2,
    AllowPersistently = 3,
    AllowAllUserGrantable = 4,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapabilityExecutableIdentity {
    path_len: u16,
    reserved: u16,
    digest: [u8; 32],
    path: [u8; 256],
}

impl Default for CapabilityExecutableIdentity {
    fn default() -> Self {
        Self {
            path_len: 0,
            reserved: 0,
            digest: [0; 32],
            path: [0; 256],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapabilityResourceDescriptor {
    kind: u32,
    path_len: u16,
    reserved: u16,
    path: [u8; 256],
}

impl Default for CapabilityResourceDescriptor {
    fn default() -> Self {
        Self {
            kind: 0,
            path_len: 0,
            reserved: 0,
            path: [0; 256],
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CapabilityRequest {
    opcode: u32,
    process_id: u64,
    executable: CapabilityExecutableIdentity,
    capability_class: CapabilityClass,
    capability_len: u16,
    resource: CapabilityResourceDescriptor,
    reason_len: u16,
    interactive: u8,
    decision_scope: u8,
    reserved0: u16,
    capability: [u8; 64],
    reason: [u8; 128],
}

pub struct MochiOsBackend<A>
where
    A: PlatformApplication,
{
    app: A,
    config: WindowConfig,
    pressed_buttons: Vec<u16>,
    font_system: Option<FontSystem>,
    swash_cache: SwashCache,
    text_layout_cache: HashMap<TextLayoutKey, Buffer>,
    pixmap: Option<Pixmap>,
    direct_input: bool,
    pointer_x: f32,
    pointer_y: f32,
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
            font_system: None,
            swash_cache: SwashCache::new(),
            text_layout_cache: HashMap::new(),
            pixmap: None,
            direct_input: false,
            pointer_x: 0.0,
            pointer_y: 0.0,
        }
    }

    pub fn run(mut self) -> Result<(), MochiOsBackendError> {
        let compositor = find_compositor()?;
        let event_endpoint = create_event_endpoint()?;
        if self.config.fullscreen {
            require_window_overlay_capability()?;
        }
        let requested_size = if self.config.fullscreen {
            display_surface_size().unwrap_or_else(|| self.config.size)
        } else {
            self.config.size
        };
        let size = checked_surface_size(requested_size)?;
        let logical_size = if self.config.fullscreen {
            requested_size
        } else {
            self.config.size
        };
        let viewport = Viewport::new(logical_size, size.0, size.1, 1.0);
        let window = MochiOsWindow::new(viewport);
        let token = create_surface(compositor, event_endpoint, size.0, size.1)?;
        let mut shared_buffer = SharedBuffer::new(size.0 as usize, size.1 as usize)?;
        self.pointer_x = (viewport.logical_size.width / 2.0).max(0.0);
        self.pointer_y = (viewport.logical_size.height / 2.0).max(0.0);
        self.direct_input = self.config.fullscreen && subscribe_input_events(event_endpoint);

        self.app
            .handle_event(PlatformEvent::Resumed { viewport }, &window);
        window.request_redraw();

        let mut display_list = DisplayList::new();

        loop {
            let mut handled_work = false;

            while let Some((len, event)) = try_recv_event()? {
                self.handle_event_message(len, event, &window)?;
                handled_work = true;
            }

            let redraw_due = self
                .app
                .next_redraw_at()
                .is_some_and(|deadline| deadline <= Instant::now());

            if window.take_redraw_requested() || redraw_due {
                if self.font_system.is_none() {
                    self.font_system = Some(create_font_system());
                }
                display_list.clear();
                let dirty_bounds = self.app.draw(window.viewport(), &mut display_list);
                let clear_color = render_display_list(
                    window.viewport(),
                    dirty_bounds,
                    &display_list,
                    self.font_system
                        .as_mut()
                        .ok_or(MochiOsBackendError::InvalidWindowSize)?,
                    &mut self.swash_cache,
                    &mut self.text_layout_cache,
                    &mut self.pixmap,
                )?;
                let pixmap = self
                    .pixmap
                    .as_ref()
                    .ok_or(MochiOsBackendError::InvalidWindowSize)?;
                attach_buffer(
                    compositor,
                    token,
                    window.width() as usize,
                    window.height() as usize,
                    pixmap,
                    clear_color,
                    &mut shared_buffer,
                    window.viewport(),
                    dirty_bounds,
                )?;
                damage_token_request(compositor, token, window.viewport(), dirty_bounds)?;
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

    fn handle_event_message(
        &mut self,
        len: usize,
        event: [u8; 32],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        if self.direct_input && len == INPUT_EVENT_SIZE && self.handle_input_event(event, window) {
            return Ok(());
        }

        self.handle_compositor_event(event, window)
    }

    fn handle_input_event(&mut self, event: [u8; 32], window: &MochiOsWindow) -> bool {
        let kind = u16::from_le_bytes([event[0], event[1]]);
        match kind {
            INPUT_EVENT_KIND_POINTER_MOVE => {
                let dx = i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let dy = i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                let bounds = window.viewport().logical_bounds();
                let max_x = (bounds.origin.x + bounds.size.width).max(bounds.origin.x);
                let max_y = (bounds.origin.y + bounds.size.height).max(bounds.origin.y);
                self.pointer_x = (self.pointer_x + dx).clamp(bounds.origin.x, max_x);
                self.pointer_y = (self.pointer_y + dy).clamp(bounds.origin.y, max_y);
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: self.pointer_x,
                        y: self.pointer_y,
                    },
                    window,
                );
                true
            }
            INPUT_EVENT_KIND_POINTER_ABSOLUTE => {
                let raw_x = i32::from_le_bytes([event[12], event[13], event[14], event[15]])
                    .clamp(0, 32_767) as f32;
                let raw_y = i32::from_le_bytes([event[16], event[17], event[18], event[19]])
                    .clamp(0, 32_767) as f32;
                let bounds = window.viewport().logical_bounds();
                self.pointer_x = bounds.origin.x + (raw_x / 32_767.0) * bounds.size.width;
                self.pointer_y = bounds.origin.y + (raw_y / 32_767.0) * bounds.size.height;
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: self.pointer_x,
                        y: self.pointer_y,
                    },
                    window,
                );
                true
            }
            INPUT_EVENT_KIND_POINTER_BUTTON => {
                let flags = u16::from_le_bytes([event[2], event[3]]);
                let detail = u16::from_le_bytes([event[6], event[7]]);
                let button = match detail {
                    1 => PointerButton::Primary,
                    2 => PointerButton::Secondary,
                    3 => PointerButton::Middle,
                    other => PointerButton::Other(other),
                };
                let state = if flags & INPUT_FLAG_PRESS != 0 {
                    ButtonState::Pressed
                } else if flags & INPUT_FLAG_RELEASE != 0 {
                    ButtonState::Released
                } else {
                    return true;
                };
                self.app
                    .handle_event(PlatformEvent::PointerButton { button, state }, window);
                true
            }
            _ => false,
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
                let state = if flags & u32::from(INPUT_FLAG_PRESS) != 0 {
                    if !self.pressed_buttons.contains(&button_id) {
                        self.pressed_buttons.push(button_id);
                    }
                    ButtonState::Pressed
                } else if flags & u32::from(INPUT_FLAG_RELEASE) != 0 {
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
            EVENT_FRAME_DONE => {}
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

fn find_display_driver() -> Result<u64, MochiOsBackendError> {
    let name = DISPLAY_SERVICE_NAME.as_bytes();
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
    Err(MochiOsBackendError::InvalidReply)
}

fn find_input_service() -> Result<u64, MochiOsBackendError> {
    let name = INPUT_SERVICE_NAME.as_bytes();
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
    Err(MochiOsBackendError::InvalidReply)
}

fn subscribe_input_events(endpoint: u64) -> bool {
    let Ok(input) = find_input_service() else {
        return false;
    };
    let request = core::ptr::addr_of_mut!(INPUT_SUBSCRIBE_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(INPUT_SUBSCRIBE_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 16);
        put_u32_raw(request, 0, INPUT_SUBSCRIBE_OPCODE);
        put_u64_raw(request, 8, endpoint);
        zero_raw(reply, 1);
    }
    matches!(ipc_call_raw(input, request, 16, reply, 1), Ok(1))
}

fn require_window_overlay_capability() -> Result<(), MochiOsBackendError> {
    if query_capability(WINDOW_OVERLAY_CAPABILITY) {
        return Ok(());
    }
    request_capability_from_shell(
        WINDOW_OVERLAY_CAPABILITY,
        Some("fullscreen desktop surface"),
    )?;
    if query_capability(WINDOW_OVERLAY_CAPABILITY) {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))
    }
}

fn query_capability(capability: &str) -> bool {
    let bytes = capability.as_bytes();
    matches!(
        syscall::call2(
            syscall::SyscallNumber::CapQuery,
            bytes.as_ptr() as u64,
            bytes.len() as u64,
        ),
        Ok(1)
    )
}

fn request_capability_from_shell(
    capability: &str,
    reason: Option<&str>,
) -> Result<(), MochiOsBackendError> {
    let prompt_mode = env::var("MOCHI_PROMPT_MODE")
        .map_err(|_| MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))?;
    if prompt_mode != "interactive" {
        return Err(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES));
    }
    let shell_endpoint = env::var("MOCHI_SHELL_ENDPOINT")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .filter(|endpoint| *endpoint != 0)
        .ok_or(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))?;
    let executable = env::var("MOCHI_EXECUTABLE_PATH")
        .or_else(|_| env::args().next().ok_or(env::VarError::NotPresent))
        .map_err(|_| MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))?;
    let process_id = syscall_result(syscall::call0(syscall::SyscallNumber::GetPid))?;
    let exec_bytes = executable.as_bytes();
    let cap_bytes = capability.as_bytes();
    let reason_bytes = reason.unwrap_or("").as_bytes();
    if exec_bytes.len() > 256 || cap_bytes.len() > 64 || reason_bytes.len() > 128 {
        return Err(MochiOsBackendError::InvalidWindowSize);
    }

    let mut request = CapabilityRequest {
        opcode: CAPABILITY_PROMPT_OPCODE,
        process_id,
        executable: CapabilityExecutableIdentity::default(),
        capability_class: CapabilityClass::UserGrantable,
        capability_len: cap_bytes.len() as u16,
        resource: CapabilityResourceDescriptor::default(),
        reason_len: reason_bytes.len() as u16,
        interactive: 1,
        decision_scope: 0,
        reserved0: 0,
        capability: [0; 64],
        reason: [0; 128],
    };
    request.executable.path_len = exec_bytes.len() as u16;
    request.executable.path[..exec_bytes.len()].copy_from_slice(exec_bytes);
    request.capability[..cap_bytes.len()].copy_from_slice(cap_bytes);
    request.reason[..reason_bytes.len()].copy_from_slice(reason_bytes);

    let mut reply = [0u8; 8];
    let msg = syscall_result(syscall::call5(
        syscall::SyscallNumber::IpcCall,
        shell_endpoint,
        (&request as *const CapabilityRequest) as u64,
        core::mem::size_of::<CapabilityRequest>() as u64,
        reply.as_mut_ptr() as u64,
        reply.len() as u64,
    ))?;
    if (msg & 0xffff_ffff) < 4 {
        return Err(MochiOsBackendError::InvalidReply);
    }
    let decision = u32::from_le_bytes([reply[0], reply[1], reply[2], reply[3]]);
    if decision == CapabilityDecision::AllowOnce as u32
        || decision == CapabilityDecision::AllowForProcess as u32
        || decision == CapabilityDecision::AllowPersistently as u32
        || decision == CapabilityDecision::AllowAllUserGrantable as u32
    {
        Ok(())
    } else {
        Err(MochiOsBackendError::Syscall(mochi_user_syscall::EACCES))
    }
}

fn display_surface_size() -> Option<crate::geometry::Size> {
    let display = find_display_driver().ok()?;
    let request = core::ptr::addr_of_mut!(DISPLAY_REQ).cast::<u8>();
    let reply = core::ptr::addr_of_mut!(DISPLAY_REPLY).cast::<u8>();
    unsafe {
        zero_raw(request, 20);
        zero_raw(reply, 32);
        put_u32_raw(request, 0, DISPLAY_GET_INFO_OPCODE);
    }
    let len = ipc_call_raw(display, request, 20, reply, 32).ok()?;
    if len < 20 {
        return None;
    }
    let status = unsafe { read_u32_raw(reply.cast_const(), 0) };
    if status != 0 {
        return None;
    }
    let width = unsafe { read_u32_raw(reply.cast_const(), 4) };
    let height = unsafe { read_u32_raw(reply.cast_const(), 8) };
    match (width, height) {
        (w, h) if w > 0 && h > 0 => Some(crate::geometry::Size::new(w as f32, h as f32)),
        _ => None,
    }
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
    sent_pages: bool,
}

#[derive(Clone, Copy)]
struct PhysicalDirtyRect {
    x: usize,
    y: usize,
    width: usize,
    height: usize,
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
            sent_pages: false,
        })
    }

    fn send_pixmap_to(
        &mut self,
        compositor: u64,
        pixmap: &Pixmap,
        background: Color,
        dirty_rect: PhysicalDirtyRect,
    ) -> Result<(), MochiOsBackendError> {
        let pixel_count = (pixmap.width() as usize)
            .checked_mul(pixmap.height() as usize)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        let bytes_len = pixel_count
            .checked_mul(4)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if bytes_len > self.byte_capacity {
            return Err(MochiOsBackendError::InvalidWindowSize);
        }
        let dst =
            unsafe { std::slice::from_raw_parts_mut(self.virt as *mut u8, self.byte_capacity) };

        let pixmap_width = pixmap.width() as usize;
        let pixmap_height = pixmap.height() as usize;
        let copy_rect = if self.sent_pages {
            dirty_rect
        } else {
            PhysicalDirtyRect {
                x: 0,
                y: 0,
                width: pixmap_width,
                height: pixmap_height,
            }
        };
        let right = copy_rect
            .x
            .saturating_add(copy_rect.width)
            .min(pixmap_width);
        let bottom = copy_rect
            .y
            .saturating_add(copy_rect.height)
            .min(pixmap_height);
        let src = pixmap.data();
        for y in copy_rect.y..bottom {
            let Some(row_start) = y.checked_mul(pixmap_width) else {
                return Err(MochiOsBackendError::ArithmeticOverflow);
            };
            for x in copy_rect.x..right {
                let Some(pixel_index) = row_start.checked_add(x) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(byte_index) = pixel_index.checked_mul(4) else {
                    return Err(MochiOsBackendError::ArithmeticOverflow);
                };
                let Some(pixel) = src.get(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let Some(out) = dst.get_mut(byte_index..byte_index + 4) else {
                    return Err(MochiOsBackendError::InvalidWindowSize);
                };
                let value = flatten_premultiplied_pixel(pixel, background);
                out.copy_from_slice(&value.to_le_bytes());
            }
        }
        let page_count = bytes_len
            .checked_add(PAGE_SIZE - 1)
            .map(|len| len / PAGE_SIZE)
            .ok_or(MochiOsBackendError::ArithmeticOverflow)?;
        if self.sent_pages {
            return Ok(());
        }
        send_pages(compositor, page_count, self.virt)?;
        self.sent_pages = true;
        Ok(())
    }
}

fn flatten_premultiplied_pixel(pixel: &[u8], background: Color) -> u32 {
    let alpha = pixel[3] as u32;
    let inv_alpha = 255_u32.saturating_sub(alpha);

    // tiny-skia stores premultiplied RGBA. The compositor surface is XRGB,
    // so each pixel is flattened into the configured clear color.
    let red = pixel[0] as u32 + (background.red as u32 * inv_alpha + 127) / 255;
    let green = pixel[1] as u32 + (background.green as u32 * inv_alpha + 127) / 255;
    let blue = pixel[2] as u32 + (background.blue as u32 * inv_alpha + 127) / 255;

    0xff00_0000 | (red.min(255) << 16) | (green.min(255) << 8) | blue.min(255)
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

fn try_recv_event() -> Result<Option<(usize, [u8; 32])>, MochiOsBackendError> {
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
    Ok(Some((len, out)))
}

fn wait_for_event<A: PlatformApplication>(
    endpoint: u64,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<(), MochiOsBackendError> {
    if let Some((len, event)) = read_event_blocking(endpoint)? {
        backend.handle_event_message(len, event, window)?;
    }
    Ok(())
}

fn wait_until_deadline<A: PlatformApplication>(
    deadline: Instant,
    window: &MochiOsWindow,
    backend: &mut MochiOsBackend<A>,
) -> Result<bool, MochiOsBackendError> {
    loop {
        if let Some((len, event)) = try_recv_event()? {
            backend.handle_event_message(len, event, window)?;
            return Ok(true);
        }
        if Instant::now() >= deadline {
            return Ok(false);
        }
        let _ = syscall::call0(syscall::SyscallNumber::ThreadYield);
    }
}

fn read_event_blocking(endpoint: u64) -> Result<Option<(usize, [u8; 32])>, MochiOsBackendError> {
    let event = core::ptr::addr_of_mut!(EVENT_BUF).cast::<u8>();
    let len = match ipc_wait_raw(endpoint, event, 32) {
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
    Ok(Some((len, out)))
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
    pixmap: &Pixmap,
    background: Color,
    shared_buffer: &mut SharedBuffer,
    viewport: Viewport,
    dirty_bounds: Rect,
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
    let dirty_rect = physical_dirty_rect(viewport, dirty_bounds);
    shared_buffer.send_pixmap_to(compositor, pixmap, background, dirty_rect)
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

fn physical_dirty_rect(viewport: Viewport, dirty_bounds: Rect) -> PhysicalDirtyRect {
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

fn damage_token_request(
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

fn render_display_list(
    viewport: Viewport,
    dirty_bounds: Rect,
    display_list: &DisplayList,
    font_system: &mut FontSystem,
    swash_cache: &mut SwashCache,
    text_layout_cache: &mut HashMap<TextLayoutKey, Buffer>,
    pixmap: &mut Option<Pixmap>,
) -> Result<Color, MochiOsBackendError> {
    let width = viewport.physical_width;
    let height = viewport.physical_height;
    let pixmap = reusable_pixmap(pixmap, width, height)?;
    let mut clear_color = Color::BLACK;

    let scale = valid_scale_factor(viewport.scale_factor);
    let transform = Transform::from_scale(scale, scale);
    let bounds = viewport.logical_bounds();
    let dirty_bounds = dirty_bounds.intersection(bounds).unwrap_or(bounds);
    let mut clip_stack = vec![create_clip_mask(
        dirty_bounds,
        None,
        width,
        height,
        transform,
    )?];

    for command in display_list.commands() {
        match command {
            DrawCommand::Clear { color } => {
                clear_color = *color;
                if let Some(rect) = to_skia_rect(dirty_bounds) {
                    let paint = solid_paint(*color);
                    pixmap.fill_rect(rect, &paint, transform, clip_stack.last());
                }
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
                    &mut *pixmap,
                    font_system,
                    swash_cache,
                    text_layout_cache,
                    command,
                    scale,
                    clip_stack.last(),
                );
            }
            DrawCommand::DrawSvg { command } => {
                if command.bounds.intersection(bounds).is_none() {
                    continue;
                }
                draw_svg_command(pixmap, command, scale, clip_stack.last())?;
            }
            DrawCommand::DrawImage { .. } => {}
        }
    }

    Ok(clear_color)
}

fn reusable_pixmap(
    pixmap: &mut Option<Pixmap>,
    width: u32,
    height: u32,
) -> Result<&mut Pixmap, MochiOsBackendError> {
    let needs_allocate = pixmap
        .as_ref()
        .is_none_or(|current| current.width() != width || current.height() != height);
    if needs_allocate {
        *pixmap = Some(Pixmap::new(width, height).ok_or(MochiOsBackendError::InvalidWindowSize)?);
    }
    pixmap
        .as_mut()
        .ok_or(MochiOsBackendError::InvalidWindowSize)
}

fn valid_scale_factor(scale_factor: f64) -> f32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor as f32
    } else {
        1.0
    }
}

fn draw_svg_command(
    target: &mut Pixmap,
    command: &SvgCommand,
    display_scale: f32,
    clip: Option<&Mask>,
) -> Result<(), MochiOsBackendError> {
    let bounds = command.bounds;
    if !is_valid_image_bounds(bounds) {
        return Ok(());
    }

    let svg_width = command.svg.width();
    let svg_height = command.svg.height();
    if !svg_width.is_finite() || !svg_height.is_finite() || svg_width <= 0.0 || svg_height <= 0.0 {
        return Ok(());
    }

    let destination_width = bounds.size.width * display_scale;
    let destination_height = bounds.size.height * display_scale;
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return Ok(());
    }

    let raster_width = destination_width.ceil() as u32;
    let raster_height = destination_height.ceil() as u32;
    if raster_width == 0 || raster_height == 0 {
        return Ok(());
    }

    let mut raster =
        Pixmap::new(raster_width, raster_height).ok_or(MochiOsBackendError::InvalidWindowSize)?;
    let render_transform = Transform::from_scale(
        raster_width as f32 / svg_width,
        raster_height as f32 / svg_height,
    );
    resvg::render(command.svg.tree(), render_transform, &mut raster.as_mut());

    if let Some(tint) = command.tint {
        tint_svg_pixmap(&mut raster, tint);
    }

    let translate_x = bounds.origin.x * display_scale;
    let translate_y = bounds.origin.y * display_scale;
    if !translate_x.is_finite() || !translate_y.is_finite() {
        return Ok(());
    }

    let paint = PixmapPaint {
        opacity: sanitize_image_opacity(command.opacity),
        quality: FilterQuality::Bicubic,
        ..PixmapPaint::default()
    };
    target.draw_pixmap(
        translate_x.round() as i32,
        translate_y.round() as i32,
        raster.as_ref(),
        &paint,
        Transform::identity(),
        clip,
    );

    Ok(())
}

fn svg_supersample_scale(destination_width: f32, destination_height: f32) -> f32 {
    if !destination_width.is_finite()
        || !destination_height.is_finite()
        || destination_width <= 0.0
        || destination_height <= 0.0
    {
        return 1.0;
    }

    if destination_width.max(destination_height) <= SVG_SMALL_RENDER_LIMIT {
        SVG_SMALL_RENDER_SUPERSAMPLE
    } else {
        1.0
    }
}

fn is_valid_image_bounds(bounds: Rect) -> bool {
    bounds.origin.x.is_finite()
        && bounds.origin.y.is_finite()
        && bounds.size.width.is_finite()
        && bounds.size.height.is_finite()
        && bounds.size.width > 0.0
        && bounds.size.height > 0.0
}

fn sanitize_image_opacity(opacity: f32) -> f32 {
    if opacity.is_finite() {
        opacity.clamp(0.0, 1.0)
    } else {
        1.0
    }
}

fn tint_svg_pixmap(pixmap: &mut Pixmap, tint: Color) {
    for pixel in pixmap.data_mut().chunks_exact_mut(4) {
        let alpha = multiply_channel(pixel[3], tint.alpha);

        pixel[0] = multiply_channel(tint.red, alpha);
        pixel[1] = multiply_channel(tint.green, alpha);
        pixel[2] = multiply_channel(tint.blue, alpha);
        pixel[3] = alpha;
    }
}

fn multiply_channel(first: u8, second: u8) -> u8 {
    let value = u16::from(first) * u16::from(second);

    ((value + 127) / 255) as u8
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
