mod background;
mod button;
mod card;
mod checkbox;
mod context_menu;
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

macro_rules! ffi_components {
    ($($tokens:tt)*) => {};
}

ffi_components! {
    container vk_begin_vstack(
        gap: u32
            => gap =
                decode_stack_gap(gap)?,

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
    ) build move |
        _node_id,
        children,
        _context
    | {
        let children =
            into_stack_children(
                children,
            );

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::VStack::new()
                    .gap(gap)
                    .alignment(alignment)
                    .distribution(
                        distribution,
                    )
                    .children(children),
            ),
        ))
    };

    container vk_begin_hstack(
        gap: u32
            => gap =
                decode_stack_gap(gap)?,

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
    ) build move |
        _node_id,
        children,
        _context
    | {
        let children =
            into_stack_children(
                children,
            );

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::HStack::new()
                    .gap(gap)
                    .alignment(alignment)
                    .distribution(
                        distribution,
                    )
                    .children(children),
            ),
        ))
    };

    container vk_begin_zstack(
        alignment: u32
            => alignment =
                decode_zstack_alignment(
                    alignment,
                )?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        let children =
            into_stack_children(
                children,
            );

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::ZStack::new()
                    .alignment(alignment)
                    .children(children),
            ),
        ))
    };

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
                decode_text_color(
                    color,
                )?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(
            children,
        )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Text::new(
                    content,
                )
                .font_family(
                    "Noto Sans JP",
                )
                .font_size(font_size)
                .line_height(line_height)
                .weight(weight)
                .alignment(alignment)
                .color(color),
            ),
        ))
    };

    leaf vk_push_button(
        title: VkString
            => title =
                copy_string(title)?,

        color: u32
            => color =
                decode_button_color(
                    color,
                )?,

        radius: f32
            => radius =
                sanitize_length(
                    radius,
                ),

        action_id: u64,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(
            children,
        )?;

        let mut button =
            crate::components::Button::new(
                title,
            )
            .color(color)
            .radius(
                crate::theme::CornerRadius::Custom(
                    radius,
                ),
            );

        if action_id != 0 {
            button = button.on_click(
                context.button_callback(
                    node_id,
                    action_id,
                ),
            );
        }

        Ok(FfiBuiltView::View(
            Box::new(button),
        ))
    };

    container vk_begin_padding(
        top: f32
            => top =
                sanitize_length(top),

        right: f32
            => right =
                sanitize_length(right),

        bottom: f32
            => bottom =
                sanitize_length(bottom),

        left: f32
            => left =
                sanitize_length(left),
    ) build move |
        _node_id,
        children,
        _context
    | {
        let content =
            zero_or_one_view(
                children,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Padding::only(
                    top,
                    right,
                    bottom,
                    left,
                )
                .content(content),
            ),
        ))
    };

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
    ) build move |
        _node_id,
        children,
        _context
    | {
        let mut child =
            zero_or_one_stack_child(
                children,
            )?;

        if let crate::layout::LayoutLength::Fixed(
            width,
        ) = width
        {
            child = child.width(width);
        }

        if let crate::layout::LayoutLength::Fixed(
            height,
        ) = height
        {
            child = child.height(height);
        }

        Ok(
            FfiBuiltView::StackChild(
                child,
            ),
        )
    };

    leaf vk_push_rectangle(
        style: VkRectangleStyle
            => style =
                decode_rectangle_style(
                    style,
                )?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(
            children,
        )?;

        Ok(FfiBuiltView::View(
            Box::new(
                build_rectangle(
                    style,
                ),
            ),
        ))
    };

    container vk_begin_background(
        style: VkRectangleStyle
            => style =
                decode_rectangle_style(
                    style,
                )?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        let content =
            zero_or_one_view(
                children,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Background::new()
                    .background(
                        build_rectangle(
                            style,
                        ),
                    )
                    .content(content),
            ),
        ))
    };

    leaf vk_push_spacer(
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(
            children,
        )?;

        Ok(
            FfiBuiltView::StackChild(
                crate::layout::IntoStackChild
                    ::into_stack_child(
                        crate::components::Spacer::new(),
                    ),
            ),
        )
    };

    leaf vk_push_divider(
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(
            children,
        )?;

        Ok(
            FfiBuiltView::StackChild(
                crate::layout::IntoStackChild
                    ::into_stack_child(
                        crate::components::Divider::new(),
                    ),
            ),
        )
    };
}
