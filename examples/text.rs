use viewkit::prelude::*;

struct TextExample;

impl App for TextExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Text Example")
            .size(720.0, 520.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
        let text = Text::new(concat!(
            "おもちは白く、丸く、美味しく、幸福を運んでくれます。",
            "猫ははちゃめちゃに可愛く、すべてが愛らしい。",
            "どちらも日々の疲れを癒やす、尊い宝です。"
        ))
        .font_size(18.0)
        .line_height(30.0)
        .weight(600)
        .alignment(TextAlignment::Start)
        .color(Color::BLACK);

        let card = Background::new()
            .background(Rectangle::new().color(RectangleColor::ElevatedSurface))
            .content(Padding::symmetric(24.0, 18.0).content(text));

        Box::new(
            VStack::new()
                .alignment(StackAlignment::Center)
                .distribution(StackDistribution::Center)
                .child(card.width(420.0)),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<TextExample>()
}
