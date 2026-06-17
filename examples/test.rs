use ui_layout::{
    AlignItems,
    Display,
    FlexDirection,
    JustifyContent,
    LayoutEngine,
    LayoutNode,
    Length,
    SizeStyle,
    Style,
};

use viewkit::components::Rectangle;
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::geometry::{
    Point,
    Size,
};
use viewkit::layout::border_box;
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::renderer::Viewport;
use viewkit::theme::Theme;
use viewkit::typography::Typography;
use viewkit::view::{
    PaintContext,
    View,
};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
        }
    }
}

impl PlatformApplication for ExampleApplication {
    fn handle_event(
        &mut self,
        event: PlatformEvent,
        _window: &dyn PlatformWindow,
    ) {
        match event {
            PlatformEvent::Resumed {
                viewport,
            } => {
                println!(
                    "resumed: {viewport:?}"
                );
            }

            PlatformEvent::Resized {
                viewport,
            } => {
                println!(
                    "resized: {viewport:?}"
                );
            }

            PlatformEvent::ScaleFactorChanged {
                viewport,
            } => {
                println!(
                    "scale factor changed: \
                     {viewport:?}"
                );
            }

            PlatformEvent::Focused(
                focused,
            ) => {
                println!(
                    "focused: {focused}"
                );
            }

            PlatformEvent::RedrawRequested => {}

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }
        }
    }

    fn draw(
        &mut self,
        viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(
            DrawCommand::Clear {
                color: self
                    .theme
                    .colors
                    .background,
            },
        );

        let rectangle_node =
            LayoutNode::new(
                Style {
                    display: Display::Block,

                    size: SizeStyle {
                        width: Length::Px(
                            280.0,
                        ),
                        height: Length::Px(
                            160.0,
                        ),
                        ..Default::default()
                    },

                    ..Default::default()
                },
            );

        let mut root =
            LayoutNode::with_children(
                Style {
                    display: Display::Flex {
                        flex_direction:
                        FlexDirection::Column,
                    },

                    size: SizeStyle {
                        width: Length::Px(
                            viewport
                                .logical_size
                                .width,
                        ),
                        height: Length::Px(
                            viewport
                                .logical_size
                                .height,
                        ),
                        ..Default::default()
                    },

                    justify_content:
                    JustifyContent::Center,

                    align_items:
                    AlignItems::Center,

                    ..Default::default()
                },
                vec![
                    rectangle_node,
                ],
            );

        LayoutEngine::layout(
            &mut root,
            viewport.logical_size.width,
            viewport.logical_size.height,
        );

        let Some(rectangle_bounds) =
            border_box(
                &root.children[0],
                Point::new(0.0, 0.0),
            )
        else {
            return;
        };

        let mut context = PaintContext {
            display_list,
            theme: &self.theme,
            typography: &self.typography,
        };

        let rectangle =
            Rectangle::new();

        rectangle.paint(
            rectangle_bounds,
            &mut context,
        );
    }
}

fn main(
) -> Result<(), Box<dyn std::error::Error>> {
    let application =
        ExampleApplication::new();

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from(
                "ViewKit Layout Example",
            ),

            size: Size::new(
                720.0,
                520.0,
            ),

            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}