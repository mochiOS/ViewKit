use viewkit::prelude::*;

struct ComponentsGallery {
    theme_mode: State<usize>,
    checked: State<bool>,
    switched: State<bool>,
    radio: State<usize>,
    segment: State<usize>,
    tab: State<usize>,
    slider: State<f32>,
    range_lower: State<f32>,
    range_upper: State<f32>,
    stepper: State<i32>,
    field: State<String>,
    editor: State<String>,
    picker: State<String>,
    dialog_open: State<bool>,
    popover_open: State<bool>,
    status: State<String>,
    image: ImageData,
    svg: SvgData,
}

impl ComponentsGallery {
    fn section(title: &str, subtitle: &str, content: impl View + 'static) -> StackChild {
        Card::new()
            .content(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::Medium)
                    .child(PageHeader::new(title).subtitle(subtitle))
                    .child(Divider::new())
                    .child(content),
            )
            .layout()
    }

    fn buttons(&self) -> StackChild {
        let status = self.status.clone();
        Self::section(
            "Buttons & Actions",
            "Semantic hierarchy, sizes, disabled state, icons, and omochi feedback.",
            VStack::new()
                .alignment(StackAlignment::Start)
                .gap(StackGap::Small)
                .child(
                    HStack::new()
                        .gap(StackGap::Small)
                        .alignment(StackAlignment::Center)
                        .child(Button::new("Standard"))
                        .child(Button::new("Primary").style(ButtonStyle::Primary))
                        .child(
                            Button::new("Accent")
                                .style(ButtonStyle::Accent)
                                .on_click(move || status.set("Accent button clicked".into())),
                        )
                        .child(Button::new("Ghost").style(ButtonStyle::Ghost))
                        .child(Button::new("Delete").style(ButtonStyle::Danger))
                        .child(Button::new("Disabled").enabled(false)),
                )
                .child(
                    HStack::new()
                        .gap(StackGap::Small)
                        .alignment(StackAlignment::Center)
                        .child(Button::new("Small").size(ButtonSize::Small))
                        .child(Button::new("Medium").size(ButtonSize::Medium))
                        .child(Button::new("Large").size(ButtonSize::Large))
                        .child(IconButton::new(SymbolName::Plus).accessibility_label("Add item"))
                        .child(
                            IconButton::new(SymbolName::ArrowUp)
                                .tone(IconButtonTone::Accent)
                                .accessibility_label("Send"),
                        ),
                ),
        )
    }

    fn inputs(&self) -> StackChild {
        let picker = self.picker.clone();
        Self::section(
            "Inputs & Forms",
            "Editable, invalid, secure, multiline, picker, and stepper states.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    HStack::new()
                        .gap(StackGap::Medium)
                        .child(
                            TextField::new(self.field.binding())
                                .placeholder("Editable text")
                                .leading_symbol(SymbolName::Search)
                                .layout()
                                .flex_grow(1.0),
                        )
                        .child(
                            TextField::new(State::new(String::new()).binding())
                                .placeholder("Invalid")
                                .invalid(true),
                        )
                        .child(
                            TextField::new(State::new("secret".into()).binding())
                                .secure(true)
                                .enabled(false),
                        ),
                )
                .child(
                    TextEditor::new(self.editor.binding())
                        .placeholder("Write multiple lines…")
                        .line_wrap(true)
                        .bordered(true)
                        .height(112.0),
                )
                .child(
                    HStack::new()
                        .gap(StackGap::Medium)
                        .alignment(StackAlignment::Center)
                        .child(
                            Picker::new(self.picker.get())
                                .option("Tokyo", {
                                    let picker = picker.clone();
                                    move || picker.set("Tokyo".into())
                                })
                                .option("Kyoto", move || picker.set("Kyoto".into()))
                                .disabled_option("Unavailable"),
                        )
                        .child(Stepper::new(self.stepper.binding()).range(0, 10))
                        .child(Text::body(format!("Value: {}", self.stepper.get()))),
                ),
        )
    }

    fn selection(&self) -> StackChild {
        Self::section(
            "Selection Controls",
            "Native state, keyboard focus, and value-following omochi thumbs.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    HStack::new()
                        .gap(StackGap::Large)
                        .alignment(StackAlignment::Center)
                        .child(Checkbox::new(self.checked.binding()).label("Checkbox"))
                        .child(Switch::new(self.switched.binding()).label("Switch"))
                        .child(RadioButton::new(self.radio.binding(), 0).label("Compact"))
                        .child(RadioButton::new(self.radio.binding(), 1).label("Comfortable")),
                )
                .child(
                    SegmentedControl::new(self.segment.binding())
                        .item(0, "List")
                        .item(1, "Grid")
                        .disabled_item(2, "Columns")
                        .accessibility_label("Layout"),
                )
                .child(
                    Slider::new(self.slider.binding())
                        .range(0.0..=100.0)
                        .step(1.0)
                        .label(format!("Slider: {:.0}", self.slider.get())),
                )
                .child(
                    RangeSlider::new(self.range_lower.binding(), self.range_upper.binding())
                        .range(0.0..=100.0)
                        .step(1.0)
                        .label(format!(
                            "Range: {:.0}–{:.0}",
                            self.range_lower.get(),
                            self.range_upper.get()
                        )),
                ),
        )
    }

    fn navigation(&self) -> StackChild {
        Self::section(
            "Navigation",
            "Tabs, sidebar/list selection, toolbars, and compact split layouts.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    Tabs::new(self.tab.binding())
                        .item(0, "General")
                        .item(1, "Details")
                        .disabled_item(2, "History"),
                )
                .child(Toolbar::new(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .child(Text::body_emphasized("Toolbar"))
                        .child(Spacer::new())
                        .child(
                            IconButton::new(SymbolName::More).accessibility_label("More actions"),
                        ),
                ))
                .child(
                    NavigationSplitView::new(
                        Sidebar::new(
                            SidebarSection::new("Library")
                                .item(SidebarItem::new("Overview").selected(true))
                                .item(SidebarItem::new("Favorites")),
                        ),
                        ContentArea::new(Text::body("Resizable detail region")),
                    )
                    .height(180.0),
                ),
        )
    }

    fn typography(&self) -> StackChild {
        Self::section(
            "Text & Typography",
            "Semantic roles and tones from the shared typography tokens.",
            VStack::new()
                .alignment(StackAlignment::Start)
                .gap(StackGap::ExtraSmall)
                .child(Text::styled("Display", TextRole::DisplayMedium))
                .child(Text::styled("Title large", TextRole::TitleLarge))
                .child(Text::body("Body text balances density and readability."))
                .child(Text::label("Label"))
                .child(Text::caption("Caption and metadata"))
                .child(Text::body("Secondary text").tone(TextTone::Secondary)),
        )
    }

    fn feedback(&self) -> StackChild {
        Self::section(
            "Feedback & Status",
            "Badges, progress, message direction, tooltip, and current live state.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    HStack::new()
                        .gap(StackGap::Small)
                        .child(Badge::new("Neutral"))
                        .child(Badge::new("Accent").tone(BadgeTone::Accent))
                        .child(Badge::new("Success").tone(BadgeTone::Success))
                        .child(Badge::new("Warning").tone(BadgeTone::Warning))
                        .child(Badge::new("Error").tone(BadgeTone::Error)),
                )
                .child(ProgressBar::new(self.slider.get() / 100.0))
                .child(MessageBubble::new("Received message"))
                .child(MessageBubble::new("Sent message").direction(MessageDirection::Sent))
                .child(Tooltip::new("Hover targets use concise help"))
                .child(Text::metadata(format!("Status: {}", self.status.get()))),
        )
    }

    fn surfaces(&self) -> StackChild {
        Self::section(
            "Surfaces, Layout & Media",
            "Token-backed containers, layout primitives, imagery, symbols, and drop targets.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    AdaptiveGrid::new(150.0, 88.0)
                        .spacing(12.0, 12.0)
                        .child(Surface::new(SurfaceKind::Pane).content(Text::body("Pane surface")))
                        .child(
                            Card::new().content(
                                HStack::new()
                                    .gap(StackGap::Small)
                                    .child(Avatar::new("OM"))
                                    .child(Text::body("Card")),
                            ),
                        )
                        .child(
                            Background::new()
                                .background(
                                    Rectangle::new()
                                        .color(RectangleColor::SubtleSurface)
                                        .radius(CornerRadius::Medium),
                                )
                                .content(Text::body("Background")),
                        )
                        .child(
                            ZStack::new()
                                .alignment(ZStackAlignment::Center)
                                .child(Ellipse::new().color(EllipseColor::Accent).frame(44.0, 44.0))
                                .child(Icon::new(SymbolName::Checkmark).size(18.0)),
                        ),
                )
                .child(
                    HStack::new()
                        .gap(StackGap::Large)
                        .child(
                            Image::new(self.image.clone())
                                .content_mode(ImageContentMode::Fill)
                                .radius(CornerRadius::Medium)
                                .accessibility_label("Sample image")
                                .frame(120.0, 72.0),
                        )
                        .child(
                            Svg::new(self.svg.clone())
                                .accessibility_label("Sample vector")
                                .frame(72.0, 72.0),
                        )
                        .child(ApplicationPlaceholder::new("ViewKit").frame(72.0, 72.0))
                        .child(FileDropTarget::new(
                            Card::new().content(Text::body("Drop a file here")),
                            {
                                let status = self.status.clone();
                                move |path| status.set(format!("Dropped {}", path.display()))
                            },
                        )),
                ),
        )
    }

    fn application_patterns(&self) -> StackChild {
        let grouped_rows = Group::new()
            .child(ListRow::new("Default row").subtitle("Secondary information"))
            .child(
                ListRow::new("Selected row")
                    .selected(true)
                    .status_marker(true),
            );
        Self::section(
            "Application Layouts",
            "List, form, settings, padding, grouping, and full application navigation primitives.",
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Large)
                .child(List::new().row(grouped_rows))
                .child(
                    Form::new(
                        FormSections::new().section(
                            Padding::all(12.0).content(
                                HStack::new()
                                    .gap(StackGap::Medium)
                                    .child(Text::body("Form content"))
                                    .child(Spacer::new())
                                    .child(Button::new("Action")),
                            ),
                        ),
                    )
                    .height(64.0),
                )
                .child(
                    SettingsPage::new(
                        "Settings",
                        SettingsSection::new("Appearance")
                            .row(
                                SettingsRow::new(
                                    "Automatic updates",
                                    Switch::new(self.switched.binding()),
                                )
                                .description("Uses the same Switch shown above."),
                            )
                            .row(SettingsRow::new("Density", Picker::new("Comfortable"))),
                    )
                    .height(220.0),
                )
                .child(
                    NavigationLayout::new(
                        Text::body_emphasized("Navigation toolbar"),
                        SidebarSection::new("Places")
                            .item(SidebarItem::new("Home").selected(true))
                            .item(SidebarItem::new("Documents")),
                        ContentArea::new(Text::body("NavigationLayout detail surface")),
                    )
                    .height(240.0),
                ),
        )
    }

    fn overlays(&self) -> StackChild {
        let open_dialog = self.dialog_open.clone();
        let open_popover = self.popover_open.clone();
        let status = self.status.clone();
        Self::section(
            "Overlays & Menus",
            "Picker-backed platform menus, context menus, popovers, and modal dialogs.",
            VStack::new()
                .alignment(StackAlignment::Start)
                .gap(StackGap::Medium)
                .child(
                    HStack::new()
                        .gap(StackGap::Small)
                        .child(Button::new("Show dialog").on_click(move || open_dialog.set(true)))
                        .child(
                            Button::new("Toggle popover")
                                .on_click(move || open_popover.set(!open_popover.get())),
                        )
                        .child(ContextMenu::new(
                            Button::new("Right-click me"),
                            Menu::new()
                                .item(MenuItem::new("Copy").shortcut("⌘C"))
                                .separator()
                                .item(
                                    MenuItem::new("Delete")
                                        .danger(true)
                                        .on_select(move || status.set("Delete selected".into())),
                                ),
                        )),
                )
                .child(
                    Menu::new()
                        .item(MenuItem::new("New document").shortcut("⌘N"))
                        .item(MenuItem::new("Open…").shortcut("⌘O"))
                        .separator()
                        .item(MenuItem::new("Unavailable").enabled(false)),
                ),
        )
    }

    fn page(&self) -> Box<dyn View + 'static> {
        let theme_mode = self.theme_mode.get();
        let mut gallery = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::Large)
            .child(
                HStack::new()
                    .alignment(StackAlignment::Center)
                    .gap(StackGap::Medium)
                    .child(
                        VStack::new()
                            .alignment(StackAlignment::Start)
                            .gap(StackGap::ExtraSmall)
                            .child(Text::styled(
                                "ViewKit Component Gallery",
                                TextRole::TitleLarge,
                            ))
                            .child(Text::body(
                                "omochi design language · live public components",
                            )),
                    )
                    .child(Spacer::new())
                    .child(
                        SegmentedControl::new(self.theme_mode.binding())
                            .item(0, "System")
                            .item(1, "Light")
                            .item(2, "Dark")
                            .accessibility_label("Appearance"),
                    ),
            )
            .child(Text::metadata(format!(
                "Theme: {}",
                ["System", "Light", "Dark"][theme_mode.min(2)]
            )))
            .child(self.buttons())
            .child(self.inputs())
            .child(self.selection())
            .child(self.navigation())
            .child(self.typography())
            .child(self.feedback())
            .child(self.surfaces())
            .child(self.application_patterns())
            .child(self.overlays());

        if self.popover_open.get() {
            let close = self.popover_open.clone();
            gallery = gallery.child(
                Popover::new().content(
                    VStack::new()
                        .gap(StackGap::Small)
                        .child(Text::body_emphasized("Popover"))
                        .child(Text::body("Escape or the button dismisses this surface."))
                        .child(Button::new("Close").on_click(move || close.set(false))),
                ),
            );
        }

        let page = ContentArea::new(Scroll::vertical(gallery));
        if self.dialog_open.get() {
            let close = self.dialog_open.clone();
            Box::new(
                Overlay::new().content(page).overlay(
                    Dialog::new()
                        .accessibility_label("Gallery dialog")
                        .on_dismiss({
                            let close = close.clone();
                            move || close.set(false)
                        })
                        .content(
                            VStack::new()
                                .alignment(StackAlignment::Stretch)
                                .gap(StackGap::Medium)
                                .child(Text::styled("Dialog", TextRole::TitleMedium))
                                .child(Text::body("A real ViewKit dialog using shared tokens."))
                                .child(Spacer::new())
                                .child(
                                    HStack::new().child(Spacer::new()).child(
                                        Button::new("Done").on_click(move || close.set(false)),
                                    ),
                                ),
                        )
                        .frame(420.0, 220.0),
                ),
            )
        } else {
            Box::new(page)
        }
    }
}

impl App for ComponentsGallery {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self {
            theme_mode: State::new(0),
            checked: State::new(true),
            switched: State::new(true),
            radio: State::new(0),
            segment: State::new(0),
            tab: State::new(0),
            slider: State::new(62.0),
            range_lower: State::new(24.0),
            range_upper: State::new(76.0),
            stepper: State::new(3),
            field: State::new(String::new()),
            editor: State::new("ViewKit keeps interaction and appearance in sync.".into()),
            picker: State::new("Tokyo".into()),
            dialog_open: State::new(false),
            popover_open: State::new(false),
            status: State::new("Ready".into()),
            image: ImageData::decode(include_bytes!("resources/test.png"))
                .expect("gallery image must decode"),
            svg: SvgData::decode(include_bytes!("resources/test.svg"))
                .expect("gallery SVG must decode"),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Component Gallery")
            .size(1120.0, 820.0)
            .resizable(true)
    }

    fn theme(&self) -> Option<Theme> {
        match self.theme_mode.get() {
            1 => Some(Theme::LIGHT),
            2 => Some(Theme::DARK),
            _ => None,
        }
    }

    fn body(&self, _context: &ViewContext) -> Self::Body {
        self.page()
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<ComponentsGallery>()
}
