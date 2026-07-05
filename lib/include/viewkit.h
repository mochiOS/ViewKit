#ifndef VIEWKIT_H
#define VIEWKIT_H

#include "viewkit_abi.h"

static inline VkString vk_string(
    const void *pointer,
    size_t length
) {
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

#endif