use viewkit::prelude::*;

const TEST_IMAGE: &[u8] = include_bytes!("resources/test.png");

struct ImageExample {
    image: ImageData,
}

impl ImageExample {
    fn preview(
        &self,
        title: &'static str,
        description: &'static str,
        content_mode: ImageContentMode,
        radius: CornerRadius,
    ) -> StackChild {
        /*
         * Image::frame()はStackChildを返すため、
         * ViewであるVStackの子として包みます。
         */
        let image = VStack::new()
            .gap(StackGap::None)
            .alignment(StackAlignment::Stretch)
            .child(
                Image::new(self.image.clone())
                    .content_mode(content_mode)
                    .radius(radius)
                    .frame(280.0, 180.0),
            );

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
                        .child(Text::new(description).font_size(11.0).line_height(18.0))
                        .child(
                            Background::new()
                                .background(
                                    Rectangle::new()
                                        .color(RectangleColor::Custom(Color::from_rgb_hex(
                                            0xeeeeee,
                                        )))
                                        .radius(CornerRadius::Large),
                                )
                                .content(image),
                        ),
                ),
            )
            .width(312.0)
    }
}

impl App for ImageExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        let image = ImageData::decode(TEST_IMAGE)
            .expect("examples/resources/test.pngを読み込めませんでした");

        Self { image }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Image Example")
            .size(1080.0, 520.0)
            .resizable(true)
    }

    fn body(&self, _context: &ViewContext) -> Box<dyn View + 'static> {
        let previews = HStack::new()
            .alignment(StackAlignment::Start)
            .gap(StackGap::Large)
            .child(self.preview(
                "Fit",
                "縦横比を維持して、画像全体を表示します。",
                ImageContentMode::Fit,
                CornerRadius::Small,
            ))
            .child(self.preview(
                "Fill",
                "縦横比を維持して、領域全体を覆います。",
                ImageContentMode::Fill,
                CornerRadius::Large,
            ))
            .child(self.preview(
                "Stretch",
                "縦横比を維持せず、領域全体へ引き伸ばします。",
                ImageContentMode::Stretch,
                CornerRadius::Full,
            ));

        Box::new(
            Padding::all(32.0).content(
                VStack::new()
                    .alignment(StackAlignment::Stretch)
                    .gap(StackGap::ExtraLarge)
                    .child(
                        Text::new("Image Component")
                            .font_size(28.0)
                            .line_height(36.0)
                            .weight(700),
                    )
                    .child(
                        Text::new("examples/resources/test.png")
                            .font_size(12.0)
                            .line_height(20.0),
                    )
                    .child(previews),
            ),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<ImageExample>()
}
