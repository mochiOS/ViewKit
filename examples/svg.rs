use viewkit::prelude::*;

const TEST_SVG: &[u8] = include_bytes!("resources/test.svg");

struct SvgExample {
    svg: SvgData,
}

impl SvgExample {
    fn preview(&self, title: &'static str, description: &'static str, svg: Svg) -> StackChild {
        Card::new()
            .content(
                Padding::all(16.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Small)
                        .child(
                            Text::new(title)
                                .font_size(16.0)
                                .line_height(24.0)
                                .weight(700),
                        )
                        .child(Text::new(description).font_size(12.0).line_height(18.0))
                        .child(svg.frame(240.0, 160.0)),
                ),
            )
            .width(272.0)
    }
}

impl App for SvgExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        let svg =
            SvgData::decode(TEST_SVG).expect("examples/resources/test.svgを読み込めませんでした");

        Self { svg }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit SVG Example")
            .size(960.0, 500.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
        let original = self.preview(
            "Original",
            "SVGに定義された色をそのまま表示します。",
            Svg::new(self.svg.clone())
                .content_mode(SvgContentMode::Fit)
                .radius(CornerRadius::Large),
        );

        let tinted = self.preview(
            "Tint",
            "SVG全体を指定した色で単色化します。",
            Svg::new(self.svg.clone())
                .content_mode(SvgContentMode::Fit)
                .radius(CornerRadius::Large)
                .tint(Color::rgba(48, 105, 255, 255)),
        );

        let filled = self.preview(
            "Fill",
            "領域全体を覆い、角丸でクリップします。",
            Svg::new(self.svg.clone())
                .content_mode(SvgContentMode::Fill)
                .radius(CornerRadius::ExtraLarge)
                .opacity(0.9),
        );

        Box::new(
            Padding::all(32.0).content(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::ExtraLarge)
                    .child(
                        Text::new("SVG Component")
                            .font_size(28.0)
                            .line_height(36.0)
                            .weight(700),
                    )
                    .child(
                        Text::new("examples/resources/test.svg")
                            .font_size(12.0)
                            .line_height(20.0),
                    )
                    .child(
                        HStack::new()
                            .alignment(StackAlignment::Start)
                            .gap(StackGap::Large)
                            .child(original)
                            .child(tinted)
                            .child(filled),
                    ),
            ),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<SvgExample>()
}
