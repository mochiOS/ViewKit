//! KomeからViewKit Runtimeを操作するためのC関数

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::ptr;
use std::slice;
use std::str;

use crate::components::ButtonColor;
use crate::layout::{StackAlignment, StackDistribution, StackGap};
use crate::runtime::{
    ActionId, ButtonNode, ComponentInstanceId, NodeId, PaddingNode, RuntimeEvent, TextNode,
    VStackNode, ViewNode, ViewNodeKind, ViewRuntime, ViewTreeBuilder,
};
use crate::theme::Color;
use crate::typography::TextAlignment;

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

#[unsafe(no_mangle)]
pub extern "C" fn vk_runtime_create(component_instance_id: u64) -> *mut VkRuntime {
    catch_unwind(AssertUnwindSafe(|| {
        Box::into_raw(Box::new(VkRuntime::new(component_instance_id)))
    }))
    .unwrap_or_else(|_| ptr::null_mut())
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
pub extern "C" fn vk_begin_vstack(
    runtime: *mut VkRuntime,
    node_id: u64,
    gap: u32,
    alignment: u32,
    distribution: u32,
) -> i32 {
    ffi_status(|| {
        let gap = decode_stack_gap(gap)?;

        let alignment = decode_stack_alignment(alignment)?;

        let distribution = decode_stack_distribution(distribution)?;

        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.begin(ViewNode::new(
            NodeId(node_id),
            ViewNodeKind::VStack(VStackNode {
                gap,
                alignment,
                distribution,
            }),
        ));

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_push_text(
    runtime: *mut VkRuntime,
    node_id: u64,
    content: VkString,
    font_size: f32,
    line_height: f32,
    weight: u16,
    alignment: u32,
    color: u32,
) -> i32 {
    ffi_status(|| {
        let content = copy_string(content)?;

        let alignment = decode_text_alignment(alignment)?;

        let color = decode_text_color(color)?;

        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.leaf(ViewNode::new(
            NodeId(node_id),
            ViewNodeKind::Text(TextNode {
                content,
                font_family: String::from("Noto Sans JP"),
                font_size: finite_or_default(font_size, 16.0),
                line_height: finite_or_default(line_height, 24.0),
                weight,
                alignment,
                color,
            }),
        ));

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_push_button(
    runtime: *mut VkRuntime,
    node_id: u64,
    title: VkString,
    color: u32,
    radius: f32,
    action_id: u64,
) -> i32 {
    ffi_status(|| {
        let title = copy_string(title)?;

        let color = decode_button_color(color)?;

        let action = if action_id == 0 {
            None
        } else {
            Some(ActionId(action_id))
        };

        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.leaf(ViewNode::new(
            NodeId(node_id),
            ViewNodeKind::Button(ButtonNode {
                title,
                color,

                radius: finite_or_default(radius, 0.0).clamp(0.0, 1.0),

                action,
            }),
        ));

        Ok(())
    })
}

#[unsafe(no_mangle)]
pub extern "C" fn vk_begin_padding(
    runtime: *mut VkRuntime,
    node_id: u64,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;

        let builder = active_builder(runtime)?;

        builder.begin(ViewNode::new(
            NodeId(node_id),
            ViewNodeKind::Padding(PaddingNode {
                top: sanitize_length(top),

                right: sanitize_length(right),

                bottom: sanitize_length(bottom),

                left: sanitize_length(left),
            }),
        ));

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
