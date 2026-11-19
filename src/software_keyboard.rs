//! Presentation of the layout resources supplied by installed keyboard apps.

use crate::accessibility::{AccessibilityNode, AccessibilityRole};
use crate::event::ViewEvent;
use crate::platform::{PlatformEvent, PointerButton, TouchPhase};
use crate::prelude::*;
use crate::view::PaintContext;
use std::{fs, io, io::Read, path::PathBuf};

fn preference_path() -> Option<PathBuf> {
    std::env::var_os("MOCHIOS_PREVIEW_CONFIG_ROOT")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))
        .map(|dir| dir.join("mochios/keyboard-layout"))
}

/// 現在のキーボード配列を、提供元のアプリIDを含む識別子で読み込みます。
pub fn preferred_layout() -> Option<String> {
    let mut id = String::new();
    fs::File::open(preference_path()?)
        .ok()?
        .take(513)
        .read_to_string(&mut id)
        .ok()?;
    (id.len() <= 512).then_some(id)
}

/// 選択したキーボード配列を、次の入力セッションでも使えるように保存します。
pub fn save_preferred_layout(id: &str) -> io::Result<()> {
    if id.is_empty() || id.len() > 512 || id.chars().any(char::is_control) {
        return Err(io::ErrorKind::InvalidInput.into());
    }
    let path = preference_path().ok_or(io::ErrorKind::NotFound)?;
    fs::create_dir_all(path.parent().ok_or(io::ErrorKind::NotFound)?)?;
    let pending = path.with_extension(format!("{}.new", std::process::id()));
    fs::write(&pending, id)?;
    fs::rename(pending, path)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyboardAction {
    Key(char),
    ToggleIme,
    Dismiss,
}

/// ソフトウェアキーを、通常の入力欄が処理するプラットフォームイベントに変換します。
pub fn software_key_event(key: char) -> PlatformEvent {
    match key {
        '\u{8}' => PlatformEvent::Backspace,
        '\n' | '\t' => PlatformEvent::KeyPressed {
            key: if key == '\n' {
                crate::platform::Key::Enter
            } else {
                crate::platform::Key::Tab
            },
            modifiers: crate::platform::KeyModifiers::default(),
        },
        key => PlatformEvent::TextInput {
            text: key.to_string(),
        },
    }
}

#[derive(Clone, Copy)]
enum Key {
    Text(char, char),
    Flick([char; 5]),
    Shift,
    Space,
    Return,
    Erase,
    Ime,
    Dismiss,
    Layout,
}

pub struct KeyboardLayout {
    pub name: String,
    rows: Vec<Vec<Key>>,
    pub kana: bool,
}

impl KeyboardLayout {
    pub fn preferred_width(&self, available: f32, theme: &Theme) -> f32 {
        let units = if self.kana { 8.5 } else { 36.0 };
        available.min(theme.controls.touch_height * units)
    }

    pub fn preferred_height(&self, width: f32, theme: &Theme) -> f32 {
        let touch = theme.controls.touch_height;
        let key = (width / if self.kana { 7.0 } else { 22.0 }).clamp(touch, touch * 1.35);
        let rows = self.rows.len() as f32;
        rows * key + (rows - 1.0) * theme.spacing.extra_small + 2.0 * theme.spacing.medium
    }
}

/// The control tokens begin with '@'. Flick keys list center, left, up,
/// right and down. A dash in a flick slot repeats the center.
pub fn parse_layouts(text: &str) -> Option<Vec<KeyboardLayout>> {
    if text.len() > 16 * 1024 {
        return None;
    }
    let mut layouts: Vec<KeyboardLayout> = Vec::new();
    for line in text.lines().map(str::trim) {
        if line.is_empty() {
            continue;
        }
        if let Some(title) = line
            .strip_prefix('[')
            .and_then(|line| line.strip_suffix(']'))
        {
            if title.is_empty() || title.chars().count() > 24 || layouts.len() >= 8 {
                return None;
            }
            layouts.push(KeyboardLayout {
                name: title.into(),
                rows: Vec::new(),
                kana: false,
            });
            continue;
        }
        let layout = layouts.last_mut()?;
        let row = line
            .split_ascii_whitespace()
            .map(parse_key)
            .collect::<Option<Vec<_>>>()?;
        if row.is_empty() || row.len() > 16 || layout.rows.len() >= 6 {
            return None;
        }
        layout.kana |= row.iter().any(|key| matches!(key, Key::Flick(_)));
        layout.rows.push(row);
    }
    (!layouts.is_empty() && layouts.iter().all(|layout| !layout.rows.is_empty())).then_some(layouts)
}

fn character(value: &str) -> Option<char> {
    let key = if let Some(hex) = value.strip_prefix("U+") {
        char::from_u32(u32::from_str_radix(hex, 16).ok()?)?
    } else {
        let mut chars = value.chars();
        let key = chars.next()?;
        if chars.next().is_some() {
            return None;
        }
        key
    };
    (!key.is_control()).then_some(key)
}

fn parse_key(token: &str) -> Option<Key> {
    Some(match token {
        "@layout" => Key::Layout,
        "@shift" => Key::Shift,
        "@space" => Key::Space,
        "@return" => Key::Return,
        "@erase" => Key::Erase,
        "@ime" => Key::Ime,
        "@hide" => Key::Dismiss,
        _ if token.starts_with('+') && token.len() > 1 => {
            let mut chars = token[1..].chars();
            let center = chars.next()?;
            let mut choices = [center; 5];
            for choice in &mut choices[1..] {
                let value = chars.next()?;
                if value != '-' {
                    *choice = value;
                }
            }
            if chars.next().is_some() || choices.iter().any(|choice| choice.is_control()) {
                return None;
            }
            Key::Flick(choices)
        }
        _ => {
            if let Some((normal, shifted)) = token.split_once('|') {
                Key::Text(character(normal)?, character(shifted)?)
            } else {
                let key = character(token)?;
                Key::Text(key, key.to_ascii_uppercase())
            }
        }
    })
}

impl Key {
    fn weight(self) -> f32 {
        match self {
            Self::Space => 4.5,
            Self::Return | Self::Erase | Self::Shift | Self::Layout => 1.5,
            _ => 1.0,
        }
    }
}

#[derive(Clone, Copy)]
struct Press {
    id: u64,
    index: usize,
    origin: Point,
    point: Point,
}

#[derive(Default)]
pub struct SoftwareKeyboard {
    pub selected: usize,
    shifted: bool,
    press: Option<Press>,
}

impl SoftwareKeyboard {
    pub fn cancel(&mut self) {
        self.press = None;
        self.shifted = false;
    }

    pub fn height(layouts: &[KeyboardLayout], theme: &Theme) -> f32 {
        let rows = layouts
            .iter()
            .map(|layout| layout.rows.len())
            .max()
            .unwrap_or(0);
        if rows == 0 {
            return 0.0;
        }
        let gap = theme.spacing.extra_small;
        rows as f32 * (theme.controls.touch_height + gap) + gap
    }

    fn keys(&self, layouts: &[KeyboardLayout], bounds: Rect, theme: &Theme) -> Vec<(Key, Rect)> {
        let Some(layout) = layouts.get(self.selected) else {
            return Vec::new();
        };
        let width = layout.preferred_width(bounds.size.width, theme);
        let gap = theme.spacing.extra_small.min(width / 70.0);
        let inset = theme.spacing.medium.min(width / 28.0);
        let x = bounds.origin.x + (bounds.size.width - width) / 2.0 + inset;
        let vertical_inset = theme.spacing.medium.min(bounds.size.height / 8.0);
        let rows = layout.rows.len() as f32;
        let height = (bounds.size.height - 2.0 * vertical_inset + gap) / rows;
        let mut keys = Vec::new();
        for (row_index, row) in layout.rows.iter().enumerate() {
            let weight = |key: Key| if layout.kana { 1.0 } else { key.weight() };
            let total = row.iter().copied().map(weight).sum::<f32>();
            let unit = (width - 2.0 * inset - (row.len() - 1) as f32 * gap).max(0.0) / total;
            let mut left = x;
            for key in row.iter().copied() {
                let width = unit * weight(key);
                keys.push((
                    key,
                    Rect::new(
                        left,
                        bounds.origin.y + vertical_inset + row_index as f32 * height,
                        width,
                        (height - gap).max(0.0),
                    ),
                ));
                left += width + gap;
            }
        }
        keys
    }

    pub fn paint(
        &self,
        layouts: &[KeyboardLayout],
        bounds: Rect,
        japanese: bool,
        context: &mut PaintContext<'_>,
    ) {
        self.paint_with_footer(layouts, bounds, 0.0, japanese, context);
    }

    /// Paints a docked keyboard tray through the navigation area. Keys occupy
    /// only `bounds`, so pointer contacts that start in the footer remain free.
    pub fn paint_with_footer(
        &self,
        layouts: &[KeyboardLayout],
        bounds: Rect,
        footer: f32,
        japanese: bool,
        context: &mut PaintContext<'_>,
    ) {
        let Some(layout) = layouts.get(self.selected) else {
            return;
        };
        let colors = context.theme.colors;
        let tray = colors.text_primary.with_alpha(36).over(colors.surface);
        let modifier = colors.text_primary.with_alpha(16).over(tray);
        let background = Rectangle::new().color(RectangleColor::Custom(tray));
        if footer > 0.0 {
            background.paint(
                Rect::new(
                    bounds.origin.x,
                    bounds.origin.y,
                    bounds.size.width,
                    bounds.size.height + footer,
                ),
                context,
            );
        } else {
            background
                .radius(CornerRadius::ExtraLarge)
                .shadow(ShadowStyle::Card)
                .paint(bounds, context);
        }
        for (index, (key, rect)) in self
            .keys(layouts, bounds, context.theme)
            .into_iter()
            .enumerate()
        {
            let pressed = self.press.filter(|press| press.index == index);
            let label = match key {
                Key::Text(normal, shifted) => {
                    if self.shifted { shifted } else { normal }.to_string()
                }
                Key::Flick(choices) => {
                    choices[pressed.map_or(0, |press| flick(press, rect))].to_string()
                }
                Key::Shift => if self.shifted { "⬆" } else { "⇧" }.into(),
                Key::Space => if japanese { "変換" } else { "Space" }.into(),
                Key::Return => "↵".into(),
                Key::Erase => "⌫".into(),
                Key::Ime => if japanese { "あ" } else { "A" }.into(),
                Key::Dismiss => "Hide keyboard".into(),
                Key::Layout => layout.name.clone(),
            };
            let regular = matches!(key, Key::Text(..) | Key::Flick(..) | Key::Space);
            let fill = if regular { colors.surface } else { modifier };
            let fill = if pressed.is_some() || (matches!(key, Key::Shift) && self.shifted) {
                colors.text_primary.with_alpha(48).over(fill)
            } else {
                fill
            };
            Rectangle::new()
                .radius(CornerRadius::Medium)
                .color(RectangleColor::Custom(fill))
                .paint(rect, context);
            if matches!(key, Key::Dismiss) {
                Icon::new(SymbolName::ChevronDown)
                    .size(context.theme.typography.body.size)
                    .color(colors.text_primary)
                    .paint(rect, context);
            } else {
                let role = match key {
                    Key::Text(..) | Key::Flick(_) => TextRole::TitleMedium,
                    Key::Layout | Key::Space => TextRole::Label,
                    _ => TextRole::TitleSmall,
                };
                let line_height = context.theme.typography.style(role).line_height;
                let text_rect = Rect::new(
                    rect.origin.x,
                    rect.origin.y + (rect.size.height - line_height).max(0.0) * 0.5,
                    rect.size.width,
                    line_height,
                );
                context
                    .display_list
                    .push(crate::draw_command::DrawCommand::PushClip { rect });
                Text::styled(&label, role)
                    .alignment(TextAlignment::Center)
                    .paint(text_rect, context);
                context
                    .display_list
                    .push(crate::draw_command::DrawCommand::PopClip);
            }
            let mut node = AccessibilityNode::new(AccessibilityRole::Button, rect);
            node.label = Some(label);
            context.record_accessibility(node);
        }
    }

    /// Returns whether this contact belongs to the keyboard and its action,
    /// if the primary pointer or touch has just released a key.
    pub fn handle(
        &mut self,
        layouts: &[KeyboardLayout],
        bounds: Rect,
        event: &ViewEvent,
        theme: &Theme,
    ) -> (bool, Option<KeyboardAction>) {
        let (id, phase, point) = match *event {
            ViewEvent::PointerPressed {
                position,
                button: PointerButton::Primary,
            } => (u64::MAX, TouchPhase::Started, position),
            ViewEvent::PointerMoved { position } => (u64::MAX, TouchPhase::Moved, position),
            ViewEvent::PointerReleased {
                position,
                button: PointerButton::Primary,
            } => (u64::MAX, TouchPhase::Ended, position),
            ViewEvent::Touch {
                id,
                phase,
                position,
            } => (id, phase, position),
            ViewEvent::PointerLeft | ViewEvent::FocusChanged { focused: false } => {
                return (self.press.take().is_some(), None);
            }
            _ => return (false, None),
        };
        if phase == TouchPhase::Started {
            if !bounds.contains(point) {
                return (false, None);
            }
            if self.press.is_none() {
                self.press = self
                    .keys(layouts, bounds, theme)
                    .iter()
                    .position(|(_, rect)| rect.contains(point))
                    .map(|index| Press {
                        id,
                        index,
                        origin: point,
                        point,
                    });
            }
            return (true, None);
        }
        let Some(press) = self.press.as_mut().filter(|press| press.id == id) else {
            return (bounds.contains(point), None);
        };
        press.point = point;
        if phase == TouchPhase::Moved {
            return (true, None);
        }
        let press = self.press.take().unwrap();
        if phase == TouchPhase::Cancelled {
            return (true, None);
        }
        let keys = self.keys(layouts, bounds, theme);
        let Some(&(key, rect)) = keys.get(press.index) else {
            return (true, None);
        };
        let action = match key {
            Key::Flick(choices) => Some(KeyboardAction::Key(choices[flick(press, rect)])),
            _ if !rect.contains(point) => None,
            Key::Shift => {
                self.shifted = !self.shifted;
                None
            }
            Key::Layout => {
                self.selected = (self.selected + 1) % layouts.len();
                self.shifted = false;
                None
            }
            Key::Ime => Some(KeyboardAction::ToggleIme),
            Key::Dismiss => Some(KeyboardAction::Dismiss),
            Key::Erase => Some(KeyboardAction::Key('\u{8}')),
            Key::Space => Some(KeyboardAction::Key(' ')),
            Key::Return => Some(KeyboardAction::Key('\n')),
            Key::Text(normal, shifted) => {
                Some(KeyboardAction::Key(if std::mem::take(&mut self.shifted) {
                    shifted
                } else {
                    normal
                }))
            }
        };
        (true, action)
    }
}

fn flick(press: Press, key: Rect) -> usize {
    let dx = press.point.x - press.origin.x;
    let dy = press.point.y - press.origin.y;
    let slop = (key.size.height * 0.23).max(8.0);
    if dx.hypot(dy) < slop {
        0
    } else if dx.abs() > dy.abs() {
        if dx < 0.0 { 1 } else { 3 }
    } else if dy < 0.0 {
        2
    } else {
        4
    }
}
