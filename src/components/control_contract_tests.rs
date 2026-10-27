use super::*;
use crate::geometry::Size;
use crate::state::State;
use crate::theme::{ControlSize, Theme};
use crate::typography::TextMeasurer;
use crate::view::{Constraints, MeasureContext, View};

#[test]
fn controls_share_size_categories_and_respect_theme_metrics() {
    for mut theme in [Theme::LIGHT, Theme::DARK] {
        theme.controls.small_height = 25.0;
        theme.controls.height = 31.0;
        theme.controls.large_height = 35.0;
        let mut text = TextMeasurer::new();
        let mut context = MeasureContext {
            theme: &theme,
            typography: &theme.typography,
            text_measurer: &mut text,
        };
        for (size, button, field) in [
            (ControlSize::Small, ButtonSize::Small, TextFieldSize::Small),
            (
                ControlSize::Medium,
                ButtonSize::Medium,
                TextFieldSize::Medium,
            ),
            (ControlSize::Large, ButtonSize::Large, TextFieldSize::Large),
        ] {
            let views: Vec<Box<dyn View>> = vec![
                Box::new(Button::new("Apply").size(button)),
                Box::new(TextField::new(State::new("value".into()).binding()).size(field)),
                Box::new(Picker::new("Tokyo").size(size)),
                Box::new(Stepper::new(State::new(3).binding()).size(size)),
            ];
            for view in views {
                assert_eq!(
                    view.measure(Constraints::loose(Size::new(300.0, 100.0)), &mut context)
                        .height,
                    size.height(&theme.controls)
                );
                assert_eq!(
                    view.measure(
                        Constraints::new(Size::new(120.0, 22.0), Size::new(120.0, 22.0)),
                        &mut context
                    )
                    .height,
                    22.0
                );
            }
        }
    }
}

