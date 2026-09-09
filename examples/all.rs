use viewkit::prelude::*;

struct ComponentLab {
    selected_conversation: State<usize>,
    tab: State<usize>,
    notifications: State<bool>,
    pinned: State<bool>,
    radio: State<usize>,
    density: State<f32>,
    count: State<i32>,
    composer: State<String>,
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
            .height(64.0)
    }

    fn sidebar(&self) -> Box<dyn View + 'static> {
        let header = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                Text::new("Messages")
                    .font_size(15.0)
                    .line_height(22.0)
                    .weight(500),
            )
            .child(Spacer::new())
            .child(IconButton::new(IconName::Search).frame(24.0, 24.0));

        Box::new(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Large)
                .child(header.height(24.0))
                .child(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::ExtraSmall)
                        .child(self.sidebar_row(
                            0,
                            "A",
                            "Aya Sato",
                            "Sounds good - see you at 3.",
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
                            "T",
                            "Taro Suzuki",
                            "Thanks! I'll take a look.",
                            "",
                            false,
                        ))
                        .child(self.sidebar_row(
                            3,
                            "S",
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
                .child(Avatar::new("A").frame(32.0, 32.0))
                .child(
                    Text::new("Aya Sato")
                        .font_size(15.0)
                        .line_height(22.0)
                        .weight(500),
                )
                .child(Spacer::new())
                .child(Badge::new("Online").tone(BadgeTone::Accent))
                .child(Picker::new("Studio").frame(120.0, 24.0))
                .child(IconButton::new(IconName::Settings).frame(24.0, 24.0)),
        )
    }

    fn received(text: &'static str, width: f32, height: f32) -> StackChild {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .child(MessageBubble::new(text).frame(width, height))
            .child(Spacer::new())
            .height(height)
    }

    fn sent(text: &'static str, width: f32, height: f32) -> StackChild {
        HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::None)
            .child(Spacer::new())
            .child(
                MessageBubble::new(text)
                    .direction(MessageDirection::Sent)
                    .frame(width, height),
            )
            .height(height)
    }

    fn messages(&self) -> Box<dyn View + 'static> {
        Box::new(VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::Medium)
            .child(
                Text::new("Today - 10:42")
                    .font_size(12.0)
                    .line_height(16.0)
                    .alignment(TextAlignment::Center)
                    .height(16.0),
            )
            .child(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::ExtraSmall)
                    .child(Self::received(
                        "Hey! Are we still on for this afternoon?",
                        295.0,
                        38.0,
                    ))
                    .child(Self::received(
                        "We've encountered a problem with the project\nand would like to discuss it with someone.",
                        351.0,
                        60.0,
                    )),
            )
            .child(Self::sent("Yes - 3:00 works for me.", 206.0, 38.0))
            .child(Self::sent(
                "Let's meet in the studio.\nI'll have the notes ready.",
                196.0,
                60.0,
            ))
            .child(Self::received("Perfect. See you then!", 182.0, 38.0))
            .child(
                Text::new("Read 10:43")
                    .font_size(12.0)
                    .line_height(16.0)
                    .height(16.0),
            )
        )
    }

    fn composer(&self) -> Box<dyn View + 'static> {
        Box::new(
            HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::Small)
                .child(IconButton::new(IconName::Plus).frame(24.0, 24.0))
                .child(
                    TextField::new(self.composer.binding())
                        .placeholder("Message")
                        .size(TextFieldSize::Medium)
                        .layout()
                        .flex_grow(1.0),
                )
                .child(
                    IconButton::new(IconName::ChevronRight)
                        .tone(IconButtonTone::Accent)
                        .frame(32.0, 32.0),
                ),
        )
    }

    fn component_dock(&self) -> Box<dyn View + 'static> {
        Box::new(
            Card::new().content(
                Padding::all(12.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Small)
                        .child(
                            HStack::new()
                                .alignment(StackAlignment::Center)
                                .gap(StackGap::Small)
                                .child(
                                    Tabs::new(self.tab.binding())
                                        .item(0, "Chat")
                                        .item(1, "Files")
                                        .disabled_item(2, "Muted"),
                                )
                                .child(Spacer::new())
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
                                .child(RadioButton::new(self.radio.binding(), 1).label("Roomy"))
                                .child(Stepper::new(self.count.binding()).range(0, 9))
                                .child(
                                    Slider::new(self.density.binding())
                                        .range(0.0..=100.0)
                                        .step(5.0)
                                        .width(160.0),
                                )
                                .child(ProgressBar::new(self.density.get() / 100.0).width(160.0)),
                        ),
                ),
            ),
        )
    }

    fn chat_pane(&self) -> Box<dyn View + 'static> {
        Box::new(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None)
                .child(
                    Padding::symmetric(24.0, 16.0)
                        .content(self.header())
                        .height(64.0),
                )
                .child(Divider::new())
                .child(
                    Scroll::vertical(Padding::all(24.0).content(self.messages()).height(560.0))
                        .layout()
                        .flex_grow(1.0),
                )
                .child(
                    Padding::symmetric(24.0, 12.0)
                        .content(self.composer())
                        .height(56.0),
                ),
        )
    }
}

impl App for ComponentLab {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self {
            selected_conversation: State::new(0),
            tab: State::new(0),
            notifications: State::new(true),
            pinned: State::new(true),
            radio: State::new(0),
            density: State::new(60.0),
            count: State::new(3),
            composer: State::new(String::new()),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Chat Components")
            .size(1180.0, 760.0)
            .resizable(true)
    }

    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static> {
        let compact = context.size().width < 900.0;

        let app_shell = if compact {
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None)
                .child(
                    Padding::only(16.0, 12.0, 16.0, 12.0)
                        .content(self.sidebar())
                        .height(280.0),
                )
                .child(Divider::new())
                .child(self.chat_pane())
                .layout()
        } else {
            HStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None)
                .child(
                    Surface::sidebar()
                        .content(Padding::only(16.0, 12.0, 16.0, 12.0).content(self.sidebar()))
                        .width(320.0),
                )
                .child(Divider::new())
                .child(self.chat_pane())
                .layout()
        };

        Box::new(
            Surface::app().content(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::None)
                    .child(app_shell.flex_grow(1.0))
                    .child(
                        Padding::only(0.0, 12.0, 12.0, 12.0)
                            .content(self.component_dock())
                            .height(128.0),
                    ),
            ),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<ComponentLab>()
}
