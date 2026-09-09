use viewkit::prelude::*;

struct ComponentLab {
    checkbox: State<bool>,
    switch: State<bool>,
    radio: State<usize>,
    segment: State<usize>,
    slider: State<f32>,
    small_field: State<String>,
    medium_field: State<String>,
    large_field: State<String>,
    selected_row: State<usize>,
    status: State<String>,
    image: ImageData,
    svg: SvgData,
}

impl ComponentLab {
    fn section(title: impl Into<String>, content: impl View + 'static, height: f32) -> StackChild {
        Card::new()
            .shadow(ShadowStyle::None)
            .content(
                Padding::all(16.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Medium)
                        .child(
                            Text::new(title.into())
                                .font_size(13.0)
                                .line_height(20.0)
                                .weight(700),
                        )
                        .child(content.layout().flex_grow(1.0)),
                ),
            )
            .height(height)
    }

    fn buttons(&self) -> StackChild {
        let status = self.status.clone();

        let row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                Button::new("Standard")
                    .on_click({
                        let status = status.clone();
                        move || status.set(String::from("Standard button clicked"))
                    })
                    .frame(104.0, 34.0),
            )
            .child(
                Button::new("Primary")
                    .style(ButtonStyle::Primary)
                    .on_click({
                        let status = status.clone();
                        move || status.set(String::from("Primary button clicked"))
                    })
                    .frame(96.0, 34.0),
            )
            .child(
                Button::new("Accent")
                    .style(ButtonStyle::Accent)
                    .on_click({
                        let status = status.clone();
                        move || status.set(String::from("Accent button clicked"))
                    })
                    .frame(88.0, 34.0),
            )
            .child(
                Button::new("Ghost")
                    .style(ButtonStyle::Ghost)
                    .on_click({
                        let status = status.clone();
                        move || status.set(String::from("Ghost button clicked"))
                    })
                    .frame(82.0, 34.0),
            )
            .child(
                Button::new("Danger")
                    .style(ButtonStyle::Danger)
                    .on_click(move || status.set(String::from("Danger button clicked")))
                    .frame(88.0, 34.0),
            )
            .child(Button::new("Disabled").enabled(false).frame(104.0, 34.0));

        Self::section(
            "Buttons",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Small)
                .child(row)
                .child(
                    Button::new("Leading aligned full width button")
                        .style(ButtonStyle::Standard)
                        .alignment(ZStackAlignment::Leading)
                        .height(38.0),
                ),
            128.0,
        )
    }

    fn form_controls(&self) -> StackChild {
        let radio = self.radio.clone();

        Self::section(
            "Form controls",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Small)
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Large)
                        .child(Checkbox::new(self.checkbox.binding()).label("Checkbox"))
                        .child(Switch::new(self.switch.binding()).label("Switch"))
                        .child(
                            Checkbox::new(State::new(false).binding())
                                .label("Disabled")
                                .enabled(false),
                        ),
                )
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Medium)
                        .child(RadioButton::new(radio.clone().binding(), 0).label("Compact"))
                        .child(RadioButton::new(radio.clone().binding(), 1).label("Comfortable"))
                        .child(
                            RadioButton::new(radio.binding(), 2)
                                .label("Disabled")
                                .enabled(false),
                        ),
                )
                .child(
                    SegmentedControl::new(self.segment.binding())
                        .item(0, "Layout")
                        .item(1, "States")
                        .disabled_item(2, "Disabled")
                        .height(36.0),
                )
                .child(
                    Slider::new(self.slider.binding())
                        .range(0.0..=100.0)
                        .step(5.0)
                        .label("Density")
                        .height(36.0),
                ),
            224.0,
        )
    }

    fn text_fields(&self) -> StackChild {
        Self::section(
            "Text fields",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Small)
                .child(
                    TextField::new(self.small_field.binding())
                        .placeholder("Small")
                        .size(TextFieldSize::Small)
                        .height(30.0),
                )
                .child(
                    TextField::new(self.medium_field.binding())
                        .placeholder("Medium")
                        .size(TextFieldSize::Medium)
                        .height(38.0),
                )
                .child(
                    TextField::new(self.large_field.binding())
                        .placeholder("Large")
                        .size(TextFieldSize::Large)
                        .height(46.0),
                )
                .child(
                    TextField::new(State::new(String::from("Invalid value")).binding())
                        .invalid(true)
                        .height(38.0),
                )
                .child(
                    TextField::new(State::new(String::from("Hidden token")).binding())
                        .secure(true)
                        .height(38.0),
                ),
            260.0,
        )
    }

    fn lists_and_menus(&self) -> StackChild {
        let rows = ["Navigation", "Settings", "Disabled row"]
            .into_iter()
            .enumerate()
            .fold(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::None),
                |stack, (index, label)| {
                    let selected = self.selected_row.get() == index;
                    let selected_row = self.selected_row.clone();
                    let status = self.status.clone();

                    stack.child(
                        ListRow::new(label)
                            .subtitle(if index == 0 {
                                "Selected list item"
                            } else {
                                "Secondary metadata"
                            })
                            .trailing(if index == 2 { "Off" } else { "On" })
                            .icon(if index == 1 {
                                IconName::Settings
                            } else {
                                IconName::LayoutList
                            })
                            .selected(selected)
                            .enabled(index != 2)
                            .on_select(move || {
                                selected_row.set(index);
                                status.set(format!("{label} selected"));
                            })
                            .height(54.0),
                    )
                },
            );

        let menu = Menu::new()
            .item(MenuItem::new("Open").shortcut("Enter").on_select(|| {
                println!("open");
            }))
            .item(MenuItem::new("Rename").shortcut("F2").on_select(|| {
                println!("rename");
            }))
            .item(MenuItem::new("Duplicate").enabled(false))
            .separator()
            .item(
                MenuItem::new("Delete")
                    .shortcut("Delete")
                    .danger(true)
                    .on_select(|| {
                        println!("delete");
                    }),
            )
            .width(220.0);

        Self::section(
            "List and context menu",
            ContextMenu::new(rows.layout().flex_grow(1.0), menu),
            240.0,
        )
    }

    fn media_and_shapes(&self) -> StackChild {
        Self::section(
            "Media and shapes",
            HStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    Image::new(self.image.clone())
                        .content_mode(ImageContentMode::Fill)
                        .radius(CornerRadius::Medium)
                        .frame(136.0, 112.0),
                )
                .child(
                    Svg::new(self.svg.clone())
                        .tint(Color::from_rgb_hex(0x0a84ff))
                        .frame(96.0, 112.0),
                )
                .child(
                    ZStack::new()
                        .alignment(ZStackAlignment::Center)
                        .child(
                            Rectangle::new()
                                .color(RectangleColor::Custom(Color::from_rgb_hex(0xe9f2ff)))
                                .radius(CornerRadius::Large)
                                .border(BorderStyle::custom(Color::from_rgb_hex(0x8ab8ff), 1.0)),
                        )
                        .child(
                            Ellipse::new()
                                .color(EllipseColor::Custom(Color::from_rgb_hex(0xffd166)))
                                .border(BorderStyle::custom(Color::from_rgb_hex(0x936900), 1.0))
                                .frame(68.0, 68.0),
                        )
                        .frame(136.0, 112.0),
                ),
            176.0,
        )
    }

    fn layout_playground(&self) -> StackChild {
        let overlay = Overlay::new()
            .content(
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::from_rgb_hex(0xf4f4f5)))
                    .radius(CornerRadius::Medium),
            )
            .overlay(
                Text::new("Overlay")
                    .font_size(12.0)
                    .line_height(20.0)
                    .weight(700)
                    .frame(72.0, 24.0),
            )
            .alignment(ZStackAlignment::Center)
            .frame(120.0, 72.0);

        let group = Group::new()
            .child(
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::from_rgb_hex(0xdff7ea)))
                    .radius(CornerRadius::Small)
                    .frame(48.0, 48.0),
            )
            .child(
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::from_rgb_hex(0xfce7f3)))
                    .radius(CornerRadius::Small)
                    .frame(72.0, 48.0),
            )
            .child(
                Rectangle::new()
                    .color(RectangleColor::Custom(Color::from_rgb_hex(0xfef3c7)))
                    .radius(CornerRadius::Small)
                    .frame(96.0, 48.0),
            );

        Self::section(
            "Layout",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .distribution(StackDistribution::SpaceBetween)
                        .gap(StackGap::Medium)
                        .child(group)
                        .child(Spacer::new()),
                )
                .child(Divider::new())
                .child(
                    Background::new()
                        .background(
                            Rectangle::new()
                                .color(RectangleColor::Custom(Color::from_rgb_hex(0xffffff)))
                                .radius(CornerRadius::Medium)
                                .border(BorderStyle::Standard { width: 1.0 }),
                        )
                        .content(
                            Padding::all(12.0).content(
                                HStack::new()
                                    .alignment(StackAlignment::Center)
                                    .gap(StackGap::Medium)
                                    .child(overlay)
                                    .child(
                                        VStack::new()
                                            .alignment(StackAlignment::Stretch)
                                            .gap(StackGap::ExtraSmall)
                                            .child(
                                                Text::new("Nested stack")
                                                    .font_size(12.0)
                                                    .line_height(20.0),
                                            )
                                            .child(
                                                Text::new("Spacing, background, overlay, divider")
                                                    .font_size(11.0)
                                                    .line_height(18.0),
                                            ),
                                    ),
                            ),
                        )
                        .height(104.0),
                ),
            236.0,
        )
    }
}

impl App for ComponentLab {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");

        Self {
            checkbox: State::new(true),
            switch: State::new(true),
            radio: State::new(1),
            segment: State::new(0),
            slider: State::new(60.0),
            small_field: State::new(String::from("Small input")),
            medium_field: State::new(String::from("Medium input")),
            large_field: State::new(String::from("Large input")),
            selected_row: State::new(0),
            status: State::new(String::from("Ready")),
            image: ImageData::from_path(format!("{manifest_dir}/examples/resources/test.png"))
                .expect("test image should load"),
            svg: SvgData::from_path(format!("{manifest_dir}/examples/resources/test.svg"))
                .expect("test SVG should load"),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Component Lab")
            .size(1180.0, 820.0)
            .resizable(true)
    }

    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static> {
        let wide = context.size().width >= 980.0;

        let content: Box<dyn View> = if wide {
            Box::new(
                HStack::new()
                    .alignment(StackAlignment::Start)
                    .gap(StackGap::Medium)
                    .child(
                        VStack::new()
                            .alignment(StackAlignment::Stretch)
                            .gap(StackGap::Medium)
                            .child(self.buttons())
                            .child(self.form_controls())
                            .child(self.text_fields())
                            .width(540.0),
                    )
                    .child(
                        VStack::new()
                            .alignment(StackAlignment::Stretch)
                            .gap(StackGap::Medium)
                            .child(self.lists_and_menus())
                            .child(self.media_and_shapes())
                            .child(self.layout_playground())
                            .width(540.0),
                    ),
            )
        } else {
            Box::new(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::Medium)
                    .child(self.buttons())
                    .child(self.form_controls())
                    .child(self.text_fields())
                    .child(self.lists_and_menus())
                    .child(self.media_and_shapes())
                    .child(self.layout_playground()),
            )
        };

        let content_height = if wide { 740.0 } else { 1340.0 };

        let body = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(
                Padding::symmetric(20.0, 14.0).content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Medium)
                        .child(
                            Text::new("ViewKit Component Lab")
                                .font_size(18.0)
                                .line_height(28.0)
                                .weight(800),
                        )
                        .child(Spacer::new())
                        .child(
                            Text::new(self.status.get())
                                .font_size(12.0)
                                .line_height(20.0)
                                .alignment(TextAlignment::End),
                        ),
                ),
            )
            .child(Divider::new())
            .child(
                Scroll::vertical(Padding::all(20.0).content(content).height(content_height))
                    .layout()
                    .flex_grow(1.0),
            );

        Box::new(
            Background::new()
                .background(
                    Rectangle::new().color(RectangleColor::Custom(Color::from_rgb_hex(0xf7f7f7))),
                )
                .content(body),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<ComponentLab>()
}
