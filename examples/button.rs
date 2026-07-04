use viewkit::prelude::*;

struct ButtonExample {
    message: State<String>,
}

impl ButtonExample {
    fn example_button(&self, title: &'static str, style: ButtonStyle) -> StackChild {
        let message = self.message.clone();

        Button::new(title)
            .style(style)
            .on_click(move || {
                message.set(format!("{title}ボタンがクリックされました"));
            })
            .frame(240.0, 44.0)
    }
}

impl App for ButtonExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self {
            message: State::new(String::from("ボタンを選択してください")),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Button Example")
            .size(720.0, 600.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
        Box::new(
            VStack::new()
                .gap(StackGap::Medium)
                .alignment(StackAlignment::Center)
                .distribution(StackDistribution::Center)
                .child(
                    Text::new(self.message.get())
                        .font_size(14.0)
                        .line_height(22.0)
                        .alignment(TextAlignment::Center)
                        .width(360.0),
                )
                .child(self.example_button("Standard", ButtonStyle::Standard))
                .child(self.example_button("Primary", ButtonStyle::Primary))
                .child(self.example_button("Accent", ButtonStyle::Accent))
                .child(self.example_button("Ghost", ButtonStyle::Ghost))
                .child(self.example_button("Danger", ButtonStyle::Danger))
                .child(
                    Button::new("Disabled")
                        .style(ButtonStyle::Primary)
                        .enabled(false)
                        .frame(240.0, 44.0),
                ),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<ButtonExample>()
}
