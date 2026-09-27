use crate::platform::input_method;
use crate::{
    components::{BorderStyle, Rectangle, RectangleColor, Text},
    geometry::Rect,
    theme::{CornerRadius, ShadowStyle},
    typography::TextRole,
    view::{PaintContext, View},
};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};

static JAPANESE_INPUT_ENABLED: AtomicBool = AtomicBool::new(false);
const CANDIDATES_PER_PAGE: usize = 8;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InputMethodState {
    raw: String,
    marked: Option<Range<usize>>,
    candidates: Vec<String>,
    selected: usize,
    suppress: Option<char>,
}

impl InputMethodState {
    pub(crate) fn enabled() -> bool {
        if let Some(enabled) = input_method::enabled() {
            JAPANESE_INPUT_ENABLED.store(enabled, Ordering::Relaxed);
        }
        JAPANESE_INPUT_ENABLED.load(Ordering::Relaxed)
    }

    pub(crate) fn toggle(&mut self) {
        let enabled = input_method::toggle()
            .unwrap_or_else(|| !JAPANESE_INPUT_ENABLED.load(Ordering::Relaxed));
        JAPANESE_INPUT_ENABLED.store(enabled, Ordering::Relaxed);
    }

    pub(crate) fn synchronize(&mut self) {
        if let Some(enabled) = input_method::enabled() {
            JAPANESE_INPUT_ENABLED.store(enabled, Ordering::Relaxed);
        }
    }

    pub(crate) fn candidates(&self) -> (&[String], usize) {
        (&self.candidates, self.selected)
    }

    pub(crate) fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub(crate) fn is_converting(&self) -> bool {
        !self.candidates.is_empty()
    }

    pub(crate) fn handle_text(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
        selection: Option<Range<usize>>,
        text: &str,
    ) -> bool {
        if let Some(suppressed) = self.suppress.take()
            && text.chars().eq([suppressed])
        {
            return true;
        }
        // mochiOS delivers both a key event and a text-input event for Space.
        // Conversion belongs to the text-input path so it cannot be missed by
        // platform key mapping and so the same event is not inserted as text.
        if text == " " && self.marked.is_some() {
            return if self.candidates.is_empty() {
                self.convert(value, cursor)
            } else {
                self.select_next(value, cursor, 1)
            };
        }
        if !self.candidates.is_empty()
            && let Some(digit) = text.chars().next()
            && text.len() == 1
            && ('1'..='8').contains(&digit)
        {
            let page = self.selected / CANDIDATES_PER_PAGE;
            let index = page * CANDIDATES_PER_PAGE + digit as usize - '1' as usize;
            if let Some(candidate) = self.candidates.get(index).cloned() {
                self.replace_marked(value, cursor, candidate);
                self.clear();
                return true;
            }
        }
        if !Self::enabled() {
            if self.marked.is_some() {
                self.clear();
            }
            return false;
        }
        if let Some(replacement) = japanese_punctuation(text) {
            if self.marked.is_some() {
                self.clear();
            }
            if let Some(range) = selection {
                value.replace_range(range.clone(), "");
                *cursor = range.start;
            }
            value.insert_str(*cursor, replacement);
            *cursor += replacement.len();
            return true;
        }
        if !text
            .chars()
            .all(|character| character.is_ascii_alphabetic())
        {
            if self.marked.is_some() {
                self.clear();
            }
            return false;
        }
        if self.marked.is_none() {
            if let Some(range) = selection {
                value.replace_range(range.clone(), "");
                *cursor = range.start;
            }
            self.marked = Some(*cursor..*cursor);
        }
        self.candidates.clear();
        self.selected = 0;
        self.raw
            .extend(text.chars().map(|character| character.to_ascii_lowercase()));
        self.refresh(value, cursor, false);
        true
    }

    pub(crate) fn convert(&mut self, value: &mut String, cursor: &mut usize) -> bool {
        if self.marked.is_none() {
            return false;
        }
        let (reading, pending) = roman_to_hiragana(&self.raw, true);
        if reading.is_empty() || !pending.is_empty() {
            return true;
        }
        self.candidates = input_method::candidates(&reading);
        if self.candidates.is_empty() {
            self.candidates.push(reading);
        }
        self.selected = 0;
        self.replace_marked(value, cursor, self.candidates[0].clone());
        true
    }

    pub(crate) fn select_next(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
        delta: isize,
    ) -> bool {
        if self.candidates.is_empty() {
            return false;
        }
        let count = self.candidates.len() as isize;
        self.selected = (self.selected as isize + delta).rem_euclid(count) as usize;
        self.replace_marked(value, cursor, self.candidates[self.selected].clone());
        true
    }

    pub(crate) fn commit(&mut self) -> bool {
        if self.marked.is_none() {
            return false;
        }
        self.clear();
        true
    }

    pub(crate) fn commit_before_text(&mut self, character: char) -> bool {
        if !self.commit() {
            return false;
        }
        self.suppress = Some(character);
        true
    }

    pub(crate) fn cancel(&mut self, value: &mut String, cursor: &mut usize) -> bool {
        if !self.candidates.is_empty() {
            self.candidates.clear();
            self.selected = 0;
            self.refresh(value, cursor, true);
            return true;
        }
        let Some(range) = self.marked.take() else {
            return false;
        };
        value.replace_range(range.clone(), "");
        *cursor = range.start;
        self.clear();
        true
    }

    pub(crate) fn backspace(&mut self, value: &mut String, cursor: &mut usize) -> bool {
        if self.marked.is_none() {
            return false;
        }
        if !self.candidates.is_empty() {
            self.candidates.clear();
            self.selected = 0;
        } else {
            self.raw.pop();
        }
        if self.raw.is_empty() {
            return self.cancel(value, cursor);
        }
        self.refresh(value, cursor, false);
        true
    }

    pub(crate) fn commit_on_blur(&mut self) {
        if self.marked.is_some() {
            self.clear();
        }
    }

    fn refresh(&mut self, value: &mut String, cursor: &mut usize, flush: bool) {
        let (reading, pending) = roman_to_hiragana(&self.raw, flush);
        self.replace_marked(value, cursor, format!("{reading}{pending}"));
    }

    fn replace_marked(&mut self, value: &mut String, cursor: &mut usize, replacement: String) {
        let Some(range) = self.marked.clone() else {
            return;
        };
        value.replace_range(range.clone(), &replacement);
        let end = range.start + replacement.len();
        self.marked = Some(range.start..end);
        *cursor = end;
    }

    fn clear(&mut self) {
        self.raw.clear();
        self.marked = None;
        self.candidates.clear();
        self.selected = 0;
    }
}

fn japanese_punctuation(text: &str) -> Option<&'static str> {
    Some(match text {
        " " => "　",
        "," => "、",
        "." => "。",
        "[" => "「",
        "]" => "」",
        "/" => "・",
        "-" => "ー",
        "!" => "！",
        "?" => "？",
        "0" => "０",
        "1" => "１",
        "2" => "２",
        "3" => "３",
        "4" => "４",
        "5" => "５",
        "6" => "６",
        "7" => "７",
        "8" => "８",
        "9" => "９",
        _ => return None,
    })
}

pub(crate) fn paint_candidates(
    anchor: Rect,
    candidates: &[String],
    selected: usize,
    context: &mut PaintContext<'_>,
) {
    if candidates.is_empty() {
        return;
    }
    let page_start = selected / CANDIDATES_PER_PAGE * CANDIDATES_PER_PAGE;
    let page_end = (page_start + CANDIDATES_PER_PAGE).min(candidates.len());
    let shown = page_end - page_start;
    let row_height = context.theme.layout.compact_control_height.max(26.0);
    let width = 260.0;
    let panel = Rect::new(
        anchor.origin.x,
        anchor.origin.y + anchor.size.height + context.theme.spacing.extra_small,
        width,
        row_height * shown as f32 + context.theme.spacing.extra_small * 2.0,
    );
    Rectangle::new()
        .color(RectangleColor::Custom(
            context.theme.colors.elevated_surface,
        ))
        .radius(CornerRadius::Small)
        .shadow(ShadowStyle::Floating)
        .border(BorderStyle::custom(context.theme.colors.border, 1.0))
        .paint(panel, context);
    for (row_index, candidate) in candidates[page_start..page_end].iter().enumerate() {
        let index = page_start + row_index;
        let row = Rect::new(
            panel.origin.x + context.theme.spacing.extra_small,
            panel.origin.y + context.theme.spacing.extra_small + row_height * row_index as f32,
            panel.size.width - context.theme.spacing.extra_small * 2.0,
            row_height,
        );
        if index == selected {
            Rectangle::new()
                .color(RectangleColor::Custom(context.theme.colors.accent_soft))
                .radius(CornerRadius::Small)
                .shadow(ShadowStyle::None)
                .border(BorderStyle::None)
                .paint(row, context);
        }
        Text::styled(format!("{}  {}", row_index + 1, candidate), TextRole::Label)
            .accessibility_hidden(true)
            .color(context.theme.colors.text_primary)
            .paint(
                Rect::new(
                    row.origin.x + context.theme.spacing.small,
                    row.origin.y,
                    row.size.width - context.theme.spacing.small * 2.0,
                    row.size.height,
                ),
                context,
            );
    }
}

fn roman_to_hiragana(raw: &str, flush: bool) -> (String, String) {
    let mut input = raw;
    let mut output = String::new();
    while !input.is_empty() {
        let bytes = input.as_bytes();
        if bytes.len() >= 2
            && bytes[0] == bytes[1]
            && matches!(
                bytes[0],
                b'b' | b'c'
                    | b'd'
                    | b'f'
                    | b'g'
                    | b'h'
                    | b'j'
                    | b'k'
                    | b'p'
                    | b'q'
                    | b's'
                    | b't'
                    | b'v'
                    | b'z'
            )
        {
            output.push('っ');
            input = &input[1..];
            continue;
        }
        if input.starts_with('n') && input.len() >= 2 {
            let next = input.as_bytes()[1];
            if next == b'n' || !matches!(next, b'a' | b'i' | b'u' | b'e' | b'o' | b'y') {
                output.push('ん');
                input = &input[1..];
                continue;
            }
        }
        let mut found = None;
        for length in (1..=input.len().min(4)).rev() {
            if !input.is_char_boundary(length) {
                continue;
            }
            if let Some(kana) = syllable(&input[..length]) {
                found = Some((length, kana));
                break;
            }
        }
        if let Some((length, kana)) = found {
            output.push_str(kana);
            input = &input[length..];
            continue;
        }
        if !flush && is_syllable_prefix(input) {
            break;
        }
        if flush && input == "n" {
            output.push('ん');
            input = &input[1..];
            continue;
        }
        break;
    }
    (output, input.to_owned())
}

fn is_syllable_prefix(value: &str) -> bool {
    ROMAJI.iter().any(|(roman, _)| roman.starts_with(value))
}

fn syllable(value: &str) -> Option<&'static str> {
    ROMAJI
        .iter()
        .find_map(|(roman, kana)| (*roman == value).then_some(*kana))
}

const ROMAJI: &[(&str, &str)] = &[
    ("kya", "きゃ"),
    ("kyu", "きゅ"),
    ("kyo", "きょ"),
    ("gya", "ぎゃ"),
    ("gyu", "ぎゅ"),
    ("gyo", "ぎょ"),
    ("sha", "しゃ"),
    ("shu", "しゅ"),
    ("sho", "しょ"),
    ("sya", "しゃ"),
    ("syu", "しゅ"),
    ("syo", "しょ"),
    ("ja", "じゃ"),
    ("ju", "じゅ"),
    ("jo", "じょ"),
    ("jya", "じゃ"),
    ("jyu", "じゅ"),
    ("jyo", "じょ"),
    ("cha", "ちゃ"),
    ("chu", "ちゅ"),
    ("cho", "ちょ"),
    ("cya", "ちゃ"),
    ("cyu", "ちゅ"),
    ("cyo", "ちょ"),
    ("tya", "ちゃ"),
    ("tyu", "ちゅ"),
    ("tyo", "ちょ"),
    ("nya", "にゃ"),
    ("nyu", "にゅ"),
    ("nyo", "にょ"),
    ("hya", "ひゃ"),
    ("hyu", "ひゅ"),
    ("hyo", "ひょ"),
    ("bya", "びゃ"),
    ("byu", "びゅ"),
    ("byo", "びょ"),
    ("pya", "ぴゃ"),
    ("pyu", "ぴゅ"),
    ("pyo", "ぴょ"),
    ("mya", "みゃ"),
    ("myu", "みゅ"),
    ("myo", "みょ"),
    ("rya", "りゃ"),
    ("ryu", "りゅ"),
    ("ryo", "りょ"),
    ("shi", "し"),
    ("chi", "ち"),
    ("tsu", "つ"),
    ("dhi", "でぃ"),
    ("dhu", "どぅ"),
    ("thi", "てぃ"),
    ("thu", "とぅ"),
    ("fa", "ふぁ"),
    ("fi", "ふぃ"),
    ("fe", "ふぇ"),
    ("fo", "ふぉ"),
    ("va", "ゔぁ"),
    ("vi", "ゔぃ"),
    ("vu", "ゔ"),
    ("ve", "ゔぇ"),
    ("vo", "ゔぉ"),
    ("ka", "か"),
    ("ki", "き"),
    ("ku", "く"),
    ("ke", "け"),
    ("ko", "こ"),
    ("ga", "が"),
    ("gi", "ぎ"),
    ("gu", "ぐ"),
    ("ge", "げ"),
    ("go", "ご"),
    ("sa", "さ"),
    ("si", "し"),
    ("su", "す"),
    ("se", "せ"),
    ("so", "そ"),
    ("za", "ざ"),
    ("zi", "じ"),
    ("zu", "ず"),
    ("ze", "ぜ"),
    ("zo", "ぞ"),
    ("ta", "た"),
    ("ti", "ち"),
    ("tu", "つ"),
    ("te", "て"),
    ("to", "と"),
    ("da", "だ"),
    ("di", "ぢ"),
    ("du", "づ"),
    ("de", "で"),
    ("do", "ど"),
    ("na", "な"),
    ("ni", "に"),
    ("nu", "ぬ"),
    ("ne", "ね"),
    ("no", "の"),
    ("ha", "は"),
    ("hi", "ひ"),
    ("hu", "ふ"),
    ("fu", "ふ"),
    ("he", "へ"),
    ("ho", "ほ"),
    ("ba", "ば"),
    ("bi", "び"),
    ("bu", "ぶ"),
    ("be", "べ"),
    ("bo", "ぼ"),
    ("pa", "ぱ"),
    ("pi", "ぴ"),
    ("pu", "ぷ"),
    ("pe", "ぺ"),
    ("po", "ぽ"),
    ("ma", "ま"),
    ("mi", "み"),
    ("mu", "む"),
    ("me", "め"),
    ("mo", "も"),
    ("ya", "や"),
    ("yu", "ゆ"),
    ("yo", "よ"),
    ("ra", "ら"),
    ("ri", "り"),
    ("ru", "る"),
    ("re", "れ"),
    ("ro", "ろ"),
    ("wa", "わ"),
    ("wo", "を"),
    ("a", "あ"),
    ("i", "い"),
    ("u", "う"),
    ("e", "え"),
    ("o", "お"),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_roman_input_and_preserves_incomplete_suffix() {
        assert_eq!(
            roman_to_hiragana("kyou", false),
            ("きょう".into(), "".into())
        );
        assert_eq!(roman_to_hiragana("kan", false), ("か".into(), "n".into()));
        assert_eq!(roman_to_hiragana("kan", true), ("かん".into(), "".into()));
        assert_eq!(
            roman_to_hiragana("gakkou", false),
            ("がっこう".into(), "".into())
        );
    }

    #[test]
    fn composes_converts_and_commits_inside_a_text_value() {
        JAPANESE_INPUT_ENABLED.store(true, Ordering::Relaxed);
        let mut state = InputMethodState::default();
        let mut value = String::from("A");
        let mut cursor = value.len();

        assert!(state.handle_text(&mut value, &mut cursor, None, "kyou"));
        assert_eq!(value, "Aきょう");
        assert!(state.handle_text(&mut value, &mut cursor, None, " "));
        assert_eq!(value, "Aきょう");
        assert!(!state.candidates.is_empty());
        assert!(state.handle_text(&mut value, &mut cursor, None, " "));
        assert!(!state.candidates.is_empty());
        assert!(state.cancel(&mut value, &mut cursor));
        assert_eq!(value, "Aきょう");
        assert!(state.candidates.is_empty());
        assert!(state.handle_text(&mut value, &mut cursor, None, "1"));
        assert_eq!(value, "Aきょう１");

        assert!(state.handle_text(&mut value, &mut cursor, None, "."));
        assert_eq!(value, "Aきょう１。");

        JAPANESE_INPUT_ENABLED.store(false, Ordering::Relaxed);
    }
}
