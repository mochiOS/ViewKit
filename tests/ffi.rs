use viewkit::ffi::{vk_begin_padding, vk_begin_vstack, vk_end_node, vk_poll_action, vk_push_button, vk_push_text, vk_runtime_collect_actions, vk_runtime_create, vk_runtime_destroy, vk_tree_begin, vk_tree_commit, VkActionEvent, VkStatus, VkString, VK_ALIGNMENT_CENTER, VK_BUTTON_COLOR_ACCENT, VK_DISTRIBUTION_CENTER, VK_STACK_GAP_LARGE, VK_TEXT_ALIGNMENT_START, VK_TEXT_COLOR_BLACK};

#[test]
fn ffi_builds_counter_tree() {
    let runtime =
        vk_runtime_create(1);

    assert!(
        !runtime.is_null(),
    );

    assert_eq!(
        vk_tree_begin(
            runtime,
            100,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_begin_padding(
            runtime,
            101,
            24.0,
            24.0,
            24.0,
            24.0,
        ),
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

    let counter_text =
        String::from(
            "count: 0",
        );

    assert_eq!(
        vk_push_text(
            runtime,
            103,
            VkString::from_str(
                &counter_text,
            ),
            18.0,
            28.0,
            600,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::Ok as i32,
    );

    let button_title =
        String::from(
            "increment",
        );

    assert_eq!(
        vk_push_button(
            runtime,
            104,
            VkString::from_str(
                &button_title,
            ),
            VK_BUTTON_COLOR_ACCENT,
            0.5,
            200,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_end_node(runtime),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_end_node(runtime),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_tree_commit(runtime),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_runtime_collect_actions(
            runtime,
        ),
        VkStatus::Ok as i32,
    );

    let mut event =
        VkActionEvent::default();

    let mut has_action =
        0_u8;

    assert_eq!(
        vk_poll_action(
            runtime,
            &mut event,
            &mut has_action,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        has_action,
        0,
    );

    assert_eq!(
        vk_runtime_destroy(
            runtime,
        ),
        VkStatus::Ok as i32,
    );
}

#[test]
fn ffi_rejects_invalid_enum_value() {
    let runtime =
        vk_runtime_create(1);

    assert!(
        !runtime.is_null(),
    );

    assert_eq!(
        vk_tree_begin(
            runtime,
            100,
        ),
        VkStatus::Ok as i32,
    );

    assert_eq!(
        vk_begin_vstack(
            runtime,
            101,
            999,
            VK_ALIGNMENT_CENTER,
            VK_DISTRIBUTION_CENTER,
        ),
        VkStatus::
            InvalidEnumValue
            as i32,
    );

    assert_eq!(
        vk_runtime_destroy(
            runtime,
        ),
        VkStatus::Ok as i32,
    );
}

#[test]
fn ffi_rejects_node_without_tree() {
    let runtime =
        vk_runtime_create(1);

    assert!(
        !runtime.is_null(),
    );

    let text =
        String::from(
            "Hello",
        );

    assert_eq!(
        vk_push_text(
            runtime,
            1,
            VkString::from_str(
                &text,
            ),
            16.0,
            24.0,
            400,
            VK_TEXT_ALIGNMENT_START,
            VK_TEXT_COLOR_BLACK,
        ),
        VkStatus::
            NoActiveBuilder
            as i32,
    );

    assert_eq!(
        vk_runtime_destroy(
            runtime,
        ),
        VkStatus::Ok as i32,
    );
}