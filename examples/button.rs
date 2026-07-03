use viewkit::components::{Button, ButtonInteractionState, ButtonStyle, Text, VStack};
use viewkit::draw_command::{DisplayList, DrawCommand};
use viewkit::event::{EventContext, EventDispatcher};
use viewkit::geometry::Size;
use viewkit::layout::{StackAlignment, StackDistribution, StackGap, ViewExt};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use viewkit::renderer::Viewport;
use viewkit::theme::Theme;
use viewkit::typography::{TextAlignment, TextMeasurer, Typography};
use viewkit::view::{PaintContext, View};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher: EventDispatcher,

    standard_button_state: ButtonInteractionState,

    primary_button_state: ButtonInteractionState,

    accent_button_state: ButtonInteractionState,

    ghost_button_state: ButtonInteractionState,

    danger_button_state: ButtonInteractionState,

    disabled_button_state: ButtonInteractionState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),

            standard_button_state: ButtonInteractionState::new(),

            primary_button_state: ButtonInteractionState::new(),

            accent_button_state: ButtonInteractionState::new(),

            ghost_button_state: ButtonInteractionState::new(),

            danger_button_state: ButtonInteractionState::new(),

            disabled_button_state: ButtonInteractionState::new(),
        }
    }

    fn make_button(
        &self,
        interaction: ButtonInteractionState,
        style: ButtonStyle,
        title: &'static str,
    ) -> Button {
        Button::new(interaction).style(style).content(
            Text::new(title)
                .font_size(14.0)
                .line_height(20.0)
                .weight(600)
                .alignment(TextAlignment::Center)
                .color(style.foreground_color(&self.theme))
                .frame(180.0, 20.0),
        )
    }

    fn build_root(&self) -> VStack {
        let standard_button = self.make_button(
            self.standard_button_state.clone(),
            ButtonStyle::Standard,
            "Standard",
        );

        let primary_button = self.make_button(
            self.primary_button_state.clone(),
            ButtonStyle::Primary,
            "Primary",
        );

        let accent_button = self.make_button(
            self.accent_button_state.clone(),
            ButtonStyle::Accent,
            "Accent",
        );

        let ghost_button =
            self.make_button(self.ghost_button_state.clone(), ButtonStyle::Ghost, "Ghost");

        let danger_button = self.make_button(
            self.danger_button_state.clone(),
            ButtonStyle::Danger,
            "Danger",
        );

        let disabled_button = self
            .make_button(
                self.disabled_button_state.clone(),
                ButtonStyle::Primary,
                "Disabled",
            )
            .enabled(false);

        VStack::new()
            .gap(StackGap::Medium)
            .alignment(StackAlignment::Center)
            .distribution(StackDistribution::Center)
            .child(standard_button.frame(240.0, 44.0))
            .child(primary_button.frame(240.0, 44.0))
            .child(accent_button.frame(240.0, 44.0))
            .child(ghost_button.frame(240.0, 44.0))
            .child(danger_button.frame(240.0, 44.0))
            .child(disabled_button.frame(240.0, 44.0))
    }

    fn handle_button_clicks(&self) {
        if self.standard_button_state.take_clicked() {
            println!("Standardボタンがクリックされました");
        }

        if self.primary_button_state.take_clicked() {
            println!("Primaryボタンがクリックされました");
        }

        if self.accent_button_state.take_clicked() {
            println!("Accentボタンがクリックされました");
        }

        if self.ghost_button_state.take_clicked() {
            println!("Ghostボタンがクリックされました");
        }

        if self.danger_button_state.take_clicked() {
            println!("Dangerボタンがクリックされました");
        }

        /*
         * disabled_button_stateは
         * enabled(false)なので、
         * clickedにはなりません。
         */
    }
}

impl PlatformApplication for ExampleApplication {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        let root = self.build_root();

        let redraw_requested = {
            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher.dispatch(
                &root,
                window.viewport().logical_bounds(),
                &event,
                &mut context,
            );

            context.redraw_requested()
        };

        self.handle_button_clicks();

        if redraw_requested {
            window.request_redraw();
        }

        match event {
            PlatformEvent::Resumed { viewport } => {
                println!("resumed: {viewport:?}");
            }

            PlatformEvent::Resized { viewport } => {
                println!("resized: {viewport:?}");
            }

            PlatformEvent::ScaleFactorChanged { viewport } => {
                println!("scale factor changed: {viewport:?}");
            }

            PlatformEvent::Focused(focused) => {
                println!("focused: {focused}");
            }

            PlatformEvent::CloseRequested => {
                println!("close requested");
            }

            PlatformEvent::PointerMoved { .. }
            | PlatformEvent::PointerButton { .. }
            | PlatformEvent::PointerLeft
            | PlatformEvent::Scroll { .. }
            | PlatformEvent::RedrawRequested => {}

            _ => {}
        }
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        let root = self.build_root();

        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.typography,
            &mut self.text_measurer,
        );

        root.paint(viewport.logical_bounds(), &mut context);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let application = ExampleApplication::new();

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from("ViewKit Button Example"),

            size: Size::new(720.0, 520.0),

            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}
