mod background;
mod button;
mod card;
mod checkbox;
mod context_menu;
mod divider;
mod ellipse;
mod group;
mod hstack;
mod icon;
mod image;
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
mod svg;
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
pub use scroll::{Scroll, ScrollAxis, ScrollBarVisibility, ScrollState};
pub use spacer::Spacer;
pub use vstack::VStack;
pub use zstack::{ZStack, ZStackAlignment};

pub use button::{Button, ButtonColor, ButtonInteractionState, ButtonStyle};
pub use card::Card;
pub use checkbox::Checkbox;
pub use context_menu::ContextMenu;
pub use ellipse::{Ellipse, EllipseColor};
pub use icon::{Icon, IconName};
pub use image::{Image, ImageContentMode};
pub use list::ListRow;
pub use menu::{Menu, MenuItem};
pub use radio::RadioButton;
pub use rectangle::{BorderStyle, Rectangle, RectangleColor};
pub use segment_control::SegmentedControl;
pub use slider::{Slider, SliderInteractionState};
pub use svg::{Svg, SvgContentMode};
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

        container vk_begin_card(
        style: VkRectangleStyle
            => style =
                decode_rectangle_style(style)?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        let content =
            zero_or_one_view(children)?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Card::new()
                    .color(style.color)
                    .radius(style.radius)
                    .border(style.border)
                    .content(content),
            ),
        ))
    };

    leaf vk_push_checkbox(
        state_id: u64,

        checked: u8
            => checked = checked != 0,

        label: VkString
            => label =
                copy_string(label)?,

        enabled: u8
            => enabled = enabled != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let checked =
            context.bool_binding(
                node_id,
                state_id,
                checked,
            )?;

        let mut checkbox =
            crate::components::Checkbox::new(
                checked,
            )
            .enabled(enabled);

        if !label.is_empty() {
            checkbox =
                checkbox.label(label);
        }

        Ok(FfiBuiltView::View(
            Box::new(checkbox),
        ))
    };

    container vk_begin_context_menu(
    ) build move |
        _node_id,
        children,
        _context
    | {
        let (content, menu) =
            exactly_two_stack_children(
                children,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::ContextMenu::new(
                    content,
                    menu,
                ),
            ),
        ))
    };

    leaf vk_push_ellipse(
        style: VkRectangleStyle
            => style =
                decode_rectangle_style(style)?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(children)?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Ellipse::new()
                    .color(style.color)
                    .border(style.border),
            ),
        ))
    };

    container vk_begin_group(
    ) build move |
        _node_id,
        children,
        _context
    | {
        Ok(FfiBuiltView::StackChildren(
            into_stack_children(children),
        ))
    };

    leaf vk_push_list_row(
        title: VkString
            => title =
                copy_string(title)?,

        subtitle: VkString
            => subtitle =
                copy_string(subtitle)?,

        trailing: VkString
            => trailing =
                copy_string(trailing)?,

        selected: u8
            => selected = selected != 0,

        enabled: u8
            => enabled = enabled != 0,

        action_id: u64,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let mut row =
            crate::components::ListRow::new(
                title,
            )
            .selected(selected)
            .enabled(enabled);

        if !subtitle.is_empty() {
            row = row.subtitle(subtitle);
        }

        if !trailing.is_empty() {
            row = row.trailing(trailing);
        }

        if action_id != 0 {
            row = row.on_select(
                context.button_callback(
                    node_id,
                    action_id,
                ),
            );
        }

        Ok(FfiBuiltView::View(
            Box::new(row),
        ))
    };

    leaf vk_push_menu_item(
        label: VkString
            => label =
                copy_string(label)?,

        shortcut: VkString
            => shortcut =
                copy_string(shortcut)?,

        enabled: u8
            => enabled = enabled != 0,

        danger: u8
            => danger = danger != 0,

        action_id: u64,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let mut item =
            crate::components::MenuItem::new(
                label,
            )
            .enabled(enabled)
            .danger(danger);

        if !shortcut.is_empty() {
            item =
                item.shortcut(shortcut);
        }

        if action_id != 0 {
            item = item.on_select(
                context.button_callback(
                    node_id,
                    action_id,
                ),
            );
        }

        Ok(FfiBuiltView::View(
            Box::new(item),
        ))
    };

    leaf vk_push_menu(
        entries: VkMenuEntries
            => entries =
                copy_menu_entries(entries)?,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let mut menu =
            crate::components::Menu::new();

        for entry in entries {
            match entry {
                DecodedMenuEntry::Separator => {
                    menu = menu.separator();
                }

                DecodedMenuEntry::Item {
                    label,
                    shortcut,
                    enabled,
                    danger,
                    action_id,
                } => {
                    let mut item =
                        crate::components::MenuItem::new(
                            label,
                        )
                        .enabled(enabled)
                        .danger(danger);

                    if let Some(shortcut) =
                        shortcut
                    {
                        item =
                            item.shortcut(shortcut);
                    }

                    if action_id != 0 {
                        item = item.on_select(
                            context.button_callback(
                                node_id,
                                action_id,
                            ),
                        );
                    }

                    menu = menu.item(item);
                }
            }
        }

        Ok(FfiBuiltView::View(
            Box::new(menu),
        ))
    };

    container vk_begin_overlay(
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
        let (content, overlay) =
            exactly_two_stack_children(
                children,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Overlay::new()
                    .content(content)
                    .overlay(overlay)
                    .alignment(alignment),
            ),
        ))
    };

    leaf vk_push_radio_button(
        state_id: u64,

        selection: u64
            => selection =
                decode_usize(selection)?,

        value: u64
            => value =
                decode_usize(value)?,

        label: VkString
            => label =
                copy_string(label)?,

        enabled: u8
            => enabled = enabled != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let selection =
            context.usize_binding(
                node_id,
                state_id,
                selection,
            )?;

        let mut radio =
            crate::components::RadioButton::new(
                selection,
                value,
            )
            .enabled(enabled);

        if !label.is_empty() {
            radio = radio.label(label);
        }

        Ok(FfiBuiltView::View(
            Box::new(radio),
        ))
    };

    container vk_begin_scroll(
        state_id: u64,

        axis: u32
            => axis =
                decode_scroll_axis(axis)?,

        scrollbar: u32
            => scrollbar =
                decode_scrollbar_visibility(
                    scrollbar,
                )?,
    ) build move |
        node_id,
        children,
        context
    | {
        let content =
            zero_or_one_stack_child(
                children,
            )?;

        let state =
            context.scroll_state(
                node_id,
                state_id,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Scroll::new(
                    state,
                )
                .axis(axis)
                .scrollbar(scrollbar)
                .content(content),
            ),
        ))
    };

    leaf vk_push_segmented_control(
        state_id: u64,

        selection: u64
            => selection =
                decode_usize(selection)?,

        items: VkSegmentedItems
            => items =
                copy_segmented_items(items)?,

        enabled: u8
            => enabled = enabled != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let selection =
            context.usize_binding(
                node_id,
                state_id,
                selection,
            )?;

        let mut control =
            crate::components::SegmentedControl::new(
                selection,
            )
            .enabled(enabled);

        for item in items {
            control = if item.enabled {
                control.item(
                    item.value,
                    item.label,
                )
            } else {
                control.disabled_item(
                    item.value,
                    item.label,
                )
            };
        }

        Ok(FfiBuiltView::View(
            Box::new(control),
        ))
    };

    leaf vk_push_slider(
        state_id: u64,

        value: f32,
        minimum: f32,
        maximum: f32,
        step: f32,

        label: VkString
            => label =
                copy_string(label)?,

        enabled: u8
            => enabled = enabled != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let value =
            context.float_binding(
                node_id,
                state_id,
                value,
            )?;

        let mut slider =
            crate::components::Slider::new(
                value,
            )
            .range(minimum..=maximum)
            .step(step)
            .enabled(enabled);

        if !label.is_empty() {
            slider = slider.label(label);
        }

        Ok(FfiBuiltView::View(
            Box::new(slider),
        ))
    };

    leaf vk_push_switch(
        state_id: u64,

        checked: u8
            => checked = checked != 0,

        label: VkString
            => label =
                copy_string(label)?,

        enabled: u8
            => enabled = enabled != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let checked =
            context.bool_binding(
                node_id,
                state_id,
                checked,
            )?;

        let mut switch =
            crate::components::Switch::new(
                checked,
            )
            .enabled(enabled);

        if !label.is_empty() {
            switch = switch.label(label);
        }

        Ok(FfiBuiltView::View(
            Box::new(switch),
        ))
    };

    leaf vk_push_text_field(
        state_id: u64,

        value: VkString
            => value =
                copy_string(value)?,

        placeholder: VkString
            => placeholder =
                copy_string(
                    placeholder,
                )?,

        size: u32
            => size =
                decode_text_field_size(
                    size,
                )?,

        radius: f32
            => radius =
                sanitize_length(radius),

        enabled: u8
            => enabled = enabled != 0,

        invalid: u8
            => invalid = invalid != 0,
    ) build move |
        node_id,
        children,
        context
    | {
        expect_no_children(children)?;

        let value =
            context.string_binding(
                node_id,
                state_id,
                value,
            )?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::TextField::new(
                    value,
                )
                .placeholder(placeholder)
                .size(size)
                .radius(
                    crate::theme::CornerRadius::Custom(
                        radius,
                    ),
                )
                .enabled(enabled)
                .invalid(invalid),
            ),
        ))
    };

    leaf vk_push_image(
        data: VkBytes
            => image =
                decode_image_data(data)?,

        content_mode: u32
            => content_mode =
                decode_image_content_mode(
                    content_mode,
                )?,

        radius_kind: u32,

        radius: f32
            => radius =
                decode_corner_radius(
                    radius_kind,
                    radius,
                )?,

        opacity: f32
            => opacity =
                sanitize_opacity(opacity),

        sampling: u32
            => sampling =
                decode_image_sampling(
                    sampling,
                )?,
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(children)?;

        Ok(FfiBuiltView::View(
            Box::new(
                crate::components::Image::new(
                    image,
                )
                .content_mode(
                    content_mode,
                )
                .radius(
                    radius,
                )
                .opacity(
                    opacity,
                )
                .sampling(
                    sampling,
                ),
            ),
        ))
    };

    leaf vk_push_svg(
    data: VkBytes
        => svg =
            decode_svg_data(data)?,

    content_mode: u32
        => content_mode =
            decode_svg_content_mode(
                content_mode,
            )?,

    radius_kind: u32,

    radius: f32
        => radius =
            decode_corner_radius(
                radius_kind,
                radius,
            )?,

    opacity: f32
        => opacity =
            sanitize_opacity(
                opacity,
            ),

    tint_enabled: u8
        => tint_enabled =
            tint_enabled != 0,

    tint: VkColor
        => tint =
            decode_optional_color(
                tint_enabled,
                tint,
            ),
    ) build move |
        _node_id,
        children,
        _context
    | {
        expect_no_children(
            children,
        )?;

        let mut view =
            crate::components::Svg::new(
                svg,
            )
            .content_mode(
                content_mode,
            )
            .radius(
                radius,
            )
            .opacity(
                opacity,
            );

        if let Some(tint) = tint {
            view = view.tint(
                tint,
            );
        }

        Ok(FfiBuiltView::View(
            Box::new(view),
        ))
    };
}
