use viewkit::prelude::*;

struct TextFieldExample {
    name: State<String>,
    project: State<String>,
    email: State<String>,
    disabled: State<String>,
}

impl App for TextFieldExample {
    fn new() -> Self {
        Self {
            name: State::new(String::new()),
            project: State::new(String::from("mochiOS")),
            email: State::new(String::from("invalid@example")),
            disabled: State::new(String::from("使用できません")),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit TextField Example")
            .size(720.0, 520.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
        let email = self.email.get();
        let email_is_invalid = !email.is_empty() && !email.contains('.');

        Box::new(
            VStack::new()
                .gap(StackGap::Large)
                .alignment(StackAlignment::Center)
                .distribution(StackDistribution::Center)
                .child(
                    TextField::new(self.name.binding())
                        .placeholder("名前を入力")
                        .frame(320.0, 36.0),
                )
                .child(TextField::new(self.project.binding()).frame(320.0, 36.0))
                .child(
                    TextField::new(self.email.binding())
                        .invalid(email_is_invalid)
                        .frame(320.0, 36.0),
                )
                .child(
                    TextField::new(self.disabled.binding())
                        .size(TextFieldSize::Large)
                        .enabled(false)
                        .frame(320.0, 44.0),
                ),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<TextFieldExample>()
}
