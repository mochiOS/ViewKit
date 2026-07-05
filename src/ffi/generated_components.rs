use super::*;
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
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
        let node = crate::runtime::ViewNodeKind::VStack(crate::runtime::VStackNode {
            gap,
            alignment,
            distribution,
        });
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_begin_hstack(
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
        let node = crate::runtime::ViewNodeKind::HStack(crate::runtime::HStackNode {
            gap,
            alignment,
            distribution,
        });
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_begin_zstack(runtime: *mut VkRuntime, node_id: u64, alignment: u32) -> i32 {
    ffi_status(|| {
        let alignment = decode_zstack_alignment(alignment)?;
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::ZStack(crate::runtime::ZStackNode { alignment });
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_begin_padding(
    runtime: *mut VkRuntime,
    node_id: u64,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> i32 {
    ffi_status(|| {
        let top = sanitize_length(top);
        let right = sanitize_length(right);
        let bottom = sanitize_length(bottom);
        let left = sanitize_length(left);
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Padding(crate::runtime::PaddingNode {
            top,
            right,
            bottom,
            left,
        });
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_begin_frame(
    runtime: *mut VkRuntime,
    node_id: u64,
    width: VkLength,
    height: VkLength,
) -> i32 {
    ffi_status(|| {
        let width = decode_layout_length(width)?;
        let height = decode_layout_length(height)?;
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Frame(crate::runtime::FrameNode { width, height });
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_begin_background(
    runtime: *mut VkRuntime,
    node_id: u64,
    style: VkRectangleStyle,
) -> i32 {
    ffi_status(|| {
        let properties = decode_rectangle_style(style)?;
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Background(properties);
        builder.begin(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
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
        let font_size = finite_or_default(font_size, 16.0);
        let line_height = finite_or_default(line_height, 24.0);
        let alignment = decode_text_alignment(alignment)?;
        let color = decode_text_color(color)?;
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Text(crate::runtime::TextNode {
            content,
            font_family: String::from("Noto Sans JP"),
            font_size,
            line_height,
            weight,
            alignment,
            color,
        });
        builder.leaf(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
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
        let radius = sanitize_length(radius);
        let action = if action_id == 0 {
            None
        } else {
            Some(crate::runtime::ActionId(action_id))
        };
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Button(crate::runtime::ButtonNode {
            title,
            color,
            radius,
            action,
        });
        builder.leaf(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_push_spacer(runtime: *mut VkRuntime, node_id: u64) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Spacer;
        builder.leaf(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_push_divider(runtime: *mut VkRuntime, node_id: u64) -> i32 {
    ffi_status(|| {
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Divider;
        builder.leaf(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
#[unsafe(no_mangle)]
#[allow(clippy::too_many_arguments)]
pub extern "C" fn vk_push_rectangle(
    runtime: *mut VkRuntime,
    node_id: u64,
    style: VkRectangleStyle,
) -> i32 {
    ffi_status(|| {
        let properties = decode_rectangle_style(style)?;
        let runtime = runtime_mut(runtime)?;
        let builder = active_builder(runtime)?;
        let node = crate::runtime::ViewNodeKind::Rectangle(properties);
        builder.leaf(crate::runtime::ViewNode::new(
            crate::runtime::NodeId(node_id),
            node,
        ));
        Ok(())
    })
}
