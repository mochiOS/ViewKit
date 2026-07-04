#ifndef VIEWKIT_H
#define VIEWKIT_H

#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

#define VK_ABI_VERSION_MAJOR UINT32_C(1)
#define VK_ABI_VERSION_MINOR UINT32_C(0)
#define VK_ABI_VERSION_PATCH UINT32_C(0)
#define VK_ABI_VERSION                                                   \
    ((VK_ABI_VERSION_MAJOR << 16) | (VK_ABI_VERSION_MINOR << 8) |        \
     VK_ABI_VERSION_PATCH)
typedef enum VkStatus {
    VK_STATUS_OK = 0,
    VK_STATUS_NULL_POINTER = 1,
    VK_STATUS_INVALID_UTF8 = 2,
    VK_STATUS_BUILDER_ALREADY_ACTIVE = 3,
    VK_STATUS_NO_ACTIVE_BUILDER = 4,
    VK_STATUS_NO_OPEN_NODE = 5,
    VK_STATUS_UNCLOSED_NODES = 6,
    VK_STATUS_MULTIPLE_ROOTS = 7,
    VK_STATUS_MISSING_ROOT = 8,
    VK_STATUS_INVALID_ENUM_VALUE = 9,
    VK_STATUS_UNSUPPORTED_EVENT = 10,
    VK_STATUS_PANIC = 255
} VkStatus;
typedef struct VkRuntime VkRuntime;
typedef struct VkString {
    const uint8_t *pointer;
    size_t length;
} VkString;
typedef struct VkActionEvent {
    uint64_t component_instance_id;
    uint64_t node_id;
    uint64_t action_id;
    uint32_t event_kind;
} VkActionEvent;
typedef struct VkLength {
    uint32_t kind;
    float value;
} VkLength;
typedef struct VkColor {
    uint8_t red;
    uint8_t green;
    uint8_t blue;
    uint8_t alpha;
} VkColor;
typedef struct VkRectangleStyle {
    uint32_t color_kind;
    VkColor custom_color;

    uint32_t radius_kind;
    float radius;

    uint32_t border_kind;
    VkColor border_color;
    float border_width;
} VkRectangleStyle;

#define VK_EVENT_BUTTON_CLICKED UINT32_C(1)
#define VK_STACK_GAP_NONE UINT32_C(0)
#define VK_STACK_GAP_EXTRA_SMALL UINT32_C(1)
#define VK_STACK_GAP_SMALL UINT32_C(2)
#define VK_STACK_GAP_MEDIUM UINT32_C(3)
#define VK_STACK_GAP_LARGE UINT32_C(4)
#define VK_STACK_GAP_EXTRA_LARGE UINT32_C(5)
#define VK_STACK_GAP_DOUBLE_EXTRA_LARGE UINT32_C(6)
#define VK_ALIGNMENT_START UINT32_C(0)
#define VK_ALIGNMENT_CENTER UINT32_C(1)
#define VK_ALIGNMENT_END UINT32_C(2)
#define VK_ALIGNMENT_STRETCH UINT32_C(3)
#define VK_DISTRIBUTION_START UINT32_C(0)
#define VK_DISTRIBUTION_CENTER UINT32_C(1)
#define VK_DISTRIBUTION_END UINT32_C(2)
#define VK_DISTRIBUTION_SPACE_BETWEEN UINT32_C(3)
#define VK_DISTRIBUTION_SPACE_AROUND UINT32_C(4)
#define VK_DISTRIBUTION_SPACE_EVENLY UINT32_C(5)
#define VK_Z_ALIGNMENT_TOP_LEADING UINT32_C(0)
#define VK_Z_ALIGNMENT_TOP UINT32_C(1)
#define VK_Z_ALIGNMENT_TOP_TRAILING UINT32_C(2)
#define VK_Z_ALIGNMENT_LEADING UINT32_C(3)
#define VK_Z_ALIGNMENT_CENTER UINT32_C(4)
#define VK_Z_ALIGNMENT_TRAILING UINT32_C(5)
#define VK_Z_ALIGNMENT_BOTTOM_LEADING UINT32_C(6)
#define VK_Z_ALIGNMENT_BOTTOM UINT32_C(7)
#define VK_Z_ALIGNMENT_BOTTOM_TRAILING UINT32_C(8)
#define VK_TEXT_ALIGNMENT_START UINT32_C(0)
#define VK_TEXT_ALIGNMENT_CENTER UINT32_C(1)
#define VK_TEXT_ALIGNMENT_END UINT32_C(2)
#define VK_TEXT_ALIGNMENT_JUSTIFIED UINT32_C(3)
#define VK_TEXT_COLOR_BLACK UINT32_C(0)
#define VK_TEXT_COLOR_WHITE UINT32_C(1)
#define VK_BUTTON_COLOR_ACCENT UINT32_C(0)
#define VK_BUTTON_COLOR_DESTRUCTIVE UINT32_C(1)
#define VK_LENGTH_AUTO UINT32_C(0)
#define VK_LENGTH_FIXED UINT32_C(1)
#define VK_RECTANGLE_COLOR_BACKGROUND UINT32_C(0)
#define VK_RECTANGLE_COLOR_SURFACE UINT32_C(1)
#define VK_RECTANGLE_COLOR_ELEVATED_SURFACE UINT32_C(2)
#define VK_RECTANGLE_COLOR_ACCENT UINT32_C(3)
#define VK_RECTANGLE_COLOR_DESTRUCTIVE UINT32_C(4)
#define VK_RECTANGLE_COLOR_CUSTOM UINT32_C(5)
#define VK_CORNER_RADIUS_NONE UINT32_C(0)
#define VK_CORNER_RADIUS_SMALL UINT32_C(1)
#define VK_CORNER_RADIUS_MEDIUM UINT32_C(2)
#define VK_CORNER_RADIUS_LARGE UINT32_C(3)
#define VK_CORNER_RADIUS_EXTRA_LARGE UINT32_C(4)
#define VK_CORNER_RADIUS_CARD UINT32_C(5)
#define VK_CORNER_RADIUS_FULL UINT32_C(6)
#define VK_CORNER_RADIUS_CUSTOM UINT32_C(7)
#define VK_BORDER_NONE UINT32_C(0)
#define VK_BORDER_STANDARD UINT32_C(1)
#define VK_BORDER_STRONG UINT32_C(2)
#define VK_BORDER_CUSTOM UINT32_C(3)

static inline VkString vk_string(const void *pointer, size_t length) {
    VkString value;
    value.pointer = (const uint8_t *)pointer;
    value.length = length;
    return value;
}

static inline VkLength vk_length_auto(void) {
    VkLength value;
    value.kind = VK_LENGTH_AUTO;
    value.value = 0.0f;
    return value;
}

static inline VkLength vk_length_fixed(float value) {
    VkLength length;
    length.kind = VK_LENGTH_FIXED;
    length.value = value;
    return length;
}

static inline VkColor vk_color_rgba(
    uint8_t red,
    uint8_t green,
    uint8_t blue,
    uint8_t alpha
) {
    VkColor color;
    color.red = red;
    color.green = green;
    color.blue = blue;
    color.alpha = alpha;
    return color;
}

static inline VkColor vk_color_transparent(void) {
    return vk_color_rgba(0, 0, 0, 0);
}

static inline VkRectangleStyle vk_rectangle_style_default(void) {
    VkRectangleStyle style;

    style.color_kind = VK_RECTANGLE_COLOR_SURFACE;
    style.custom_color = vk_color_transparent();

    style.radius_kind = VK_CORNER_RADIUS_NONE;
    style.radius = 0.0f;

    style.border_kind = VK_BORDER_NONE;
    style.border_color = vk_color_transparent();
    style.border_width = 0.0f;

    return style;
}

uint32_t vk_abi_version(void);
VkString vk_status_name(int32_t status);
VkRuntime *vk_runtime_create(uint64_t component_instance_id);
int32_t vk_runtime_destroy(VkRuntime *runtime);
int32_t vk_tree_begin(VkRuntime *runtime, uint64_t root_node_id);
int32_t vk_tree_abort(VkRuntime *runtime);
int32_t vk_tree_commit(VkRuntime *runtime);
int32_t vk_end_node(VkRuntime *runtime);
int32_t vk_begin_vstack(
    VkRuntime *runtime,
    uint64_t node_id,
    uint32_t gap,
    uint32_t alignment,
    uint32_t distribution
);
int32_t vk_begin_hstack(
    VkRuntime *runtime,
    uint64_t node_id,
    uint32_t gap,
    uint32_t alignment,
    uint32_t distribution
);
int32_t vk_begin_zstack(
    VkRuntime *runtime,
    uint64_t node_id,
    uint32_t alignment
);
int32_t vk_begin_padding(
    VkRuntime *runtime,
    uint64_t node_id,
    float top,
    float right,
    float bottom,
    float left
);
int32_t vk_begin_frame(
    VkRuntime *runtime,
    uint64_t node_id,
    VkLength width,
    VkLength height
);
int32_t vk_begin_background(
    VkRuntime *runtime,
    uint64_t node_id,
    VkRectangleStyle style
);
int32_t vk_push_text(
    VkRuntime *runtime,
    uint64_t node_id,
    VkString content,
    float font_size,
    float line_height,
    uint16_t weight,
    uint32_t alignment,
    uint32_t color
);
int32_t vk_push_button(
    VkRuntime *runtime,
    uint64_t node_id,
    VkString title,
    uint32_t color,
    float radius,
    uint64_t action_id
);
int32_t vk_push_spacer(
    VkRuntime *runtime,
    uint64_t node_id
);
int32_t vk_push_divider(
    VkRuntime *runtime,
    uint64_t node_id
);
int32_t vk_push_rectangle(
    VkRuntime *runtime,
    uint64_t node_id,
    VkRectangleStyle style
);
int32_t vk_runtime_collect_actions(VkRuntime *runtime);
int32_t vk_poll_action(
    VkRuntime *runtime,
    VkActionEvent *output,
    uint8_t *has_action
);

#ifdef __cplusplus
}
#endif

#endif /* VIEWKIT_H */
