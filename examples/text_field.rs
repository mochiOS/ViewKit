use viewkit::components::{
    TextField,
    TextFieldInteractionState,
    TextFieldSize,
    VStack,
};
use viewkit::draw_command::{
    DisplayList,
    DrawCommand,
};
use viewkit::event::{
    EventContext,
    EventDispatcher,
};
use viewkit::geometry::Size;
use viewkit::layout::{
    StackAlignment,
    StackDistribution,
    StackGap,
    ViewExt,
};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{
    PlatformApplication,
    PlatformEvent,
    PlatformWindow,
    WindowConfig,
};
use viewkit::renderer::Viewport;
use viewkit::theme::Theme;
use viewkit::typography::{
    TextMeasurer,
    Typography,
};
use viewkit::view::{
    PaintContext,
    View,
};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,
    event_dispatcher: EventDispatcher,

    empty_state:
        TextFieldInteractionState,

    value_state:
        TextFieldInteractionState,

    invalid_state:
        TextFieldInteractionState,

    disabled_state:
        TextFieldInteractionState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography:
            Typography::DEFAULT,

            text_measurer:
            TextMeasurer::new(),

            event_dispatcher:
            EventDispatcher::new(),

            empty_state:
            TextFieldInteractionState::new(),

            value_state:
            TextFieldInteractionState::new(),

            invalid_state:
            TextFieldInteractionState::new(),

            disabled_state:
            TextFieldInteractionState::new(),
        }
    }

    fn build_root(&self) -> VStack {
        let empty =
            TextField::new(
                self.empty_state.clone(),
            )
                .placeholder(
                    "名前を入力",
                );

        let value =
            TextField::new(
                self.value_state.clone(),
            )
                .value(
                    "mochiOS",
                );

        let invalid =
            TextField::new(
                self.invalid_state.clone(),
            )
                .value(
                    "invalid@example",
                )
                .invalid(
                    true,
                );

        let disabled =
            TextField::new(
                self.disabled_state.clone(),
            )
                .placeholder(
                    "使用できません",
                )
                .size(
                    TextFieldSize::Large,
                )
                .enabled(
                    false,
                );

        VStack::new()
            .gap(
                StackGap::Large,
            )
            .alignment(
                StackAlignment::Center,
            )
            .distribution(
                StackDistribution::Center,
            )
            .child(
                empty.frame(
                    320.0,
                    36.0,
                ),
            )
            .child(
                value.frame(
                    320.0,
                    36.0,
                ),
            )
            .child(
                invalid.frame(
                    320.0,
                    36.0,
                ),
            )
            .child(
                disabled.frame(
                    320.0,
                    44.0,
                ),
            )
    }
}

impl PlatformApplication
for ExampleApplication
{
    fn handle_event(
        &mut self,
        event: PlatformEvent,
        window: &dyn PlatformWindow,
    ) {
        let root =
            self.build_root();

        let redraw_requested = {
            let mut context =
                EventContext::new(
                    &self.theme,
                    &self.typography,
                    &mut self.text_measurer,
                );

            self.event_dispatcher.dispatch(
                &root,
                window
                    .viewport()
                    .logical_bounds(),
                &event,
                &mut context,
            );

            context.redraw_requested()
        };

        if redraw_requested {
            window.request_redraw();
        }
    }

    fn draw(
        &mut self,
        viewport: Viewport,
        display_list: &mut DisplayList,
    ) {
        display_list.push(
            DrawCommand::Clear {
                color:
                self.theme
                    .colors
                    .background,
            },
        );

        let root =
            self.build_root();

        let mut context =
            PaintContext {
                display_list,
                theme: &self.theme,
                typography:
                &self.typography,
                text_measurer:
                &mut self.text_measurer,
            };

        root.paint(
            viewport.logical_bounds(),
            &mut context,
        );
    }
}

fn main(
) -> Result<
    (),
    Box<dyn std::error::Error>,
> {
    let application =
        ExampleApplication::new();

    let backend =
        LinuxBackend::new(
            application,
            WindowConfig {
                title: String::from(
                    "ViewKit TextField Example",
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