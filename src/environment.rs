//! ウィンドウ単位の入力方式と、論理座標での表示可能領域。
//! アプリが明示的に利用します。既存コンポーネントの配置は自動変換しません。
use crate::{
    edge_insets::EdgeInsets,
    geometry::{Rect, Size},
    theme::{ControlSize, Theme},
};

/// ランチャーから渡す表示上の希望値。権限や入力の真正性を表しません。
pub const INPUT_MODE_ENV: &str = "VIEWKIT_INPUT_MODE";

/// 操作面の寸法を選ぶための入力方式。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub enum InputMode {
    /// 従来のマウス・トラックパッド向けの寸法。
    #[default]
    Pointer,
    /// 指で操作できる寸法。描画倍率とは独立しています。
    Touch,
}
impl InputMode {
    /// ランチャーへ渡せる安定した表記を返します。
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pointer => "pointer",
            Self::Touch => "touch",
        }
    }
    pub(crate) fn process_preference() -> Self {
        match std::env::var(INPUT_MODE_ENV).as_deref() {
            Ok("touch") => Self::Touch,
            _ => Self::Pointer,
        }
    }
}

/// プラットフォームがクライアント領域の論理座標で通知する情報。
/// 未通知の安全領域はゼロ、キーボードは非表示として扱います。
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(target_os = "linux", derive(serde::Serialize, serde::Deserialize))]
pub struct WindowEnvironment {
    pub input_mode: InputMode,
    pub safe_area: EdgeInsets,
    pub keyboard_occlusion: Option<Rect>,
}
impl WindowEnvironment {
    /// 安全領域を除いた矩形。過大・不正なinsetでも負の寸法を返しません。
    pub fn safe_bounds(self, bounds: Rect) -> Rect {
        let inset = EdgeInsets::new(
            self.safe_area.top,
            self.safe_area.right,
            self.safe_area.bottom,
            self.safe_area.left,
        );
        let width = bounds.size.width.max(0.0);
        let height = bounds.size.height.max(0.0);
        let left = inset.left.min(width);
        let top = inset.top.min(height);
        Rect::new(
            bounds.origin.x + left,
            bounds.origin.y + top,
            (width - left - inset.right).max(0.0),
            (height - top - inset.bottom).max(0.0),
        )
    }

    /// 下端を覆うキーボードと安全領域を除いた内容領域。
    /// フローティングキーボードでは画面全体を縮めず、keyboard_overlapを利用します。
    pub fn content_bounds(self, bounds: Rect) -> Rect {
        let mut content = self.safe_bounds(bounds);
        if let Some(keyboard) = self.keyboard_overlap(content) {
            if keyboard.origin.x <= content.origin.x
                && keyboard.origin.x + keyboard.size.width >= content.origin.x + content.size.width
                && keyboard.origin.y + keyboard.size.height
                    >= content.origin.y + content.size.height
            {
                content.size.height = (keyboard.origin.y - content.origin.y).max(0.0);
            }
        }
        content
    }

    /// 指定領域とキーボードの重なり。浮遊型のキーボードにも対応します。
    pub fn keyboard_overlap(self, bounds: Rect) -> Option<Rect> {
        let keyboard = self.keyboard_occlusion?;
        if ![
            keyboard.origin.x,
            keyboard.origin.y,
            keyboard.size.width,
            keyboard.size.height,
        ]
        .into_iter()
        .all(f32::is_finite)
            || keyboard.size.width <= 0.0
            || keyboard.size.height <= 0.0
        {
            return None;
        }
        keyboard.intersection(bounds)
    }
}

/// アプリが入力方式に応じて選択できる共通寸法。すべて論理座標単位です。
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct InteractionMetrics {
    pub control_size: ControlSize,
    pub control_height: f32,
    pub toolbar_height: f32,
    pub row_height: f32,
    pub sidebar_row_height: f32,
    pub grid_cell: Size,
}
impl InteractionMetrics {
    /// テーマの既存値を基準に、タッチ時だけ最小操作寸法を確保します。
    pub fn new(theme: &Theme, input: InputMode) -> Self {
        let touch = input == InputMode::Touch;
        let control_size = if touch {
            ControlSize::Touch
        } else {
            ControlSize::Medium
        };
        let height = control_size.height(&theme.controls);
        Self {
            control_size,
            control_height: height,
            toolbar_height: if touch {
                theme
                    .layout
                    .top_bar_height
                    .max(height + theme.spacing.small * 2.0)
            } else {
                theme.layout.top_bar_height
            },
            row_height: if touch {
                height + theme.spacing.extra_small
            } else {
                theme.layout.control_height
            },
            sidebar_row_height: if touch {
                height + theme.spacing.extra_small
            } else {
                theme.controls.height + theme.spacing.extra_small
            },
            grid_cell: Size::new(
                theme
                    .layout
                    .browser_grid_cell_width
                    .max(if touch { height * 3.0 } else { 0.0 }),
                theme
                    .layout
                    .browser_grid_cell_height
                    .max(if touch { height * 3.0 } else { 0.0 }),
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn docked_keyboard_overlaps_safe_area_instead_of_adding_it() {
        let environment = WindowEnvironment {
            safe_area: EdgeInsets::new(24., 0., 20., 0.),
            keyboard_occlusion: Some(Rect::new(0., 500., 800., 300.)),
            ..Default::default()
        };
        assert_eq!(
            environment.content_bounds(Rect::new(0., 0., 800., 800.)),
            Rect::new(0., 24., 800., 476.)
        );
        let floating = WindowEnvironment {
            keyboard_occlusion: Some(Rect::new(100., 400., 300., 200.)),
            ..environment
        };
        assert_eq!(
            floating.content_bounds(Rect::new(0., 0., 800., 800.)),
            floating.safe_bounds(Rect::new(0., 0., 800., 800.))
        );
        assert!(
            floating
                .keyboard_overlap(Rect::new(0., 0., 800., 800.))
                .is_some()
        );
    }
    #[test]
    fn invalid_and_oversized_insets_cannot_make_negative_content() {
        let environment = WindowEnvironment {
            safe_area: EdgeInsets {
                top: f32::NAN,
                right: 900.,
                bottom: 900.,
                left: -1.,
            },
            ..Default::default()
        };
        assert_eq!(
            environment.safe_bounds(Rect::new(0., 0., 100., 100.)),
            Rect::new(0., 0., 0., 0.)
        );
    }
    #[test]
    fn pointer_defaults_stay_unchanged_and_touch_targets_have_a_shared_minimum() {
        let desktop = InteractionMetrics::new(&Theme::LIGHT, InputMode::Pointer);
        let touch = InteractionMetrics::new(&Theme::LIGHT, InputMode::Touch);
        assert_eq!(desktop.control_height, Theme::LIGHT.controls.height);
        assert_eq!(desktop.toolbar_height, Theme::LIGHT.layout.top_bar_height);
        assert!(touch.control_height >= 44. && touch.row_height >= touch.control_height);
    }
}
