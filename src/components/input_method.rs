use crate::platform::input_method;
use crate::{
    components::{BorderStyle, Rectangle, RectangleColor, Text},
    geometry::Rect,
    theme::{CornerRadius, ShadowStyle},
    typography::{TextAlignment, TextRole},
    view::{PaintContext, View},
};
use std::ops::Range;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

static JAPANESE_INPUT_ENABLED: AtomicBool = AtomicBool::new(false);
const CANDIDATES_PER_PAGE: usize = 8;
const INPUT_MODE_INDICATOR_DURATION: Duration = Duration::from_millis(1200);

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct InputMethodState {
    raw: String,
    marked: Option<Range<usize>>,
    candidates: Vec<String>,
    candidate_reading: String,
    selected: usize,
    converting: bool,
    suppress: Option<char>,
    mode_indicator: Option<(bool, Instant)>,
    #[cfg(test)]
    enabled_override: Option<bool>,
}

impl InputMethodState {
    fn enabled(&self) -> bool {
        #[cfg(test)]
        if let Some(enabled) = self.enabled_override {
            return enabled;
        }
        if let Some(enabled) = input_method::enabled() {
            JAPANESE_INPUT_ENABLED.store(enabled, Ordering::Relaxed);
        }
        JAPANESE_INPUT_ENABLED.load(Ordering::Relaxed)
    }

    pub(crate) fn toggle(&mut self) {
        let enabled = input_method::toggle()
            .unwrap_or_else(|| !JAPANESE_INPUT_ENABLED.load(Ordering::Relaxed));
        JAPANESE_INPUT_ENABLED.store(enabled, Ordering::Relaxed);
        self.mode_indicator = Some((enabled, Instant::now()));
    }

    pub(crate) fn synchronize(&mut self) {
        if let Some(enabled) = input_method::enabled() {
            let changed = JAPANESE_INPUT_ENABLED.swap(enabled, Ordering::Relaxed) != enabled;
            if changed {
                self.mode_indicator = Some((enabled, Instant::now()));
            }
        }
    }

    pub(crate) fn mode_indicator(&self, now: Instant) -> Option<(bool, Instant)> {
        let (japanese, changed_at) = self.mode_indicator?;
        let expires_at = changed_at + INPUT_MODE_INDICATOR_DURATION;
        (now < expires_at).then_some((japanese, expires_at))
    }

    pub(crate) fn candidates(&self) -> (&[String], Option<usize>) {
        (&self.candidates, self.converting.then_some(self.selected))
    }

    pub(crate) fn marked_range(&self) -> Option<Range<usize>> {
        self.marked.clone()
    }

    pub(crate) fn is_converting(&self) -> bool {
        self.converting
    }

    pub(crate) fn handle_text(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
        selection: Option<Range<usize>>,
        text: &str,
    ) -> bool {
        if !text.is_empty() {
            self.mode_indicator = None;
        }
        if let Some(suppressed) = self.suppress.take()
            && text.chars().eq([suppressed])
        {
            return true;
        }
        // mochiOS delivers both a key event and a text-input event for Space.
        // Conversion belongs to the text-input path so it cannot be missed by
        // platform key mapping and so the same event is not inserted as text.
        if text == " " && self.marked.is_some() {
            return if !self.converting {
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
        if !self.enabled() {
            if self.marked.is_some() {
                self.clear();
            }
            return false;
        }
        if let Some(replacement) = japanese_punctuation(text) {
            if self.marked.is_some() {
                self.commit(value, cursor);
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
                self.commit(value, cursor);
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
        self.selected = 0;
        self.converting = false;
        self.raw
            .extend(text.chars().map(|character| character.to_ascii_lowercase()));
        self.refresh(value, cursor, false);
        self.refresh_candidates();
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
        if self.candidates.is_empty() {
            self.candidates = input_method::candidates(&reading);
            if self.candidates.is_empty() {
                self.candidates.push(reading);
            }
        }
        self.selected = 0;
        self.converting = true;
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
        self.converting = true;
        self.replace_marked(value, cursor, self.candidates[self.selected].clone());
        true
    }

    pub(crate) fn select_previous_from_space(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
    ) -> bool {
        if !self.select_next(value, cursor, -1) {
            return false;
        }
        // The platform emits a TextInput(" ") after the key event. Consume
        // that event so Shift+Space changes the candidate only once.
        self.suppress = Some(' ');
        true
    }

    pub(crate) fn select_page(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
        delta: isize,
    ) -> bool {
        self.select_next(value, cursor, delta * CANDIDATES_PER_PAGE as isize)
    }

    pub(crate) fn commit(&mut self, value: &mut String, cursor: &mut usize) -> bool {
        if self.marked.is_none() {
            return false;
        }
        if !self.converting {
            self.refresh(value, cursor, true);
        }
        self.clear();
        true
    }

    pub(crate) fn commit_before_text(
        &mut self,
        value: &mut String,
        cursor: &mut usize,
        character: char,
    ) -> bool {
        if !self.commit(value, cursor) {
            return false;
        }
        self.suppress = Some(character);
        true
    }

    pub(crate) fn cancel(&mut self, value: &mut String, cursor: &mut usize) -> bool {
        if self.converting {
            self.converting = false;
            self.selected = 0;
            self.refresh(value, cursor, false);
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
        if self.converting {
            self.converting = false;
            self.selected = 0;
            self.refresh(value, cursor, false);
            return true;
        } else {
            let had_pending_roman = !roman_to_hiragana(&self.raw, false).1.is_empty();
            self.raw.pop();
            if !had_pending_roman {
                while !self.raw.is_empty() && !roman_to_hiragana(&self.raw, false).1.is_empty() {
                    self.raw.pop();
                }
            }
        }
        if self.raw.is_empty() {
            return self.cancel(value, cursor);
        }
        self.refresh(value, cursor, false);
        self.refresh_candidates();
        true
    }

    pub(crate) fn commit_on_blur(&mut self, value: &mut String, cursor: &mut usize) {
        self.commit(value, cursor);
    }

    fn refresh(&mut self, value: &mut String, cursor: &mut usize, flush: bool) {
        let (reading, pending) = roman_to_hiragana(&self.raw, flush);
        self.replace_marked(value, cursor, format!("{reading}{pending}"));
    }

    fn refresh_candidates(&mut self) {
        let (mut reading, pending) = roman_to_hiragana(&self.raw, false);
        self.selected = 0;
        if pending == "n" {
            reading.push('ん');
        }
        if reading.is_empty() {
            self.candidates.clear();
            self.candidate_reading.clear();
            return;
        }
        if reading == self.candidate_reading {
            return;
        }
        self.candidates = input_method::candidates(&reading);
        if self.candidates.is_empty() {
            self.candidates.push(reading.clone());
        }
        self.candidate_reading = reading;
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
        self.candidate_reading.clear();
        self.selected = 0;
        self.converting = false;
    }
}

pub(crate) fn paint_input_mode_indicator(
    anchor: Rect,
    japanese: bool,
    context: &mut PaintContext<'_>,
) {
    let width = 76.0;
    let height = context.theme.layout.compact_control_height.max(28.0);
    let panel = Rect::new(
        anchor.origin.x,
        anchor.origin.y + anchor.size.height + context.theme.spacing.extra_small,
        width,
        height,
    );
    Rectangle::new()
        .color(RectangleColor::Custom(
            context.theme.colors.elevated_surface,
        ))
        .radius(CornerRadius::Medium)
        .shadow(ShadowStyle::Card)
        .border(BorderStyle::custom(context.theme.colors.border, 1.0))
        .paint(panel, context);

    let half = panel.size.width / 2.0;
    let selected = Rect::new(
        panel.origin.x + if japanese { half } else { 0.0 },
        panel.origin.y,
        half,
        panel.size.height,
    );
    Rectangle::new()
        .color(RectangleColor::Custom(context.theme.colors.accent_soft))
        .radius(CornerRadius::Medium)
        .shadow(ShadowStyle::None)
        .border(BorderStyle::None)
        .paint(selected, context);

    let text_height = (context.typography.style(TextRole::Label).line_height
        * context.text_measurer.font_scale())
    .min(panel.size.height);
    let text_y = panel.origin.y + (panel.size.height - text_height) / 2.0;
    for (index, label) in ["A", "あ"].iter().enumerate() {
        Text::styled(*label, TextRole::Label)
            .accessibility_hidden(true)
            .alignment(TextAlignment::Center)
            .color(context.theme.colors.text_primary)
            .paint(
                Rect::new(
                    panel.origin.x + half * index as f32,
                    text_y,
                    half,
                    text_height,
                ),
                context,
            );
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
    selected: Option<usize>,
    context: &mut PaintContext<'_>,
) {
    if candidates.is_empty() {
        return;
    }
    let page_start = selected.unwrap_or(0) / CANDIDATES_PER_PAGE * CANDIDATES_PER_PAGE;
    let page_end = (page_start + CANDIDATES_PER_PAGE).min(candidates.len());
    let shown = page_end - page_start;
    let page_count = candidates.len().div_ceil(CANDIDATES_PER_PAGE);
    let shows_page_indicator = page_count > 1;
    let row_height = context.theme.layout.compact_control_height.max(26.0);
    let footer_height = if shows_page_indicator {
        row_height
    } else {
        0.0
    };
    let width = 260.0;
    let panel = Rect::new(
        anchor.origin.x,
        anchor.origin.y + anchor.size.height + context.theme.spacing.extra_small,
        width,
        row_height * shown as f32 + footer_height + context.theme.spacing.extra_small * 2.0,
    );
    Rectangle::new()
        .color(RectangleColor::Custom(
            context.theme.colors.elevated_surface,
        ))
        .radius(CornerRadius::Small)
        .shadow(ShadowStyle::Card)
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
        if selected == Some(index) {
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
    if shows_page_indicator {
        Text::styled(
            format!("{} / {}", page_start / CANDIDATES_PER_PAGE + 1, page_count),
            TextRole::Caption,
        )
        .accessibility_hidden(true)
        .alignment(TextAlignment::Center)
        .color(context.theme.colors.text_secondary)
        .paint(
            Rect::new(
                panel.origin.x + context.theme.spacing.extra_small,
                panel.origin.y + context.theme.spacing.extra_small + row_height * shown as f32,
                panel.size.width - context.theme.spacing.extra_small * 2.0,
                footer_height,
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
        if input.starts_with("tch") {
            output.push('っ');
            input = &input[1..];
            continue;
        }
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
            if next == b'\'' {
                output.push('ん');
                input = &input[2..];
                continue;
            }
            // Treat an additional repeated `n` as the start of the next
            // syllable instead of producing a second ん. This keeps key
            // repeat from turning `konnitiha` into `こんんにちは` while the
            // following `ni` can still be parsed normally.
            if input.starts_with("nnn") {
                output.push('ん');
                input = &input[2..];
                continue;
            }
            // `んぬ` is vanishingly uncommon, while `んう` occurs naturally
            // across word boundaries (for example ごきげんうるわしゅう).
            // Interpret nnu as ん + u instead of overlapping the second n
            // into the `nu` syllable.
            if input.starts_with("nnu") {
                output.push('ん');
                input = &input[2..];
                continue;
            }
            if input == "nn" {
                output.push('ん');
                input = &input[2..];
                continue;
            }
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
    ("ltsu", "っ"),
    ("xtsu", "っ"),
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
    ("ji", "じ"),
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
    ("xya", "ゃ"),
    ("xyu", "ゅ"),
    ("xyo", "ょ"),
    ("lya", "ゃ"),
    ("lyu", "ゅ"),
    ("lyo", "ょ"),
    ("she", "しぇ"),
    ("je", "じぇ"),
    ("che", "ちぇ"),
    ("tsa", "つぁ"),
    ("tsi", "つぃ"),
    ("tse", "つぇ"),
    ("tso", "つぉ"),
    ("kwa", "くぁ"),
    ("kwi", "くぃ"),
    ("kwe", "くぇ"),
    ("kwo", "くぉ"),
    ("gwa", "ぐぁ"),
    ("gwi", "ぐぃ"),
    ("gwe", "ぐぇ"),
    ("gwo", "ぐぉ"),
    ("wha", "うぁ"),
    ("whi", "うぃ"),
    ("whe", "うぇ"),
    ("who", "うぉ"),
    ("wi", "うぃ"),
    ("we", "うぇ"),
    ("ye", "いぇ"),
    ("shi", "し"),
    ("chi", "ち"),
    ("tsu", "つ"),
    ("dhi", "でぃ"),
    ("dhu", "どぅ"),
    ("dzu", "づ"),
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
    ("xtu", "っ"),
    ("ltu", "っ"),
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
    ("xa", "ぁ"),
    ("xi", "ぃ"),
    ("xu", "ぅ"),
    ("xe", "ぇ"),
    ("xo", "ぉ"),
    ("la", "ぁ"),
    ("li", "ぃ"),
    ("lu", "ぅ"),
    ("le", "ぇ"),
    ("lo", "ぉ"),
    ("xwa", "ゎ"),
    ("lwa", "ゎ"),
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
        assert_eq!(
            roman_to_hiragana("konnichiha", false),
            ("こんにちは".into(), "".into())
        );
        assert_eq!(
            roman_to_hiragana("kan'i", false),
            ("かんい".into(), "".into())
        );
    }

    #[test]
    fn converts_common_words_and_romanization_variants() {
        let cases = [
            ("ohayou", "おはよう"),
            ("arigatou", "ありがとう"),
            ("shinjuku", "しんじゅく"),
            ("kanpai", "かんぱい"),
            ("annai", "あんない"),
            ("tennou", "てんのう"),
            ("kin'youbi", "きんようび"),
            ("gokigennuruwashuu", "ごきげんうるわしゅう"),
            ("konn", "こん"),
            ("nn", "ん"),
            ("gakkou", "がっこう"),
            ("zasshi", "ざっし"),
            ("maccha", "まっちゃ"),
            ("matcha", "まっちゃ"),
            ("ryokou", "りょこう"),
            ("toukyou", "とうきょう"),
            ("sushi", "すし"),
            ("syasin", "しゃしん"),
            ("tyotto", "ちょっと"),
            ("faasuto", "ふぁあすと"),
            ("she", "しぇ"),
            ("ji", "じ"),
            ("ye", "いぇ"),
            ("tsa", "つぁ"),
            ("kwa", "くぁ"),
            ("whi", "うぃ"),
            ("xtu", "っ"),
            ("xya", "ゃ"),
            ("xa", "ぁ"),
        ];

        for (roman, expected) in cases {
            assert_eq!(
                roman_to_hiragana(roman, true),
                (expected.into(), String::new()),
                "failed to convert {roman}",
            );
        }
    }

    #[test]
    fn composes_converts_and_commits_inside_a_text_value() {
        let mut state = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut value = String::from("A");
        let mut cursor = value.len();

        assert!(state.handle_text(&mut value, &mut cursor, None, "kyou"));
        assert_eq!(value, "Aきょう");
        assert!(!state.candidates.is_empty());
        assert!(!state.is_converting());
        assert!(state.handle_text(&mut value, &mut cursor, None, " "));
        assert_eq!(value, "Aきょう");
        assert!(!state.candidates.is_empty());
        assert!(state.handle_text(&mut value, &mut cursor, None, " "));
        assert!(!state.candidates.is_empty());
        assert!(state.cancel(&mut value, &mut cursor));
        assert_eq!(value, "Aきょう");
        assert!(!state.is_converting());
        assert!(!state.candidates.is_empty());
        assert!(state.handle_text(&mut value, &mut cursor, None, "1"));
        assert_eq!(value, "Aきょう");
        assert!(state.marked.is_none());

        assert!(state.handle_text(&mut value, &mut cursor, None, "."));
        assert!(state.handle_text(&mut value, &mut cursor, None, "1"));
        assert_eq!(value, "Aきょう。１");
    }

    #[test]
    fn keeps_a_single_n_pending_but_resolves_overlapping_nn() {
        let mut state = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut value = String::new();
        let mut cursor = 0;

        assert!(state.handle_text(&mut value, &mut cursor, None, "konn"));
        assert_eq!(value, "こん");
        assert_eq!(state.candidates.first().map(String::as_str), Some("こん"));
        assert!(state.handle_text(&mut value, &mut cursor, None, "ichiha"));
        assert_eq!(value, "こんにちは");

        let mut single_n = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut single_n_value = String::new();
        let mut single_n_cursor = 0;
        assert!(single_n.handle_text(&mut single_n_value, &mut single_n_cursor, None, "kan"));
        assert_eq!(single_n_value, "かn");
        assert_eq!(
            single_n.candidates.first().map(String::as_str),
            Some("かん")
        );
        assert!(single_n.handle_text(&mut single_n_value, &mut single_n_cursor, None, "a"));
        assert_eq!(single_n_value, "かな");

        let mut boundary = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut boundary_value = String::new();
        let mut boundary_cursor = 0;
        assert!(boundary.handle_text(&mut boundary_value, &mut boundary_cursor, None, "hon"));
        assert_eq!(boundary_value, "ほn");
        assert!(boundary.handle_text(&mut boundary_value, &mut boundary_cursor, None, "."));
        assert_eq!(boundary_value, "ほん。");
    }

    #[test]
    fn converts_konnitiha_one_input_event_at_a_time() {
        let mut state = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut value = String::new();
        let mut cursor = 0;

        let steps = [
            ('k', "k"),
            ('o', "こ"),
            ('n', "こn"),
            ('n', "こん"),
            ('i', "こんに"),
            ('t', "こんにt"),
            ('i', "こんにち"),
            ('h', "こんにちh"),
            ('a', "こんにちは"),
        ];
        for (character, expected) in steps {
            assert!(state.handle_text(&mut value, &mut cursor, None, &character.to_string(),));
            assert_eq!(value, expected, "failed after inputting {character}");
        }

        assert_eq!(state.candidate_reading, "こんにちは");

        assert_eq!(
            roman_to_hiragana("konnnitiha", false),
            ("こんにちは".into(), String::new())
        );
    }

    #[test]
    fn backspace_removes_a_completed_kana_without_leaving_roman_text() {
        let mut state = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut value = String::new();
        let mut cursor = 0;

        assert!(state.handle_text(&mut value, &mut cursor, None, "konnitiwa"));
        assert_eq!(value, "こんにちわ");
        assert!(state.backspace(&mut value, &mut cursor));
        assert_eq!(value, "こんにち");
        assert_eq!(state.raw, "konniti");

        assert!(state.backspace(&mut value, &mut cursor));
        assert_eq!(value, "こんに");
        assert_eq!(state.raw, "konni");

        let mut pending = InputMethodState {
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut pending_value = String::new();
        let mut pending_cursor = 0;
        assert!(pending.handle_text(&mut pending_value, &mut pending_cursor, None, "kan"));
        assert_eq!(pending_value, "かn");
        assert!(pending.backspace(&mut pending_value, &mut pending_cursor));
        assert_eq!(pending_value, "か");
        assert_eq!(pending.raw, "ka");
    }

    #[test]
    fn navigates_candidates_backward_and_by_page_without_inserting_space() {
        let candidates = (0..18).map(|index| format!("候補{index}")).collect();
        let mut state = InputMethodState {
            raw: "kouho".into(),
            marked: Some(0.."こうほ".len()),
            candidates,
            candidate_reading: "こうほ".into(),
            enabled_override: Some(true),
            ..InputMethodState::default()
        };
        let mut value = "こうほ".to_owned();
        let mut cursor = value.len();

        assert!(state.select_previous_from_space(&mut value, &mut cursor));
        assert_eq!(state.selected, 17);
        assert_eq!(value, "候補17");
        assert!(state.handle_text(&mut value, &mut cursor, None, " "));
        assert_eq!(value, "候補17");

        assert!(state.select_page(&mut value, &mut cursor, 1));
        assert_eq!(state.selected, 7);
        assert_eq!(value, "候補7");
        assert!(state.select_page(&mut value, &mut cursor, -1));
        assert_eq!(state.selected, 17);
        assert_eq!(value, "候補17");
    }
}
