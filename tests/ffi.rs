use viewkit::ffi::{
    VK_ALIGNMENT_CENTER, VK_BORDER_STANDARD, VK_BUTTON_COLOR_ACCENT, VK_CORNER_RADIUS_CARD,
    VK_DISTRIBUTION_CENTER, VK_DISTRIBUTION_START, VK_RECTANGLE_COLOR_SURFACE, VK_STACK_GAP_LARGE,
    VK_STACK_GAP_SMALL, VK_TEXT_ALIGNMENT_START, VK_TEXT_COLOR_BLACK, VkActionEvent, VkLength,
    VkRectangleStyle, VkStatus, VkString, vk_begin_background, vk_begin_frame, vk_begin_hstack,
    vk_begin_padding, vk_begin_vstack, vk_end_node, vk_poll_action, vk_push_button,
    vk_push_divider, vk_push_rectangle, vk_push_spacer, vk_push_text, vk_runtime_collect_actions,
    vk_runtime_create, vk_runtime_destroy, vk_tree_begin, vk_tree_commit,
};

#[test]
fn ffi_builds_counter_tree() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    assert_eq!(vk_tree_begin(runtime, 100,), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_padding(runtime, 101, 24.0, 24.0, 24.0, 24.0,),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_begin_vstack(
            runtime,
            102,
            VK_STACK_GAP_LARGE,
            VK_ALIGNMENT_CENTER,
            VK_DISTRIBUTION_CENTER,
        ),
        VkStatus::Ok as i32,
    );

    let counter_text = String::from("count: 0");

    assert_eq!(
        vk_push_text(
            runtime,
            103,
            VkString::from_str(&counter_text,),
            18.0,
            28.0,
            600,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    let button_title = String::from("increment");

    assert_eq!(
        vk_push_button(
            runtime,
            104,
            VkString::from_str(&button_title,),
            VK_BUTTON_COLOR_ACCENT,
            0.5,
            200,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_tree_commit(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_runtime_collect_actions(runtime,), VkStatus::Ok as i32,);

    let mut event = VkActionEvent::default();

    let mut has_action = 0_u8;

    assert_eq!(
        vk_poll_action(runtime, &mut event, &mut has_action,),
        VkStatus::Ok as i32,
    );

    assert_eq!(has_action, 0,);

    assert_eq!(vk_runtime_destroy(runtime,), VkStatus::Ok as i32,);
}

#[test]
fn ffi_rejects_invalid_enum_value() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    assert_eq!(vk_tree_begin(runtime, 100,), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_vstack(
            runtime,
            101,
            999,
            VK_ALIGNMENT_CENTER,
            VK_DISTRIBUTION_CENTER,
        ),
        VkStatus::InvalidEnumValue as i32,
    );

    assert_eq!(vk_runtime_destroy(runtime,), VkStatus::Ok as i32,);
}

#[test]
fn ffi_rejects_node_without_tree() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    let text = String::from("Hello");

    assert_eq!(
        vk_push_text(
            runtime,
            1,
            VkString::from_str(&text,),
            16.0,
            24.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::NoActiveBuilder as i32,
    );

    assert_eq!(vk_runtime_destroy(runtime,), VkStatus::Ok as i32,);
}

#[test]
fn ffi_builds_stack_with_spacer_and_divider() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    assert_eq!(vk_tree_begin(runtime, 100,), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_hstack(
            runtime,
            101,
            VK_STACK_GAP_SMALL,
            VK_ALIGNMENT_CENTER,
            VK_DISTRIBUTION_START,
        ),
        VkStatus::Ok as i32,
    );

    let left = String::from("Left");

    assert_eq!(
        vk_push_text(
            runtime,
            102,
            VkString::from_str(&left),
            14.0,
            22.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_push_spacer(runtime, 103,), VkStatus::Ok as i32,);

    assert_eq!(vk_push_divider(runtime, 104,), VkStatus::Ok as i32,);

    let right = String::from("Right");

    assert_eq!(
        vk_push_text(
            runtime,
            105,
            VkString::from_str(&right),
            14.0,
            22.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_tree_commit(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_runtime_destroy(runtime), VkStatus::Ok as i32,);
}

#[test]
fn ffi_builds_fixed_width_frame() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    assert_eq!(vk_tree_begin(runtime, 100,), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_frame(runtime, 101, VkLength::fixed(320.0), VkLength::auto(),),
        VkStatus::Ok as i32,
    );

    let content = String::from("Framed text");

    assert_eq!(
        vk_push_text(
            runtime,
            102,
            VkString::from_str(&content,),
            14.0,
            22.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_tree_commit(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_runtime_destroy(runtime), VkStatus::Ok as i32,);
}

#[test]
fn ffi_rejects_invalid_frame_length() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null(),);

    assert_eq!(vk_tree_begin(runtime, 100,), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_frame(
            runtime,
            101,
            VkLength {
                kind: 999,
                value: 100.0,
            },
            VkLength::auto(),
        ),
        VkStatus::InvalidEnumValue as i32,
    );

    assert_eq!(vk_runtime_destroy(runtime), VkStatus::Ok as i32,);
}

#[test]
fn ffi_builds_rectangle_and_background() {
    let runtime = vk_runtime_create(1);

    assert!(!runtime.is_null());

    assert_eq!(vk_tree_begin(runtime, 100), VkStatus::Ok as i32,);

    let style = VkRectangleStyle {
        color_kind: VK_RECTANGLE_COLOR_SURFACE,
        radius_kind: VK_CORNER_RADIUS_CARD,
        border_kind: VK_BORDER_STANDARD,
        border_width: 1.0,

        ..VkRectangleStyle::default()
    };

    assert_eq!(
        vk_begin_frame(runtime, 101, VkLength::fixed(320.0), VkLength::fixed(120.0),),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_begin_background(runtime, 102, style),
        VkStatus::Ok as i32,
    );

    let content = String::from("Background content");

    assert_eq!(
        vk_push_text(
            runtime,
            103,
            VkString::from_str(&content),
            14.0,
            22.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(
        vk_begin_frame(runtime, 104, VkLength::fixed(200.0), VkLength::fixed(48.0),),
        VkStatus::Ok as i32,
    );

    assert_eq!(vk_push_rectangle(runtime, 105, style), VkStatus::Ok as i32,);

    assert_eq!(vk_end_node(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_tree_commit(runtime), VkStatus::Ok as i32,);

    assert_eq!(vk_runtime_destroy(runtime), VkStatus::Ok as i32,);
}
