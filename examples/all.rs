use viewkit::prelude::*;

struct ComponentLab {
    selected_conversation: State<usize>,
    notifications: State<bool>,
    pinned: State<bool>,
    radio: State<usize>,
    tabs: State<usize>,
    segment: State<usize>,
    density: State<f32>,
    count: State<i32>,
    composer: State<String>,
    field: State<String>,
}

impl ComponentLab {
    fn sidebar_row(
        &self,
        index: usize,
        initials: &'static str,
        name: &'static str,
        preview: &'static str,
        time: &'static str,
        unread: bool,
    ) -> StackChild {
        let selected = self.selected_conversation.get() == index;
        let selected_conversation = self.selected_conversation.clone();

        ListRow::new(name)
            .subtitle(preview)
            .leading_avatar(initials)
            .trailing(time)
            .status_marker(unread)
            .selected(selected)
            .on_select(move || selected_conversation.set(index))
            .layout()
    }

    fn sidebar(&self) -> Box<dyn View + 'static> {
        let header = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(Text::body_emphasized("Messages"))
            .child(Spacer::new())
            .child(IconButton::new(IconName::Search));

        Box::new(
            VStack::new()
                .alignment(StackAlignment::Start)
                .gap(StackGap::Large)
                .child(header)
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::ExtraSmall)
                        .child(self.sidebar_row(
                            0,
                            "A",
                            "Aya Sato",
                            "Sounds good — see you at 3.",
                            "10:42",
                            false,
                        ))
                        .child(self.sidebar_row(
                            1,
                            "L",
                            "Leo Tanaka",
                            "I sent the latest build.",
                            "9:18",
                            true,
                        ))
                        .child(self.sidebar_row(
                            2,
                            "N",
                            "Taro Suzuki",
                            "Thanks! I'll take a look.",
                            "",
                            false,
                        ))
                        .child(self.sidebar_row(
                            3,
                            "E",
                            "Suzu Okada",
                            "Coffee next week?",
                            "Tue",
                            false,
                        ))
                        .layout()
                        .flex_grow(1.0),
                ),
        )
    }

    fn header(&self) -> Box<dyn View + 'static> {
        Box::new(
            HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::Medium)
                .child(Avatar::new("A"))
                .child(Text::body_emphasized("Aya Sato"))
                .child(Spacer::new()),
        )
    }

    fn received(text: &'static str) -> StackChild {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .child(MessageBubble::new(text))
            .child(Spacer::new())
            .layout()
    }

    fn sent(text: &'static str) -> StackChild {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .child(Spacer::new())
            .child(MessageBubble::new(text).direction(MessageDirection::Sent))
            .layout()
    }

    fn messages(&self) -> Box<dyn View + 'static> {
        Box::new(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Medium)
                .child(
                    Text::metadata("Today · 10:42").alignment(TextAlignment::Center),
                )
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::ExtraSmall)
                        .child(Self::received("Hey! Are we still on for this afternoon?"))
                        .child(Self::received(
                            "We've encountered a problem with the project\nand would like to discuss it with someone.",
                        )),
                )
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::ExtraSmall)
                        .child(Self::sent("Yes — 3:00 works for me."))
                        .child(Self::sent(
                            "Let’s meet in the studio.\nI’ll have the notes ready.",
                        )),
                )
                .child(Self::received("Perfect. See you then!"))
                .child(Text::metadata("Read 10:43")),
        )
    }

    fn composer(&self) -> Box<dyn View + 'static> {
        Box::new(
            HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::Small)
                .child(IconButton::new(IconName::Plus))
                .child(
                    TextField::new(self.composer.binding())
                        .placeholder("Message")
                        .capsule()
                        .layout()
                        .flex_grow(1.0),
                )
                .child(IconButton::new(IconName::ArrowUp).tone(IconButtonTone::Accent)),
        )
    }

    fn component_gallery(&self) -> Box<dyn View + 'static> {
        Box::new(ContentArea::new(Scroll::vertical(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Large)
                .child(Text::styled("ViewKit Components", TextRole::TitleLarge))
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Start)
                        .gap(StackGap::Small)
                        .child(Text::styled("Typography", TextRole::TitleSmall))
                        .child(Text::styled("Display Large", TextRole::DisplayLarge))
                        .child(Text::styled("Display Medium", TextRole::DisplayMedium))
                        .child(Text::styled("Title Large", TextRole::TitleLarge))
                        .child(Text::styled("Title Medium", TextRole::TitleMedium))
                        .child(Text::styled("Title Small", TextRole::TitleSmall))
                        .child(Text::body("Body"))
                        .child(Text::label("Label"))
                        .child(Text::caption("Caption")),
                )
                .child(Divider::new())
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Start)
                        .gap(StackGap::Small)
                        .child(Text::styled("Buttons", TextRole::TitleSmall))
                        .child(
                            HStack::new()
                                .alignment(StackAlignment::Center)
                                .gap(StackGap::Small)
                                .child(Button::new("Standard"))
                                .child(Button::new("Primary").style(ButtonStyle::Primary))
                                .child(Button::new("Accent").style(ButtonStyle::Accent))
                                .child(Button::new("Ghost").style(ButtonStyle::Ghost))
                                .child(Button::new("Danger").style(ButtonStyle::Danger))
                                .child(Button::new("Disabled").enabled(false))
                                .child(IconButton::new(IconName::Settings))
                                .child(
                                    IconButton::new(IconName::ArrowUp).tone(IconButtonTone::Accent),
                                ),
                        ),
                )
                .child(Divider::new())
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Start)
                        .gap(StackGap::Small)
                        .child(Text::styled("Fields and Navigation", TextRole::TitleSmall))
                        .child(
                            HStack::new()
                                .alignment(StackAlignment::Center)
                                .gap(StackGap::Large)
                                .child(
                                    TextField::new(self.field.binding())
                                        .placeholder("Search or enter text"),
                                )
                                .child(Picker::new("Studio"))
                                .child(
                                    Tabs::new(self.tabs.binding())
                                        .item(0, "General")
                                        .item(1, "Details")
                                        .disabled_item(2, "History"),
                                ),
                        )
                        .child(
                            SegmentedControl::new(self.segment.binding())
                                .item(0, "List")
                                .item(1, "Grid")
                                .disabled_item(2, "Columns"),
                        ),
                )
                .child(Divider::new())
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Small)
                        .child(Badge::new("Neutral"))
                        .child(Badge::new("Active").tone(BadgeTone::Accent))
                        .child(Badge::new("Synced").tone(BadgeTone::Success))
                        .child(Badge::new("Draft").tone(BadgeTone::Warning))
                        .child(Badge::new("Error").tone(BadgeTone::Error)),
                )
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Large)
                        .child(Checkbox::new(self.pinned.binding()).label("Pinned"))
                        .child(Switch::new(self.notifications.binding()).label("Alerts"))
                        .child(RadioButton::new(self.radio.binding(), 0).label("Compact"))
                        .child(RadioButton::new(self.radio.binding(), 1).label("Roomy")),
                )
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Large)
                        .child(Stepper::new(self.count.binding()).range(0, 9))
                        .child(
                            Slider::new(self.density.binding())
                                .range(0.0..=100.0)
                                .step(5.0),
                        )
                        .child(ProgressBar::new(self.density.get() / 100.0)),
                )
                .child(Divider::new())
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Start)
                        .gap(StackGap::Small)
                        .child(Text::styled("Lists and Messages", TextRole::TitleSmall))
                        .child(ListRow::new("Default sidebar item").subtitle("Secondary text"))
                        .child(
                            ListRow::new("Selected sidebar item")
                                .subtitle("With status")
                                .selected(true)
                                .status_marker(true),
                        )
                        .child(MessageBubble::new("Received message bubble"))
                        .child(
                            MessageBubble::new("Sent message bubble")
                                .direction(MessageDirection::Sent),
                        ),
                )
                .child(Divider::new())
                .child(
                    HStack::new()
                        .alignment(StackAlignment::Start)
                        .gap(StackGap::Large)
                        .child(
                            VStack::new()
                                .alignment(StackAlignment::Start)
                                .gap(StackGap::Small)
                                .child(Text::styled("Menu", TextRole::TitleSmall))
                                .child(
                                    Menu::new()
                                        .item(MenuItem::new("New Document").shortcut("Ctrl+N"))
                                        .item(MenuItem::new("Open").shortcut("Ctrl+O"))
                                        .separator()
                                        .item(MenuItem::new("Delete").danger(true))
                                        .item(MenuItem::new("Unavailable").enabled(false)),
                                )
                                .child(Tooltip::new("Tooltip")),
                        )
                        .child(
                            Card::new().content(
                                VStack::new()
                                    .alignment(StackAlignment::Start)
                                    .gap(StackGap::Small)
                                    .child(Text::styled("Card", TextRole::TitleSmall))
                                    .child(Text::body("Content receives default insets.")),
                            ),
                        )
                        .child(
                            Popover::new().content(
                                VStack::new()
                                    .alignment(StackAlignment::Start)
                                    .gap(StackGap::Small)
                                    .child(Text::styled("Popover", TextRole::TitleSmall))
                                    .child(Text::body("Compact floating content.")),
                            ),
                        )
                        .child(
                            Dialog::new().content(
                                VStack::new()
                                    .alignment(StackAlignment::Stretch)
                                    .gap(StackGap::Medium)
                                    .child(Text::styled("Dialog", TextRole::TitleMedium))
                                    .child(Text::body("A modal surface with standard spacing."))
                                    .child(Spacer::new())
                                    .child(
                                        HStack::new()
                                            .alignment(StackAlignment::Center)
                                            .gap(StackGap::Small)
                                            .child(Spacer::new())
                                            .child(Button::new("Cancel"))
                                            .child(
                                                Button::new("Continue").style(ButtonStyle::Accent),
                                            ),
                                    ),
                            ),
                        ),
                ),
        )))
    }

    fn chat_pane(&self) -> Box<dyn View + 'static> {
        Box::new(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None)
                .child(Toolbar::new(self.header()))
                .child(
                    ContentArea::new(Scroll::vertical(self.messages()))
                        .layout()
                        .flex_grow(1.0),
                )
                .child(Toolbar::bottom(self.composer())),
        )
    }
}

impl App for ComponentLab {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self {
            selected_conversation: State::new(0),
            notifications: State::new(true),
            pinned: State::new(true),
            radio: State::new(0),
            tabs: State::new(0),
            segment: State::new(0),
            density: State::new(60.0),
            count: State::new(3),
            composer: State::new(String::new()),
            field: State::new(String::new()),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Chat Components")
            .size(1180.0, 760.0)
            .resizable(true)
    }

    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static> {
        let component_lab = context.size().width >= 1440.0;

        if component_lab {
            self.component_gallery()
        } else {
            Box::new(NavigationSplitView::new(self.sidebar(), self.chat_pane()))
        }
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<ComponentLab>()
}
