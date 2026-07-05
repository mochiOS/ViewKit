//! KomeからViewKit Runtimeを操作するためのC関数

use crate::components::{BorderStyle, ButtonColor, RectangleColor, ZStackAlignment};
use crate::draw_command::{DisplayList, DrawCommand};
use crate::event::{EventContext, EventDispatcher};
use crate::geometry::{Rect, Size};
use crate::layout::{LayoutLength, StackAlignment, StackDistribution, StackGap};
use crate::platform::linux::LinuxBackend;
use crate::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use crate::renderer::Viewport;
use crate::runtime::{
    ComponentInstanceId, NodeId, RectangleNode, RuntimeEvent, ViewNode, ViewNodeKind, ViewRuntime,
    ViewTreeBuilder,
};
use crate::theme::{Color, CornerRadius, Theme};
use crate::typography::{TextAlignment, TextMeasurer, Typography};
use crate::view::{PaintContext, RedrawSchedule, View};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::str;
use std::time::Instant;

mod generated_components;

pub use generated_components::*;

pub const VK_Z_ALIGNMENT_TOP_LEADING: u32 = 0;
pub const VK_Z_ALIGNMENT_TOP: u32 = 1;
pub const VK_Z_ALIGNMENT_TOP_TRAILING: u32 = 2;
pub const VK_Z_ALIGNMENT_LEADING: u32 = 3;
pub const VK_Z_ALIGNMENT_CENTER: u32 = 4;
pub const VK_Z_ALIGNMENT_TRAILING: u32 = 5;
pub const VK_Z_ALIGNMENT_BOTTOM_LEADING: u32 = 6;
pub const VK_Z_ALIGNMENT_BOTTOM: u32 = 7;
pub const VK_Z_ALIGNMENT_BOTTOM_TRAILING: u32 = 8;

pub const VK_ABI_VERSION_MAJOR: u32 = 1;
pub const VK_ABI_VERSION_MINOR: u32 = 0;
pub const VK_ABI_VERSION_PATCH: u32 = 0;

/*
 * 0x00MMmmpp
 *
 * MM: major
 * mm: minor
 * pp: patch
 *
 * 1.0.0は0x00010000になります。
 */
pub const VK_ABI_VERSION: u32 =
    (VK_ABI_VERSION_MAJOR << 16) | (VK_ABI_VERSION_MINOR << 8) | VK_ABI_VERSION_PATCH;

#[repr(i32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VkStatus {
    Ok = 0,

    NullPointer = 1,
    InvalidUtf8 = 2,

    BuilderAlreadyActive = 3,
    NoActiveBuilder = 4,

    NoOpenNode = 5,
    UnclosedNodes = 6,
    MultipleRoots = 7,
    MissingRoot = 8,

    InvalidEnumValue = 9,
    UnsupportedEvent = 10,

    PlatformError = 11,
    UnsupportedPlatform = 12,

    Panic = 255,
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkString {
    pub pointer: *const u8,
    pub length: usize,
}

impl VkString {
    pub fn from_str(value: &str) -> Self {
        Self {
            pointer: value.as_ptr(),

            length: value.len(),
        }
    }
}

pub const VK_LENGTH_AUTO: u32 = 0;

pub const VK_LENGTH_FIXED: u32 = 1;

pub const VK_RECTANGLE_COLOR_BACKGROUND: u32 = 0;
pub const VK_RECTANGLE_COLOR_SURFACE: u32 = 1;
pub const VK_RECTANGLE_COLOR_ELEVATED_SURFACE: u32 = 2;
pub const VK_RECTANGLE_COLOR_ACCENT: u32 = 3;
pub const VK_RECTANGLE_COLOR_DESTRUCTIVE: u32 = 4;
pub const VK_RECTANGLE_COLOR_CUSTOM: u32 = 5;

pub const VK_CORNER_RADIUS_NONE: u32 = 0;
pub const VK_CORNER_RADIUS_SMALL: u32 = 1;
pub const VK_CORNER_RADIUS_MEDIUM: u32 = 2;
pub const VK_CORNER_RADIUS_LARGE: u32 = 3;
pub const VK_CORNER_RADIUS_EXTRA_LARGE: u32 = 4;
pub const VK_CORNER_RADIUS_CARD: u32 = 5;
pub const VK_CORNER_RADIUS_FULL: u32 = 6;
pub const VK_CORNER_RADIUS_CUSTOM: u32 = 7;

pub const VK_BORDER_NONE: u32 = 0;
pub const VK_BORDER_STANDARD: u32 = 1;
pub const VK_BORDER_STRONG: u32 = 2;
pub const VK_BORDER_CUSTOM: u32 = 3;

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkColor {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub alpha: u8,
}

impl VkColor {
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Self {
            red,
            green,
            blue,
            alpha,
        }
    }

    pub const fn transparent() -> Self {
        Self::rgba(0, 0, 0, 0)
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkRectangleStyle {
    pub color_kind: u32,
    pub custom_color: VkColor,

    pub radius_kind: u32,
    pub radius: f32,

    pub border_kind: u32,
    pub border_color: VkColor,
    pub border_width: f32,
}

impl Default for VkRectangleStyle {
    fn default() -> Self {
        Self {
            color_kind: VK_RECTANGLE_COLOR_SURFACE,
            custom_color: VkColor::transparent(),

            radius_kind: VK_CORNER_RADIUS_NONE,
            radius: 0.0,

            border_kind: VK_BORDER_NONE,
            border_color: VkColor::transparent(),
            border_width: 0.0,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct VkLength {
    pub kind: u32,
    pub value: f32,
}

impl VkLength {
    pub const fn auto() -> Self {
        Self {
            kind: VK_LENGTH_AUTO,
            value: 0.0,
        }
    }

    pub const fn fixed(value: f32) -> Self {
        Self {
            kind: VK_LENGTH_FIXED,
            value,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Debug, Default)]
pub struct VkActionEvent {
    pub component_instance_id: u64,
    pub node_id: u64,
    pub action_id: u64,
    pub event_kind: u32,
}

pub const VK_EVENT_BUTTON_CLICKED: u32 = 1;

pub const VK_STACK_GAP_NONE: u32 = 0;

pub const VK_STACK_GAP_EXTRA_SMALL: u32 = 1;

pub const VK_STACK_GAP_SMALL: u32 = 2;

pub const VK_STACK_GAP_MEDIUM: u32 = 3;

pub const VK_STACK_GAP_LARGE: u32 = 4;

pub const VK_STACK_GAP_EXTRA_LARGE: u32 = 5;

pub const VK_STACK_GAP_DOUBLE_EXTRA_LARGE: u32 = 6;

pub const VK_ALIGNMENT_START: u32 = 0;

pub const VK_ALIGNMENT_CENTER: u32 = 1;

pub const VK_ALIGNMENT_END: u32 = 2;

pub const VK_ALIGNMENT_STRETCH: u32 = 3;

pub const VK_DISTRIBUTION_START: u32 = 0;

pub const VK_DISTRIBUTION_CENTER: u32 = 1;

pub const VK_DISTRIBUTION_END: u32 = 2;

pub const VK_DISTRIBUTION_SPACE_BETWEEN: u32 = 3;

pub const VK_DISTRIBUTION_SPACE_AROUND: u32 = 4;

pub const VK_DISTRIBUTION_SPACE_EVENLY: u32 = 5;

pub const VK_TEXT_ALIGNMENT_START: u32 = 0;

pub const VK_TEXT_ALIGNMENT_CENTER: u32 = 1;

pub const VK_TEXT_ALIGNMENT_END: u32 = 2;

pub const VK_TEXT_ALIGNMENT_JUSTIFIED: u32 = 3;

pub const VK_TEXT_COLOR_BLACK: u32 = 0;

pub const VK_TEXT_COLOR_WHITE: u32 = 1;

pub const VK_BUTTON_COLOR_ACCENT: u32 = 0;

pub const VK_BUTTON_COLOR_DESTRUCTIVE: u32 = 1;

pub struct VkRuntime {
    component_instance: ComponentInstanceId,

    runtime: ViewRuntime,

    builder: Option<ViewTreeBuilder>,
}

impl VkRuntime {
    fn new(component_instance_id: u64) -> Self {
        let component_instance = ComponentInstanceId(component_instance_id);

        Self {
            component_instance,

            runtime: ViewRuntime::new(component_instance),

            builder: None,
        }
    }

    pub fn runtime(&self) -> &ViewRuntime {
        &self.runtime
    }

    pub fn runtime_mut(&mut self) -> &mut ViewRuntime {
        &mut self.runtime
    }
}

struct VkWindowApplication<'a> {
    runtime: &'a mut VkRuntime,

    root: Option<Box<dyn View>>,

    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,
}

impl<'a> VkWindowApplication<'a> {
    fn new(runtime: &'a mut VkRuntime) -> Self {
        Self {
            runtime,

            root: None,

            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),
        }
    }

    fn rebuild_root(&mut self) {
        self.root = self.runtime.runtime_mut().build_view();
    }

    fn ensure_root(&mut self) {
        if self.root.is_none() {
            self.rebuild_root();
        }
    }
}

impl PlatformApplication for VkWindowApplication<'_> {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        match &event {
            PlatformEvent::Resumed { .. }
            | PlatformEvent::Resized { .. }
            | PlatformEvent::ScaleFactorChanged { .. } => {
                self.rebuild_root();
                window.request_redraw();
                return;
            }

            PlatformEvent::RedrawRequested | PlatformEvent::CloseRequested => {
                return;
            }

            _ => {}
        }

        self.ensure_root();

        let Some(root) = self.root.as_ref() else {
            return;
        };

        let bounds = window.viewport().logical_bounds();

        let redraw_request = {
            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(root.as_ref(), bounds, &event, &mut context);

            context.redraw_request()
        };

        self.runtime.runtime_mut().collect_actions();

        if redraw_request.is_requested() {
            window.request_redraw();
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) -> Rect {
        self.ensure_root();

        let bounds = viewport.logical_bounds();

        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        if let Some(root) = self.root.as_ref() {
            let mut context = PaintContext::new(
                display_list,
                &self.theme,
                &self.typography,
                &mut self.text_measurer,
            )
            .with_redraw_schedule(&mut self.redraw_schedule);

            root.paint(bounds, &mut context);
        }

        bounds
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        self.redraw_schedule.deadline()
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_create(component_instance_id: u64) -> *mut VkRuntime {
    catch_unwind(AssertUnwindSafe(|| {
        Box::into_raw(Box::new(VkRuntime::new(component_instance_id)))
    }))
    .unwrap_or_else(|_| ptr::null_mut())
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_abi_version() -> u32 {
    VK_ABI_VERSION
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_status_name(status: i32) -> VkString {
    VkString::from_str(status_name(status))
}

fn status_name(status: i32) -> &'static str {
    match status {
        value if value == VkStatus::Ok as i32 => "ok",

        value if value == VkStatus::NullPointer as i32 => "null_pointer",

        value if value == VkStatus::InvalidUtf8 as i32 => "invalid_utf8",

        value if value == VkStatus::BuilderAlreadyActive as i32 => "builder_already_active",

        value if value == VkStatus::NoActiveBuilder as i32 => "no_active_builder",

        value if value == VkStatus::NoOpenNode as i32 => "no_open_node",

        value if value == VkStatus::UnclosedNodes as i32 => "unclosed_nodes",

        value if value == VkStatus::MultipleRoots as i32 => "multiple_roots",

        value if value == VkStatus::MissingRoot as i32 => "missing_root",

        value if value == VkStatus::InvalidEnumValue as i32 => "invalid_enum_value",

        value if value == VkStatus::UnsupportedEvent as i32 => "unsupported_event",

        value if value == VkStatus::Panic as i32 => "panic",

        value if value == VkStatus::PlatformError as i32 => "platform_error",

        value if value == VkStatus::UnsupportedPlatform as i32 => "unsupported_platform",

        _ => "unknown_status",
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_destroy(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        if runtime.is_null() {
            return Ok(());
        }

        unsafe {
            drop(Box::from_raw(runtime));
        }

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_begin(runtime: *mut VkRuntime, root_node_id: u64) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        if runtime.builder.is_some() {
            return Err(VkStatus::BuilderAlreadyActive);
        }

        let mut builder = ViewTreeBuilder::new(runtime.component_instance);

        builder.begin(ViewNode::new(NodeId(root_node_id), ViewNodeKind::Root));

        runtime.builder = Some(builder);

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_abort(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        runtime.builder = None;

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_end_node(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.end().map_err(map_builder_error)
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_tree_commit(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let mut builder = runtime.builder.take().ok_or(VkStatus::NoActiveBuilder)?;

        /*
         * vk_tree_begin()が作成したRootを閉じる。
         *
         * 子Nodeが閉じられていない場合は、
         * finish()がUnclosedNodesを返す。
         */
        builder.end().map_err(map_builder_error)?;

        let tree = builder.finish().map_err(map_builder_error)?;

        runtime.runtime.commit(tree);

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_collect_actions(runtime: *mut VkRuntime) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        runtime.runtime.collect_actions();

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_poll_action(
    runtime: *mut VkRuntime,
    output: *mut VkActionEvent,
    has_action: *mut u8,
) -> i32 {
    ffi_status(|| {
        if output.is_null() || has_action.is_null() {
            return Err(VkStatus::NullPointer);
        }

        unsafe {
            *has_action = 0;
        }

        let runtime = runtime_mut(runtime)?;

        let Some(action) = runtime.runtime.poll_action() else {
            return Ok(());
        };

        let event_kind = match action.event {
            RuntimeEvent::ButtonClicked => VK_EVENT_BUTTON_CLICKED,

            _ => {
                return Err(VkStatus::UnsupportedEvent);
            }
        };

        unsafe {
            *output = VkActionEvent {
                component_instance_id: action.component_instance.0,

                node_id: action.node_id.0,

                action_id: action.action_id.0,

                event_kind,
            };

            *has_action = 1;
        }

        Ok(())
    })
}

fn runtime_mut<'a>(runtime: *mut VkRuntime) -> Result<&'a mut VkRuntime, VkStatus> {
    if runtime.is_null() {
        return Err(VkStatus::NullPointer);
    }

    Ok(unsafe { &mut *runtime })
}

fn active_builder(runtime: &mut VkRuntime) -> Result<&mut ViewTreeBuilder, VkStatus> {
    runtime.builder.as_mut().ok_or(VkStatus::NoActiveBuilder)
}

fn copy_string(value: VkString) -> Result<String, VkStatus> {
    if value.length == 0 {
        return Ok(String::new());
    }

    if value.pointer.is_null() {
        return Err(VkStatus::NullPointer);
    }

    let bytes = unsafe { slice::from_raw_parts(value.pointer, value.length) };

    let value = str::from_utf8(bytes).map_err(|_| VkStatus::InvalidUtf8)?;

    Ok(value.to_owned())
}

fn decode_stack_gap(value: u32) -> Result<StackGap, VkStatus> {
    match value {
        VK_STACK_GAP_NONE => Ok(StackGap::None),

        VK_STACK_GAP_EXTRA_SMALL => Ok(StackGap::ExtraSmall),

        VK_STACK_GAP_SMALL => Ok(StackGap::Small),

        VK_STACK_GAP_MEDIUM => Ok(StackGap::Medium),

        VK_STACK_GAP_LARGE => Ok(StackGap::Large),

        VK_STACK_GAP_EXTRA_LARGE => Ok(StackGap::ExtraLarge),

        VK_STACK_GAP_DOUBLE_EXTRA_LARGE => Ok(StackGap::DoubleExtraLarge),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_stack_alignment(value: u32) -> Result<StackAlignment, VkStatus> {
    match value {
        VK_ALIGNMENT_START => Ok(StackAlignment::Start),

        VK_ALIGNMENT_CENTER => Ok(StackAlignment::Center),

        VK_ALIGNMENT_END => Ok(StackAlignment::End),

        VK_ALIGNMENT_STRETCH => Ok(StackAlignment::Stretch),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_stack_distribution(value: u32) -> Result<StackDistribution, VkStatus> {
    match value {
        VK_DISTRIBUTION_START => Ok(StackDistribution::Start),

        VK_DISTRIBUTION_CENTER => Ok(StackDistribution::Center),

        VK_DISTRIBUTION_END => Ok(StackDistribution::End),

        VK_DISTRIBUTION_SPACE_BETWEEN => Ok(StackDistribution::SpaceBetween),

        VK_DISTRIBUTION_SPACE_AROUND => Ok(StackDistribution::SpaceAround),

        VK_DISTRIBUTION_SPACE_EVENLY => Ok(StackDistribution::SpaceEvenly),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_text_alignment(value: u32) -> Result<TextAlignment, VkStatus> {
    match value {
        VK_TEXT_ALIGNMENT_START => Ok(TextAlignment::Start),

        VK_TEXT_ALIGNMENT_CENTER => Ok(TextAlignment::Center),

        VK_TEXT_ALIGNMENT_END => Ok(TextAlignment::End),

        VK_TEXT_ALIGNMENT_JUSTIFIED => Ok(TextAlignment::Justified),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_text_color(value: u32) -> Result<Color, VkStatus> {
    match value {
        VK_TEXT_COLOR_BLACK => Ok(Color::BLACK),

        VK_TEXT_COLOR_WHITE => Ok(Color::WHITE),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_button_color(value: u32) -> Result<ButtonColor, VkStatus> {
    match value {
        VK_BUTTON_COLOR_ACCENT => Ok(ButtonColor::Accent),

        VK_BUTTON_COLOR_DESTRUCTIVE => Ok(ButtonColor::Destructive),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn map_builder_error(error: crate::runtime::TreeBuilderError) -> VkStatus {
    match error {
        crate::runtime::TreeBuilderError::NoOpenNode => VkStatus::NoOpenNode,

        crate::runtime::TreeBuilderError::UnclosedNodes => VkStatus::UnclosedNodes,

        crate::runtime::TreeBuilderError::MultipleRoots => VkStatus::MultipleRoots,

        crate::runtime::TreeBuilderError::MissingRoot => VkStatus::MissingRoot,
    }
}

fn sanitize_length(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or_default(value: f32, default: f32) -> f32 {
    if value.is_finite() && value > 0.0 {
        value
    } else {
        default
    }
}

fn ffi_status<F>(operation: F) -> i32
where
    F: FnOnce() -> Result<(), VkStatus>,
{
    match catch_unwind(AssertUnwindSafe(operation)) {
        Ok(Ok(())) => VkStatus::Ok as i32,

        Ok(Err(status)) => status as i32,

        Err(_) => VkStatus::Panic as i32,
    }
}

fn decode_zstack_alignment(value: u32) -> Result<ZStackAlignment, VkStatus> {
    match value {
        VK_Z_ALIGNMENT_TOP_LEADING => Ok(ZStackAlignment::TopLeading),

        VK_Z_ALIGNMENT_TOP => Ok(ZStackAlignment::Top),

        VK_Z_ALIGNMENT_TOP_TRAILING => Ok(ZStackAlignment::TopTrailing),

        VK_Z_ALIGNMENT_LEADING => Ok(ZStackAlignment::Leading),

        VK_Z_ALIGNMENT_CENTER => Ok(ZStackAlignment::Center),

        VK_Z_ALIGNMENT_TRAILING => Ok(ZStackAlignment::Trailing),

        VK_Z_ALIGNMENT_BOTTOM_LEADING => Ok(ZStackAlignment::BottomLeading),

        VK_Z_ALIGNMENT_BOTTOM => Ok(ZStackAlignment::Bottom),

        VK_Z_ALIGNMENT_BOTTOM_TRAILING => Ok(ZStackAlignment::BottomTrailing),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_layout_length(value: VkLength) -> Result<LayoutLength, VkStatus> {
    match value.kind {
        VK_LENGTH_AUTO => Ok(LayoutLength::Auto),

        VK_LENGTH_FIXED => Ok(LayoutLength::Fixed(sanitize_length(value.value))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_rectangle_style(style: VkRectangleStyle) -> Result<RectangleNode, VkStatus> {
    Ok(RectangleNode {
        color: decode_rectangle_color(style.color_kind, style.custom_color)?,

        radius: decode_corner_radius(style.radius_kind, style.radius)?,

        border: decode_border_style(style.border_kind, style.border_color, style.border_width)?,
    })
}

fn decode_rectangle_color(kind: u32, custom_color: VkColor) -> Result<RectangleColor, VkStatus> {
    match kind {
        VK_RECTANGLE_COLOR_BACKGROUND => Ok(RectangleColor::Background),

        VK_RECTANGLE_COLOR_SURFACE => Ok(RectangleColor::Surface),

        VK_RECTANGLE_COLOR_ELEVATED_SURFACE => Ok(RectangleColor::ElevatedSurface),

        VK_RECTANGLE_COLOR_ACCENT => Ok(RectangleColor::Accent),

        VK_RECTANGLE_COLOR_DESTRUCTIVE => Ok(RectangleColor::Destructive),

        VK_RECTANGLE_COLOR_CUSTOM => Ok(RectangleColor::Custom(decode_color(custom_color))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_corner_radius(kind: u32, value: f32) -> Result<CornerRadius, VkStatus> {
    match kind {
        VK_CORNER_RADIUS_NONE => Ok(CornerRadius::None),
        VK_CORNER_RADIUS_SMALL => Ok(CornerRadius::Small),
        VK_CORNER_RADIUS_MEDIUM => Ok(CornerRadius::Medium),
        VK_CORNER_RADIUS_LARGE => Ok(CornerRadius::Large),
        VK_CORNER_RADIUS_EXTRA_LARGE => Ok(CornerRadius::ExtraLarge),
        VK_CORNER_RADIUS_CARD => Ok(CornerRadius::Card),
        VK_CORNER_RADIUS_FULL => Ok(CornerRadius::Full),

        VK_CORNER_RADIUS_CUSTOM => Ok(CornerRadius::Custom(sanitize_length(value))),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_border_style(kind: u32, color: VkColor, width: f32) -> Result<BorderStyle, VkStatus> {
    let width = sanitize_length(width);

    match kind {
        VK_BORDER_NONE => Ok(BorderStyle::None),

        VK_BORDER_STANDARD => Ok(BorderStyle::standard(width)),

        VK_BORDER_STRONG => Ok(BorderStyle::strong(width)),

        VK_BORDER_CUSTOM => Ok(BorderStyle::custom(decode_color(color), width)),

        _ => Err(VkStatus::InvalidEnumValue),
    }
}

fn decode_color(color: VkColor) -> Color {
    Color::rgba(color.red, color.green, color.blue, color.alpha)
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_run_window(
    runtime: *mut VkRuntime,
    title: VkString,
    width: f32,
    height: f32,
    resizable: u8,
) -> i32 {
    ffi_status(|| {
        let title = copy_string(title)?;

        let width = finite_or_default(width, 800.0).max(1.0);
        let height = finite_or_default(height, 600.0).max(1.0);

        let runtime = runtime_mut(runtime)?;

        if runtime.builder.is_some() {
            return Err(VkStatus::BuilderAlreadyActive);
        }

        if runtime.runtime_mut().tree().is_none() {
            return Err(VkStatus::MissingRoot);
        }

        #[cfg(target_os = "linux")]
        {
            let application = VkWindowApplication::new(runtime);

            let backend = LinuxBackend::new(
                application,
                WindowConfig {
                    title,
                    size: Size::new(width, height),
                    resizable: resizable != 0,
                },
            );

            backend.run().map_err(|_| VkStatus::PlatformError)?;

            Ok(())
        }

        #[cfg(not(target_os = "linux"))]
        {
            let _ = title;
            let _ = width;
            let _ = height;
            let _ = resizable;

            Err(VkStatus::UnsupportedPlatform)
        }
    })
}
