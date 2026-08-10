use std::cell::Cell;
use std::collections::HashMap;
use std::fmt::Write as _;
use std::time::Instant;

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, Family, FontSystem, Metrics, Shaping, SwashCache, Weight,
};
use mochi_user_syscall as syscall;
use tiny_skia::{
    BlendMode, FillRule, FilterQuality, Mask, Paint, Path, PathBuilder, Pixmap, PixmapPaint,
    Rect as SkiaRect, Stroke, Transform,
};

use crate::draw_command::{
    DisplayList, DrawCommand, ImageCommand, ImageSampling, SvgCommand, TextCommand,
};
use crate::font::create_font_system;
use crate::geometry::{Rect, Size};
use crate::image::ImageData;
use crate::platform::{
    ButtonState, CursorIcon, Key, KeyModifiers, PlatformApplication, PlatformEvent, PlatformWindow,
    PointerButton, WindowConfig,
};
use crate::renderer::Viewport;
use crate::svg::SvgData;
use crate::theme::Color;

mod buffer;
mod connection;
mod gpu_renderer;
mod present;
mod renderer;
mod surface;
mod window;

use buffer::{PhysicalDirtyRect, SharedBuffer};
use connection::*;
use gpu_renderer::GpuSceneRenderer;
use present::damage_token_request;
pub use renderer::render_offscreen_xrgb;
use renderer::{TextLayoutKey, render_display_list};
use surface::{
    CompositorSurface, attach_buffer, attach_gpu_scene, renderer_caps, set_cursor_image,
    set_cursor_position, simple_token_request,
};
use window::{MochiOsWindow, checked_surface_size};

const COMPOSITOR_SERVICE_NAME: &str = "compositor.service";
const DISPLAY_SERVICE_NAME: &str = "display.driver";
const INPUT_SERVICE_NAME: &str = "input.service";
const WINDOW_OVERLAY_CAPABILITY: &str = "window.overlay";
const WINDOW_SECURE_OVERLAY_CAPABILITY: &str = "window.secure-overlay";
const DISPLAY_GET_INFO_OPCODE: u32 = 1;
const OP_CREATE_SURFACE: u32 = 1;
const OP_ATTACH_BUFFER: u32 = 2;
const OP_DAMAGE: u32 = 3;
const OP_COMMIT: u32 = 4;
const OP_DESTROY_SURFACE: u32 = 6;
const OP_SET_CURSOR_POSITION: u32 = 7;
const OP_SET_CURSOR_IMAGE: u32 = 8;
const OP_GET_RENDERER_CAPS: u32 = 9;
const OP_CONTEXT_MENU_SHOW: u32 = 121;
const OP_APPEARANCE_CHANGED: u32 = 123;
const ROLE_TOPLEVEL: u32 = 1;
const ROLE_BACKGROUND: u32 = 3;
const ROLE_PANEL: u32 = 4;
const ROLE_SECURE_OVERLAY: u32 = 5;
const PIXEL_FORMAT_XRGB8888: u32 = 1;
const PIXEL_FORMAT_ARGB8888_PREMULTIPLIED: u32 = 2;
const PIXEL_FORMAT_GPU_SCENE: u32 = 3;
const RENDERER_CAP_GPU_SCENE: u32 = 1;
const PAGE_SIZE: usize = 4096;
const MAX_SURFACE_EXTENT: u32 = 16_384;
const ERRNO_EAGAIN: u64 = 11;
const EVENT_POINTER_ENTER: u32 = 2;
const EVENT_POINTER_LEAVE: u32 = 3;
const EVENT_POINTER_MOTION: u32 = 4;
const EVENT_POINTER_BUTTON: u32 = 5;
const EVENT_KEY: u32 = 6;
const EVENT_CLOSE_REQUESTED: u32 = 7;
const EVENT_FOCUS_GAINED: u32 = 8;
const EVENT_FOCUS_LOST: u32 = 9;
const EVENT_FRAME_DONE: u32 = 10;
const EVENT_CONFIGURE: u32 = 11;
const EVENT_POINTER_SCROLL: u32 = 12;
const EVENT_CONTEXT_MENU_RESULT: u32 = 13;
const EVENT_APPEARANCE_CHANGED: u32 = 14;
const INPUT_SUBSCRIBE_OPCODE: u32 = 0x5355_4253;
const INPUT_EVENT_SIZE: usize = 32;
const INPUT_EVENT_KIND_POINTER_MOVE: u16 = 2;
const INPUT_EVENT_KIND_POINTER_BUTTON: u16 = 3;
const INPUT_EVENT_KIND_POINTER_WHEEL: u16 = 4;
const INPUT_EVENT_KIND_POINTER_ABSOLUTE: u16 = 5;
const KEY_ESCAPE: u16 = 1;
const KEY_BACKSPACE: u16 = 2;
const KEY_TAB: u16 = 3;
const KEY_ENTER: u16 = 4;
const KEY_SPACE: u16 = 5;
const KEY_A: u16 = 32;
const KEY_DELETE: u16 = 79;
const KEY_HOME: u16 = 80;
const KEY_END: u16 = 81;
const KEY_LEFT: u16 = 82;
const KEY_RIGHT: u16 = 83;
const KEY_UP: u16 = 84;
const KEY_DOWN: u16 = 85;
const KEY_PAGE_UP: u16 = 86;
const KEY_PAGE_DOWN: u16 = 87;
const INPUT_FLAG_PRESS: u16 = 1 << 0;
const INPUT_FLAG_RELEASE: u16 = 1 << 1;
const INPUT_MOD_SHIFT: u32 = 1 << 0;
const INPUT_MOD_CONTROL: u32 = 1 << 1;
const INPUT_MOD_ALT: u32 = 1 << 2;
const TEXT_LAYOUT_CACHE_CAPACITY: usize = 1024;
const CURSOR_SVG_PATH: &str = "/system/icons/cursor.svg";
const CURSOR_WIDTH: u32 = 12;
const CURSOR_HEIGHT: u32 = 20;
const CURSOR_HOTSPOT_X: f32 = 1.0;
const CURSOR_HOTSPOT_Y: f32 = 1.0;
const PERF_LOG_ENABLED: bool = false;
const METRICS_INTERVAL_TICKS: u64 = 500;
const SLOW_FRAME_THRESHOLD_TICKS: u64 = 16;
const INITIAL_FRAME_LOGS: u64 = 8;

static mut CREATE_SURFACE_REQ: [u8; 24] = [0; 24];
static mut ATTACH_BUFFER_REQ: [u8; 28] = [0; 28];
static mut TOKEN_REQ: [u8; 12] = [0; 12];
static mut DAMAGE_REQ: [u8; 28] = [0; 28];
static mut IPC_REPLY: [u8; 16] = [0; 16];
const EVENT_BUFFER_SIZE: usize = 4096;
static mut EVENT_BUF: [u8; EVENT_BUFFER_SIZE] = [0; EVENT_BUFFER_SIZE];
static mut DISPLAY_REQ: [u8; 20] = [0; 20];
static mut DISPLAY_REPLY: [u8; 32] = [0; 32];
static mut INPUT_SUBSCRIBE_REQ: [u8; 16] = [0; 16];
static mut INPUT_SUBSCRIBE_REPLY: [u8; 1] = [0; 1];

#[derive(Debug, thiserror::Error)]
pub enum MochiOsBackendError {
    #[error("mochiOS syscall failed: {0}")]
    Syscall(u64),

    #[error("mochiOS {stage} failed: {errno}")]
    Stage { stage: &'static str, errno: u64 },

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

impl MochiOsBackendError {
    fn at(self, stage: &'static str) -> Self {
        match self {
            Self::Syscall(errno) => Self::Stage { stage, errno },
            error => error,
        }
    }
}

/// Owns a compositor background surface for the lifetime of the desktop session.
#[must_use = "dropping the desktop background removes its compositor surface"]
pub struct DesktopBackground {
    _surface: CompositorSurface,
    _buffer: SharedBuffer,
}

impl DesktopBackground {
    /// Creates a screen-filling, aspect-preserving background image.
    pub fn from_image(image: ImageData) -> Result<Self, MochiOsBackendError> {
        require_window_overlay_capability()?;
        let compositor = find_compositor()?;
        let requested_size =
            display_surface_size().ok_or(MochiOsBackendError::InvalidWindowSize)?;
        let (width, height) = checked_surface_size(requested_size)?;
        let viewport = Viewport::new(requested_size, width, height, 1.0);
        let surface = CompositorSurface::create(compositor, 0, ROLE_BACKGROUND, width, height)?;
        let mut buffer = SharedBuffer::new(width as usize, height as usize)?;

        let image_scale =
            (width as f32 / image.width() as f32).max(height as f32 / image.height() as f32);
        let image_width = image.width() as f32 * image_scale;
        let image_height = image.height() as f32 * image_scale;
        let image_bounds = Rect::new(
            (width as f32 - image_width) * 0.5,
            (height as f32 - image_height) * 0.5,
            image_width,
            image_height,
        );
        let mut display_list = DisplayList::new();
        display_list.push(DrawCommand::Clear {
            color: Color::BLACK,
        });
        display_list.push(DrawCommand::DrawImage {
            command: ImageCommand {
                image,
                bounds: image_bounds,
                opacity: 1.0,
                sampling: ImageSampling::Bicubic,
            },
        });

        let mut font_system = create_font_system();
        let mut swash_cache = SwashCache::new();
        let mut text_layout_cache = HashMap::new();
        let mut pixmap = None;
        let mut clip_masks = Vec::new();
        let bounds = viewport.logical_bounds();
        let clear_color = render_display_list(
            viewport,
            bounds,
            &display_list,
            &mut font_system,
            &mut swash_cache,
            &mut text_layout_cache,
            &mut pixmap,
            &mut clip_masks,
            false,
        )?;
        let pixmap = pixmap
            .as_ref()
            .ok_or(MochiOsBackendError::InvalidWindowSize)?;
        attach_buffer(
            compositor,
            surface.token(),
            width as usize,
            height as usize,
            pixmap,
            clear_color,
            &mut buffer,
            viewport,
            bounds,
            PIXEL_FORMAT_XRGB8888,
        )?;
        damage_token_request(compositor, surface.token(), viewport, bounds)?;
        simple_token_request(compositor, OP_COMMIT, surface.token())?;

        Ok(Self {
            _surface: surface,
            _buffer: buffer,
        })
    }
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
    clip_masks: Vec<Mask>,
    gpu_renderer: GpuSceneRenderer,
    gpu_scene: Vec<u8>,
    direct_input: bool,
    pointer_x: f32,
    pointer_y: f32,
    cursor_image: Option<ImageData>,
    cursor_dirty: Option<Rect>,
    clear_color: Color,
    pending_pointer_motion: PendingPointerMotion,
    pending_resize: Option<(u32, u32)>,
    close_requested: bool,
    metrics: BackendMetrics,
}

#[derive(Default)]
struct PendingPointerMotion {
    absolute: Option<(f32, f32)>,
    compositor_position: Option<(f32, f32)>,
    relative_dx: f32,
    relative_dy: f32,
    pending: bool,
}

#[derive(Default)]
struct BackendMetrics {
    next_report_tick: u64,
    full_frames: u64,
    cursor_frames: u64,
    frame_logs_emitted: u64,
    input_events: u64,
    coalesced_pointer_events: u64,
    draw_cycles: u64,
    render_cycles: u64,
    attach_cycles: u64,
    commit_cycles: u64,
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
            clip_masks: Vec::new(),
            gpu_renderer: GpuSceneRenderer::new(),
            gpu_scene: Vec::new(),
            direct_input: false,
            pointer_x: 0.0,
            pointer_y: 0.0,
            cursor_image: None,
            cursor_dirty: None,
            clear_color: Color::BLACK,
            pending_pointer_motion: PendingPointerMotion::default(),
            pending_resize: None,
            close_requested: false,
            metrics: BackendMetrics::default(),
        }
    }

    pub fn run(mut self) -> Result<(), MochiOsBackendError> {
        let compositor = find_compositor()?;
        let event_endpoint = create_event_endpoint()?;
        if self.config.secure_overlay {
            require_window_secure_overlay_capability()?;
        } else if self.config.fullscreen {
            require_window_overlay_capability()?;
        }
        let requested_size = if self.config.fullscreen {
            display_surface_size().unwrap_or_else(|| self.config.size)
        } else {
            self.config.size
        };
        let size = checked_surface_size(requested_size)?;
        let viewport = scaled_viewport(size.0, size.1, self.app.interface_scale_factor());
        let role = if self.config.secure_overlay {
            ROLE_SECURE_OVERLAY
        } else if self.config.fullscreen {
            ROLE_PANEL
        } else {
            ROLE_TOPLEVEL
        };
        let pixel_format = if self.config.fullscreen {
            PIXEL_FORMAT_ARGB8888_PREMULTIPLIED
        } else {
            PIXEL_FORMAT_XRGB8888
        };
        let surface = CompositorSurface::create(compositor, event_endpoint, role, size.0, size.1)
            .map_err(|error| error.at("surface creation"))?;
        let token = surface.token();
        let window = MochiOsWindow::new(viewport, compositor, token);
        let mut gpu_enabled = renderer_caps(compositor) & RENDERER_CAP_GPU_SCENE != 0;
        let mut shared_buffer = if gpu_enabled {
            SharedBuffer::new_gpu_scene(size.0 as usize, size.1 as usize)
                .map_err(|error| error.at("GPU shared buffer allocation"))?
        } else {
            SharedBuffer::new(size.0 as usize, size.1 as usize)
                .map_err(|error| error.at("pixel shared buffer allocation"))?
        };
        self.pointer_x = (viewport.logical_size.width / 2.0).max(0.0);
        self.pointer_y = (viewport.logical_size.height / 2.0).max(0.0);
        self.direct_input = false;
        if self.config.fullscreen {
            let cursor_image = load_cursor_image();
            if let Some(image) = cursor_image.as_ref() {
                set_cursor_image(compositor, image)?;
            }
            self.cursor_image = cursor_image;
        }
        self.log_backend_started(size);

        self.app
            .handle_event(PlatformEvent::Resumed { viewport }, &window);
        window.request_redraw();

        let mut display_list = DisplayList::new();
        'event_loop: loop {
            let mut handled_work = false;

            while let Some((len, event)) = try_recv_event()? {
                self.handle_or_queue_event_message(len, event, &window)?;
                handled_work = true;
            }
            if self.close_requested || self.app.exit_requested() {
                break 'event_loop Ok(());
            }
            if let Some((width, height)) = self.pending_resize.take() {
                let viewport = scaled_viewport(width, height, self.app.interface_scale_factor());
                window.set_viewport(viewport);
                shared_buffer = if gpu_enabled {
                    SharedBuffer::new_gpu_scene(width as usize, height as usize)?
                } else {
                    SharedBuffer::new(width as usize, height as usize)?
                };
                self.pixmap = None;
                self.clip_masks.clear();
                self.app
                    .handle_event(PlatformEvent::Resized { viewport }, &window);
                window.request_redraw();
                handled_work = true;
            }
            if self.flush_pending_pointer_motion(&window) {
                handled_work = true;
            }

            let redraw_due = self
                .app
                .next_redraw_at()
                .is_some_and(|deadline| deadline <= Instant::now());

            let redraw_requested = window.take_redraw_requested();
            if redraw_requested || redraw_due {
                if self.font_system.is_none() {
                    self.font_system = Some(create_font_system());
                }
                display_list.clear();
                let frame_start = perf_counter();
                let frame_tick_start = perf_tick();
                let draw_start = perf_counter();
                let mut dirty_bounds = self.app.draw(window.viewport(), &mut display_list);
                let draw_cycles = perf_counter_elapsed(draw_start);
                self.metrics.draw_cycles = self.metrics.draw_cycles.saturating_add(draw_cycles);
                if let Some(cursor_rect) = self.current_cursor_rect(window.viewport()) {
                    dirty_bounds = dirty_bounds.union(cursor_rect);
                }
                self.cursor_dirty = None;
                let render_start = perf_counter();
                let mut gpu_scene = None;
                if gpu_enabled {
                    match self.gpu_renderer.render(
                        window.viewport(),
                        dirty_bounds,
                        &display_list,
                        self.font_system
                            .as_mut()
                            .ok_or(MochiOsBackendError::InvalidWindowSize)?,
                        &mut self.swash_cache,
                        &mut self.text_layout_cache,
                        self.config.fullscreen,
                        &mut self.gpu_scene,
                    ) {
                        Ok(()) => gpu_scene = Some(self.gpu_scene.as_slice()),
                        Err(_) => {
                            gpu_enabled = false;
                            shared_buffer = SharedBuffer::new(
                                window.width() as usize,
                                window.height() as usize,
                            )?;
                        }
                    }
                }
                let clear_color = if gpu_scene.is_some() {
                    Color::TRANSPARENT
                } else {
                    render_display_list(
                        window.viewport(),
                        dirty_bounds,
                        &display_list,
                        self.font_system
                            .as_mut()
                            .ok_or(MochiOsBackendError::InvalidWindowSize)?,
                        &mut self.swash_cache,
                        &mut self.text_layout_cache,
                        &mut self.pixmap,
                        &mut self.clip_masks,
                        self.config.fullscreen,
                    )?
                };
                let render_cycles = perf_counter_elapsed(render_start);
                self.metrics.render_cycles =
                    self.metrics.render_cycles.saturating_add(render_cycles);
                self.clear_color = clear_color;
                let attach_start = perf_counter();
                if let Some(scene) = gpu_scene.as_deref() {
                    attach_gpu_scene(
                        compositor,
                        token,
                        window.width() as usize,
                        window.height() as usize,
                        scene,
                        &mut shared_buffer,
                    )
                    .map_err(|error| error.at("GPU scene attach"))?;
                } else {
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
                        pixel_format,
                    )
                    .map_err(|error| error.at("pixel buffer attach"))?;
                }
                let attach_cycles = perf_counter_elapsed(attach_start);
                self.metrics.attach_cycles =
                    self.metrics.attach_cycles.saturating_add(attach_cycles);
                let commit_start = perf_counter();
                damage_token_request(compositor, token, window.viewport(), dirty_bounds)
                    .map_err(|error| error.at("surface damage"))?;
                simple_token_request(compositor, OP_COMMIT, token)
                    .map_err(|error| error.at("surface commit"))?;
                let commit_cycles = perf_counter_elapsed(commit_start);
                self.metrics.commit_cycles =
                    self.metrics.commit_cycles.saturating_add(commit_cycles);
                self.metrics.full_frames = self.metrics.full_frames.saturating_add(1);
                self.report_frame_timing(
                    "full",
                    perf_counter_elapsed(frame_start),
                    perf_tick_elapsed(frame_tick_start),
                    draw_cycles,
                    render_cycles,
                    attach_cycles,
                    commit_cycles,
                    dirty_bounds,
                );
                self.report_metrics_if_due();
                handled_work = true;
            } else if let Some(dirty_bounds) = self.cursor_dirty.take() {
                if self.cursor_image.is_some() {
                    let frame_start = perf_counter();
                    let frame_tick_start = perf_tick();
                    let commit_start = perf_counter();
                    let scale = window.viewport().scale_factor as f32;
                    set_cursor_position(
                        compositor,
                        self.pointer_x * scale,
                        self.pointer_y * scale,
                        true,
                    )?;
                    let commit_cycles = perf_counter_elapsed(commit_start);
                    self.metrics.commit_cycles =
                        self.metrics.commit_cycles.saturating_add(commit_cycles);
                    self.metrics.cursor_frames = self.metrics.cursor_frames.saturating_add(1);
                    self.report_frame_timing(
                        "cursor",
                        perf_counter_elapsed(frame_start),
                        perf_tick_elapsed(frame_tick_start),
                        0,
                        0,
                        0,
                        commit_cycles,
                        dirty_bounds,
                    );
                    self.report_metrics_if_due();
                    handled_work = true;
                }
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

    fn handle_or_queue_event_message(
        &mut self,
        len: usize,
        event: [u8; EVENT_BUFFER_SIZE],
        window: &MochiOsWindow,
    ) -> Result<(), MochiOsBackendError> {
        let message_len = len.min(event.len());
        if self.app.handle_platform_message(&event[..message_len]) {
            window.request_redraw();
            return Ok(());
        }

        let mut core_event = [0u8; 32];
        core_event.copy_from_slice(&event[..32]);
        if len >= 12 && self.queue_compositor_pointer_motion(core_event) {
            self.metrics.input_events = self.metrics.input_events.saturating_add(1);
            self.metrics.coalesced_pointer_events =
                self.metrics.coalesced_pointer_events.saturating_add(1);
            return Ok(());
        }
        if self.direct_input && len == INPUT_EVENT_SIZE && self.queue_pointer_motion(core_event) {
            self.metrics.input_events = self.metrics.input_events.saturating_add(1);
            self.metrics.coalesced_pointer_events =
                self.metrics.coalesced_pointer_events.saturating_add(1);
            return Ok(());
        }

        self.flush_pending_pointer_motion(window);
        self.metrics.input_events = self.metrics.input_events.saturating_add(1);
        self.handle_event_message(len, core_event, window)
    }

    fn queue_pointer_motion(&mut self, event: [u8; 32]) -> bool {
        let kind = u16::from_le_bytes([event[0], event[1]]);
        match kind {
            INPUT_EVENT_KIND_POINTER_MOVE => {
                let dx = i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let dy = i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                self.pending_pointer_motion.relative_dx += dx;
                self.pending_pointer_motion.relative_dy += dy;
                self.pending_pointer_motion.pending = true;
                true
            }
            INPUT_EVENT_KIND_POINTER_ABSOLUTE => {
                let raw_x = i32::from_le_bytes([event[12], event[13], event[14], event[15]])
                    .clamp(0, 32_767) as f32;
                let raw_y = i32::from_le_bytes([event[16], event[17], event[18], event[19]])
                    .clamp(0, 32_767) as f32;
                self.pending_pointer_motion.absolute = Some((raw_x, raw_y));
                self.pending_pointer_motion.relative_dx = 0.0;
                self.pending_pointer_motion.relative_dy = 0.0;
                self.pending_pointer_motion.pending = true;
                true
            }
            _ => false,
        }
    }

    fn queue_compositor_pointer_motion(&mut self, event: [u8; 32]) -> bool {
        if u32::from_le_bytes([event[0], event[1], event[2], event[3]]) != EVENT_POINTER_MOTION {
            return false;
        }
        let x = i32::from_le_bytes([event[4], event[5], event[6], event[7]]) as f32;
        let y = i32::from_le_bytes([event[8], event[9], event[10], event[11]]) as f32;
        self.pending_pointer_motion.absolute = None;
        self.pending_pointer_motion.compositor_position = Some((x, y));
        self.pending_pointer_motion.relative_dx = 0.0;
        self.pending_pointer_motion.relative_dy = 0.0;
        self.pending_pointer_motion.pending = true;
        true
    }

    fn flush_pending_pointer_motion(&mut self, window: &MochiOsWindow) -> bool {
        if !self.pending_pointer_motion.pending {
            return false;
        }

        let bounds = window.viewport().logical_bounds();
        if let Some((x, y)) = self.pending_pointer_motion.compositor_position.take() {
            let scale = window.viewport().scale_factor as f32;
            self.pointer_x = x / scale;
            self.pointer_y = y / scale;
        } else if let Some((raw_x, raw_y)) = self.pending_pointer_motion.absolute.take() {
            self.pointer_x = bounds.origin.x + (raw_x / 32_767.0) * bounds.size.width;
            self.pointer_y = bounds.origin.y + (raw_y / 32_767.0) * bounds.size.height;
        }
        let max_x = (bounds.origin.x + bounds.size.width).max(bounds.origin.x);
        let max_y = (bounds.origin.y + bounds.size.height).max(bounds.origin.y);
        let scale = window.viewport().scale_factor as f32;
        self.pointer_x = (self.pointer_x + self.pending_pointer_motion.relative_dx / scale)
            .clamp(bounds.origin.x, max_x);
        self.pointer_y = (self.pointer_y + self.pending_pointer_motion.relative_dy / scale)
            .clamp(bounds.origin.y, max_y);

        self.pending_pointer_motion.relative_dx = 0.0;
        self.pending_pointer_motion.relative_dy = 0.0;
        self.pending_pointer_motion.pending = false;

        self.app.handle_event(
            PlatformEvent::PointerMoved {
                x: self.pointer_x,
                y: self.pointer_y,
            },
            window,
        );
        true
    }

    fn handle_input_event(&mut self, event: [u8; 32], window: &MochiOsWindow) -> bool {
        let kind = u16::from_le_bytes([event[0], event[1]]);
        match kind {
            INPUT_EVENT_KIND_POINTER_MOVE => {
                let previous = self.current_cursor_rect(window.viewport());
                let dx = i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let dy = i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                let bounds = window.viewport().logical_bounds();
                let max_x = (bounds.origin.x + bounds.size.width).max(bounds.origin.x);
                let max_y = (bounds.origin.y + bounds.size.height).max(bounds.origin.y);
                let scale = window.viewport().scale_factor as f32;
                self.pointer_x = (self.pointer_x + dx / scale).clamp(bounds.origin.x, max_x);
                self.pointer_y = (self.pointer_y + dy / scale).clamp(bounds.origin.y, max_y);
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: self.pointer_x,
                        y: self.pointer_y,
                    },
                    window,
                );
                self.mark_cursor_dirty(window.viewport(), previous);
                true
            }
            INPUT_EVENT_KIND_POINTER_ABSOLUTE => {
                let previous = self.current_cursor_rect(window.viewport());
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
                self.mark_cursor_dirty(window.viewport(), previous);
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
            INPUT_EVENT_KIND_POINTER_WHEEL => {
                let delta_x =
                    i32::from_le_bytes([event[12], event[13], event[14], event[15]]) as f32;
                let delta_y =
                    i32::from_le_bytes([event[16], event[17], event[18], event[19]]) as f32;
                self.app
                    .handle_event(PlatformEvent::Scroll { delta_x, delta_y }, window);
                true
            }
            _ => false,
        }
    }

    fn report_metrics_if_due(&mut self) {
        if !PERF_LOG_ENABLED {
            return;
        }
        let now = perf_tick();
        if self.metrics.next_report_tick == 0 {
            self.metrics.next_report_tick = now.saturating_add(METRICS_INTERVAL_TICKS);
        }
        if now < self.metrics.next_report_tick {
            return;
        }

        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios stats: full={} cursor={} input={} coalesced={} draw={}cy render={}cy attach={}cy commit={}cy\n",
            self.metrics.full_frames,
            self.metrics.cursor_frames,
            self.metrics.input_events,
            self.metrics.coalesced_pointer_events,
            self.metrics.draw_cycles,
            self.metrics.render_cycles,
            self.metrics.attach_cycles,
            self.metrics.commit_cycles,
        );
        perf_log(&line);

        self.metrics.full_frames = 0;
        self.metrics.cursor_frames = 0;
        self.metrics.input_events = 0;
        self.metrics.coalesced_pointer_events = 0;
        self.metrics.draw_cycles = 0;
        self.metrics.render_cycles = 0;
        self.metrics.attach_cycles = 0;
        self.metrics.commit_cycles = 0;
        self.metrics.next_report_tick = now.saturating_add(METRICS_INTERVAL_TICKS);
    }

    fn report_frame_timing(
        &mut self,
        kind: &str,
        total_cycles: u64,
        total_ticks: u64,
        draw_cycles: u64,
        render_cycles: u64,
        attach_cycles: u64,
        commit_cycles: u64,
        dirty_bounds: Rect,
    ) {
        if !PERF_LOG_ENABLED {
            return;
        }
        let force_initial = self.metrics.frame_logs_emitted < INITIAL_FRAME_LOGS;
        if !force_initial && total_ticks < SLOW_FRAME_THRESHOLD_TICKS {
            return;
        }
        self.metrics.frame_logs_emitted = self.metrics.frame_logs_emitted.saturating_add(1);
        let label = if total_ticks < SLOW_FRAME_THRESHOLD_TICKS {
            "frame"
        } else {
            "slow-frame"
        };
        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios {} kind={} total={}cy ticks={} draw={}cy render={}cy attach={}cy commit={}cy dirty=({:.0},{:.0} {:.0}x{:.0})\n",
            label,
            kind,
            total_cycles,
            total_ticks,
            draw_cycles,
            render_cycles,
            attach_cycles,
            commit_cycles,
            dirty_bounds.origin.x,
            dirty_bounds.origin.y,
            dirty_bounds.size.width,
            dirty_bounds.size.height,
        );
        perf_log(&line);
    }

    fn log_backend_started(&self, size: (u32, u32)) {
        if !PERF_LOG_ENABLED {
            return;
        }
        let mut line = String::new();
        let _ = write!(
            line,
            "viewkit/mochios perf-start fullscreen={} size={}x{} direct_input={}\n",
            self.config.fullscreen, size.0, size.1, self.direct_input,
        );
        perf_log(&line);
    }

    fn current_cursor_rect(&self, viewport: Viewport) -> Option<Rect> {
        self.cursor_image.as_ref()?;
        let bounds = viewport.logical_bounds();
        Some(
            Rect::new(
                self.pointer_x - CURSOR_HOTSPOT_X,
                self.pointer_y - CURSOR_HOTSPOT_Y,
                CURSOR_WIDTH as f32,
                CURSOR_HEIGHT as f32,
            )
            .intersection(bounds)
            .unwrap_or_else(|| Rect::new(self.pointer_x, self.pointer_y, 1.0, 1.0)),
        )
    }

    fn mark_cursor_dirty(&mut self, viewport: Viewport, previous: Option<Rect>) {
        let Some(current) = self.current_cursor_rect(viewport) else {
            return;
        };
        let dirty = previous
            .map_or(current, |previous| previous.union(current))
            .expanded(2.0);
        self.cursor_dirty = Some(self.cursor_dirty.map_or(dirty, |old| old.union(dirty)));
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
        let scale = window.viewport().scale_factor as f32;
        let logical_x = a as f32 / scale;
        let logical_y = b as f32 / scale;

        match kind {
            EVENT_POINTER_ENTER | EVENT_POINTER_MOTION => {
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: logical_x,
                        y: logical_y,
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
                // Motion notifications are best-effort, while button messages
                // carry the authoritative local pointer position.
                self.app.handle_event(
                    PlatformEvent::PointerMoved {
                        x: logical_x,
                        y: logical_y,
                    },
                    window,
                );
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
            EVENT_POINTER_SCROLL => {
                self.app.handle_event(
                    PlatformEvent::Scroll {
                        delta_x: a as f32,
                        delta_y: b as f32,
                    },
                    window,
                );
            }
            EVENT_KEY => {
                let flags = (c & 0xffff) as u16;
                if flags & INPUT_FLAG_PRESS != 0 {
                    let modifiers = key_modifiers_from_wire(c >> 16);
                    if let Some(key) = key_from_wire(a as u16, b as u32) {
                        self.app
                            .handle_event(PlatformEvent::KeyPressed { key, modifiers }, window);
                    }
                    if let Some(event) = self.key_event(a as u16, b as u32, modifiers) {
                        self.app.handle_event(event, window);
                    }
                }
            }
            EVENT_CLOSE_REQUESTED => {
                self.app.handle_event(PlatformEvent::CloseRequested, window);
                self.close_requested = true;
            }
            EVENT_FOCUS_GAINED => {
                self.app.handle_event(PlatformEvent::Focused(true), window);
            }
            EVENT_FOCUS_LOST => {
                self.app.handle_event(PlatformEvent::Focused(false), window);
            }
            EVENT_FRAME_DONE => {}
            EVENT_CONFIGURE => {
                let width = u32::try_from(a).map_err(|_| MochiOsBackendError::InvalidWindowSize)?;
                let height =
                    u32::try_from(b).map_err(|_| MochiOsBackendError::InvalidWindowSize)?;
                checked_surface_size(Size::new(width as f32, height as f32))?;
                self.pending_resize = Some((width, height));
            }
            EVENT_CONTEXT_MENU_RESULT => {
                let status = unsafe { read_u32_raw(event.as_ptr(), 4) };
                let request_id = unsafe { read_u64_raw(event.as_ptr(), 8) };
                let command_id = unsafe { read_u32_raw(event.as_ptr(), 16) };
                self.app.handle_event(
                    PlatformEvent::ContextMenuResult {
                        request_id,
                        command_id: (status == 0).then_some(command_id),
                    },
                    window,
                );
            }
            EVENT_APPEARANCE_CHANGED => {
                if self.app.reload_appearance() {
                    let previous_scale = window.viewport().scale_factor as f32;
                    let viewport = scaled_viewport(
                        window.width(),
                        window.height(),
                        self.app.interface_scale_factor(),
                    );
                    let physical_x = self.pointer_x * previous_scale;
                    let physical_y = self.pointer_y * previous_scale;
                    self.pointer_x = physical_x / viewport.scale_factor as f32;
                    self.pointer_y = physical_y / viewport.scale_factor as f32;
                    window.set_viewport(viewport);
                    self.app
                        .handle_event(PlatformEvent::ScaleFactorChanged { viewport }, window);
                    window.request_redraw();
                }
            }
            _ => {}
        }

        Ok(())
    }

    fn key_event(
        &self,
        keycode: u16,
        codepoint: u32,
        modifiers: KeyModifiers,
    ) -> Option<PlatformEvent> {
        if modifiers.shortcut() {
            return (keycode == KEY_A).then_some(PlatformEvent::SelectAll);
        }
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
            KEY_LEFT => {
                if modifiers.shift() {
                    PlatformEvent::SelectLeft
                } else {
                    PlatformEvent::ArrowLeft
                }
            }
            KEY_RIGHT => {
                if modifiers.shift() {
                    PlatformEvent::SelectRight
                } else {
                    PlatformEvent::ArrowRight
                }
            }
            KEY_HOME => {
                if modifiers.shift() {
                    PlatformEvent::SelectHome
                } else {
                    PlatformEvent::Home
                }
            }
            KEY_END => {
                if modifiers.shift() {
                    PlatformEvent::SelectEnd
                } else {
                    PlatformEvent::End
                }
            }
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

fn scaled_viewport(width: u32, height: u32, scale_factor: f64) -> Viewport {
    let scale_factor = if scale_factor.is_finite() && scale_factor > 0.0 {
        scale_factor.clamp(0.75, 2.0)
    } else {
        1.0
    };
    Viewport::new(
        Size::new(
            width as f32 / scale_factor as f32,
            height as f32 / scale_factor as f32,
        ),
        width,
        height,
        scale_factor,
    )
}

pub fn notify_appearance_changed() -> Result<(), MochiOsBackendError> {
    let compositor = find_compositor()?;
    let request = OP_APPEARANCE_CHANGED.to_le_bytes();
    let mut reply = [0u8; 16];
    let len = ipc_call_raw(
        compositor,
        request.as_ptr(),
        request.len(),
        reply.as_mut_ptr(),
        reply.len(),
    )?;
    status_from_raw(reply.as_ptr(), len)
}

fn key_modifiers_from_wire(modifiers: u32) -> KeyModifiers {
    let mut bits = 0;
    if modifiers & INPUT_MOD_SHIFT != 0 {
        bits |= KeyModifiers::SHIFT;
    }
    if modifiers & INPUT_MOD_CONTROL != 0 {
        bits |= KeyModifiers::CONTROL;
    }
    if modifiers & INPUT_MOD_ALT != 0 {
        bits |= KeyModifiers::ALT;
    }
    KeyModifiers::from_bits(bits)
}

fn key_from_wire(keycode: u16, codepoint: u32) -> Option<Key> {
    if let Some(character) = char::from_u32(codepoint)
        && !character.is_control()
    {
        return Some(Key::Character(character));
    }

    Some(match keycode {
        KEY_ESCAPE => Key::Escape,
        KEY_BACKSPACE => Key::Backspace,
        KEY_TAB => Key::Tab,
        KEY_ENTER => Key::Enter,
        KEY_SPACE => Key::Space,
        KEY_DELETE => Key::Delete,
        KEY_HOME => Key::Home,
        KEY_END => Key::End,
        KEY_LEFT => Key::ArrowLeft,
        KEY_RIGHT => Key::ArrowRight,
        KEY_UP => Key::ArrowUp,
        KEY_DOWN => Key::ArrowDown,
        KEY_PAGE_UP => Key::PageUp,
        KEY_PAGE_DOWN => Key::PageDown,
        _ => return None,
    })
}
