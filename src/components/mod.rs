mod background;
mod button;
mod card;
mod checkbox;
pub mod context_menu;
mod divider;
mod ellipse;
mod group;
mod hstack;
mod list;
mod menu;
mod overlay;
mod padding;
mod radio;
mod rectangle;
mod scroll;
mod segment_control;
mod slider;
mod spacer;
mod switch;
mod text;
mod text_field;
mod vstack;
mod zstack;

pub use background::Background;
pub use divider::Divider;
pub use group::Group;
pub use hstack::HStack;
pub use overlay::Overlay;
pub use padding::Padding;
pub use scroll::{Scroll, ScrollAxis, ScrollState};
pub use spacer::Spacer;
pub use vstack::VStack;
pub use zstack::{ZStack, ZStackAlignment};

pub use button::{Button, ButtonColor, ButtonInteractionState, ButtonStyle};
pub use card::Card;
pub use checkbox::Checkbox;
pub use context_menu::ContextMenu;
pub use ellipse::{Ellipse, EllipseColor};
pub use list::ListRow;
pub use menu::{Menu, MenuItem};
pub use radio::RadioButton;
pub use rectangle::{BorderStyle, Rectangle, RectangleColor};
pub use segment_control::SegmentedControl;
pub use slider::{Slider, SliderInteractionState};
pub use switch::Switch;
pub use text::Text;
pub use text_field::{TextField, TextFieldInteractionState, TextFieldSize};

/// このマクロはRustコンパイル時には何も生成しません。
/// tools/ffi-genがmacroを読み取って生成するためにあります
macro_rules! ffi_components {
    ($($tokens:tt)*) => {};
}

ffi_components! {
    container vk_begin_vstack(
        gap: u32
            => gap = decode_stack_gap(gap)?,

        alignment: u32
            => alignment =
                decode_stack_alignment(
                    alignment,
                )?,

        distribution: u32
            => distribution =
                decode_stack_distribution(
                    distribution,
                )?,
    ) => crate::runtime::ViewNodeKind::VStack(
        crate::runtime::VStackNode {
            gap,
            alignment,
            distribution,
        }
    );

    container vk_begin_hstack(
        gap: u32
            => gap = decode_stack_gap(gap)?,

        alignment: u32
            => alignment =
                decode_stack_alignment(
                    alignment,
                )?,

        distribution: u32
            => distribution =
                decode_stack_distribution(
                    distribution,
                )?,
    ) => crate::runtime::ViewNodeKind::HStack(
        crate::runtime::HStackNode {
            gap,
            alignment,
            distribution,
        }
    );

    container vk_begin_zstack(
        alignment: u32
            => alignment =
                decode_zstack_alignment(
                    alignment,
                )?,
    ) => crate::runtime::ViewNodeKind::ZStack(
        crate::runtime::ZStackNode {
            alignment,
        }
    );

    container vk_begin_padding(
        top: f32
            => top = sanitize_length(top),

        right: f32
            => right =
                sanitize_length(right),

        bottom: f32
            => bottom =
                sanitize_length(bottom),

        left: f32
            => left =
                sanitize_length(left),
    ) => crate::runtime::ViewNodeKind::Padding(
        crate::runtime::PaddingNode {
            top,
            right,
            bottom,
            left,
        }
    );

    container vk_begin_frame(
        width: VkLength
            => width =
                decode_layout_length(
                    width,
                )?,

        height: VkLength
            => height =
                decode_layout_length(
                    height,
                )?,
    ) => crate::runtime::ViewNodeKind::Frame(
        crate::runtime::FrameNode {
            width,
            height,
        }
    );

    container vk_begin_background(
        style: VkRectangleStyle
            => properties =
                decode_rectangle_style(
                    style,
                )?,
    ) => crate::runtime::ViewNodeKind::Background(
        properties
    );

    leaf vk_push_text(
        content: VkString
            => content =
                copy_string(content)?,

        font_size: f32
            => font_size =
                finite_or_default(
                    font_size,
                    16.0,
                ),

        line_height: f32
            => line_height =
                finite_or_default(
                    line_height,
                    24.0,
                ),

        weight: u16,

        alignment: u32
            => alignment =
                decode_text_alignment(
                    alignment,
                )?,

        color: u32
            => color =
                decode_text_color(color)?,
    ) => crate::runtime::ViewNodeKind::Text(
        crate::runtime::TextNode {
            content,

            font_family:
                String::from(
                    "Noto Sans JP",
                ),

            font_size,
            line_height,
            weight,
            alignment,
            color,
        }
    );

    leaf vk_push_button(
        title: VkString
            => title =
                copy_string(title)?,

        color: u32
            => color =
                decode_button_color(color)?,

        radius: f32
            => radius =
                sanitize_length(radius),

        action_id: u64
            => action =
                if action_id == 0 {
                    None
                } else {
                    Some(
                        crate::runtime::ActionId(
                            action_id,
                        ),
                    )
                },
    ) => crate::runtime::ViewNodeKind::Button(
        crate::runtime::ButtonNode {
            title,
            color,
            radius,
            action,
        }
    );

    leaf vk_push_spacer(
    ) => crate::runtime::ViewNodeKind::Spacer;

    leaf vk_push_divider(
    ) => crate::runtime::ViewNodeKind::Divider;

    leaf vk_push_rectangle(
        style: VkRectangleStyle
            => properties =
                decode_rectangle_style(
                    style,
                )?,
    ) => crate::runtime::ViewNodeKind::Rectangle(
        properties
    );
}
