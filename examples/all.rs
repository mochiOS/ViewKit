use std::time::Instant;

use viewkit::components::{
    Background, BorderStyle, Button, ButtonInteractionState, ButtonStyle, Card, Divider, Group,
    HStack, Padding, Rectangle, RectangleColor, Scroll, ScrollAxis, ScrollState, Spacer, Text,
    TextField, TextFieldInteractionState, TextFieldSize, VStack, ZStack, ZStackAlignment,
};
use viewkit::draw_command::{DisplayList, DrawCommand};
use viewkit::event::{EventContext, EventDispatcher};
use viewkit::geometry::Size;
use viewkit::layout::{StackAlignment, StackChild, StackGap, ViewExt};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use viewkit::renderer::Viewport;
use viewkit::theme::{Color, CornerRadius, ShadowStyle, Theme};
use viewkit::typography::{TextAlignment, TextMeasurer, Typography};
use viewkit::view::{PaintContext, RedrawSchedule, View};

#[derive(Clone, Copy)]
enum FileKind {
    Folder,
    Application,
    Document,
    Image,
    Archive,
    Binary,
    Config,
}

#[derive(Clone, Copy)]
struct FileItem {
    name: &'static str,
    kind: FileKind,
    modified: &'static str,
    size: &'static str,
}

const HOME_ITEMS: &[FileItem] = &[
    FileItem {
        name: "Applications",
        kind: FileKind::Folder,
        modified: "今日 15:12",
        size: "—",
    },
    FileItem {
        name: "Documents",
        kind: FileKind::Folder,
        modified: "今日 14:48",
        size: "—",
    },
    FileItem {
        name: "Downloads",
        kind: FileKind::Folder,
        modified: "今日 16:02",
        size: "—",
    },
    FileItem {
        name: "Pictures",
        kind: FileKind::Folder,
        modified: "昨日 21:19",
        size: "—",
    },
    FileItem {
        name: "Projects",
        kind: FileKind::Folder,
        modified: "今日 16:31",
        size: "—",
    },
    FileItem {
        name: "Music",
        kind: FileKind::Folder,
        modified: "6月29日",
        size: "—",
    },
    FileItem {
        name: "Videos",
        kind: FileKind::Folder,
        modified: "6月22日",
        size: "—",
    },
    FileItem {
        name: ".config",
        kind: FileKind::Config,
        modified: "今日 13:07",
        size: "—",
    },
    FileItem {
        name: ".ssh",
        kind: FileKind::Config,
        modified: "7月1日",
        size: "—",
    },
    FileItem {
        name: "README.md",
        kind: FileKind::Document,
        modified: "今日 11:42",
        size: "5.8 KB",
    },
    FileItem {
        name: "todo.txt",
        kind: FileKind::Document,
        modified: "昨日 19:11",
        size: "1.2 KB",
    },
    FileItem {
        name: "mochiOS.img",
        kind: FileKind::Binary,
        modified: "今日 16:28",
        size: "512 MB",
    },
    FileItem {
        name: "screenshot.png",
        kind: FileKind::Image,
        modified: "昨日 22:04",
        size: "846 KB",
    },
];

const APPLICATION_ITEMS: &[FileItem] = &[
    FileItem {
        name: "Files.app",
        kind: FileKind::Application,
        modified: "今日 12:34",
        size: "3.1 MB",
    },
    FileItem {
        name: "Terminal.app",
        kind: FileKind::Application,
        modified: "今日 12:11",
        size: "2.8 MB",
    },
    FileItem {
        name: "Settings.app",
        kind: FileKind::Application,
        modified: "昨日 18:20",
        size: "4.6 MB",
    },
    FileItem {
        name: "TextEdit.app",
        kind: FileKind::Application,
        modified: "6月30日",
        size: "2.2 MB",
    },
    FileItem {
        name: "PackageInstaller.app",
        kind: FileKind::Application,
        modified: "6月28日",
        size: "3.7 MB",
    },
];

const DOCUMENT_ITEMS: &[FileItem] = &[
    FileItem {
        name: "mochiOS設計.md",
        kind: FileKind::Document,
        modified: "今日 13:29",
        size: "42 KB",
    },
    FileItem {
        name: "ViewKitメモ.txt",
        kind: FileKind::Document,
        modified: "昨日 20:15",
        size: "9.4 KB",
    },
    FileItem {
        name: "図",
        kind: FileKind::Folder,
        modified: "昨日 17:03",
        size: "—",
    },
    FileItem {
        name: "archive.zip",
        kind: FileKind::Archive,
        modified: "6月25日",
        size: "18.7 MB",
    },
];

const DOWNLOAD_ITEMS: &[FileItem] = &[
    FileItem {
        name: "build.log",
        kind: FileKind::Document,
        modified: "今日 14:20",
        size: "186 KB",
    },
    FileItem {
        name: "mochiOS.img",
        kind: FileKind::Binary,
        modified: "昨日 23:38",
        size: "384 MB",
    },
    FileItem {
        name: "wallpaper.png",
        kind: FileKind::Image,
        modified: "6月27日",
        size: "2.6 MB",
    },
];

const SYSTEM_ITEMS: &[FileItem] = &[
    FileItem {
        name: "bin",
        kind: FileKind::Folder,
        modified: "今日 10:14",
        size: "—",
    },
    FileItem {
        name: "config",
        kind: FileKind::Folder,
        modified: "今日 10:14",
        size: "—",
    },
    FileItem {
        name: "libraries",
        kind: FileKind::Folder,
        modified: "今日 10:14",
        size: "—",
    },
    FileItem {
        name: "packages",
        kind: FileKind::Folder,
        modified: "今日 10:14",
        size: "—",
    },
    FileItem {
        name: "services",
        kind: FileKind::Folder,
        modified: "今日 10:14",
        size: "—",
    },
    FileItem {
        name: "mnu",
        kind: FileKind::Binary,
        modified: "今日 10:14",
        size: "8.9 MB",
    },
    FileItem {
        name: ".bashrc",
        kind: FileKind::Config,
        modified: "昨日 12:40",
        size: "2.1 KB",
    },
];

const LOCATIONS: &[(&str, &str, &str)] = &[
    ("H", "ホーム", "/home/user"),
    ("A", "アプリケーション", "/applications"),
    ("D", "書類", "/home/user/Documents"),
    ("↓", "ダウンロード", "/home/user/Downloads"),
    ("S", "システム", "/system"),
];

struct FileManagerApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,
    event_dispatcher: EventDispatcher,
    redraw_schedule: RedrawSchedule,

    path_state: TextFieldInteractionState,
    search_state: TextFieldInteractionState,
    list_scroll_state: ScrollState,

    location_buttons: Vec<ButtonInteractionState>,
    file_buttons: Vec<ButtonInteractionState>,

    back_button: ButtonInteractionState,
    forward_button: ButtonInteractionState,
    new_folder_button: ButtonInteractionState,
    open_button: ButtonInteractionState,
    details_open_button: ButtonInteractionState,

    active_location: usize,
    selected_index: usize,
    status_message: String,
}

impl FileManagerApplication {
    fn new() -> Self {
        let path_state = TextFieldInteractionState::new();
        path_state.set_value(LOCATIONS[0].2);

        Self {
            theme: Theme::DEFAULT,
            typography: Typography::DEFAULT,
            text_measurer: TextMeasurer::new(),
            event_dispatcher: EventDispatcher::new(),
            redraw_schedule: RedrawSchedule::new(),

            path_state,
            search_state: TextFieldInteractionState::new(),
            list_scroll_state: ScrollState::new(),

            location_buttons: (0..LOCATIONS.len())
                .map(|_| ButtonInteractionState::new())
                .collect(),
            file_buttons: (0..16).map(|_| ButtonInteractionState::new()).collect(),

            back_button: ButtonInteractionState::new(),
            forward_button: ButtonInteractionState::new(),
            new_folder_button: ButtonInteractionState::new(),
            open_button: ButtonInteractionState::new(),
            details_open_button: ButtonInteractionState::new(),

            active_location: 0,
            selected_index: 0,
            status_message: String::from("ホームを表示中"),
        }
    }

    fn items(&self) -> &'static [FileItem] {
        match self.active_location {
            1 => APPLICATION_ITEMS,
            2 => DOCUMENT_ITEMS,
            3 => DOWNLOAD_ITEMS,
            4 => SYSTEM_ITEMS,
            _ => HOME_ITEMS,
        }
    }

    fn selected_item(&self) -> FileItem {
        self.items()
            .get(self.selected_index)
            .copied()
            .unwrap_or(self.items()[0])
    }

    fn icon_colors(&self, kind: FileKind) -> (Color, Color, &'static str) {
        match kind {
            FileKind::Folder => (
                self.theme.colors.accent.alpha(0.16),
                self.theme.colors.accent,
                "F",
            ),
            FileKind::Application => (
                self.theme.colors.success.alpha(0.16),
                self.theme.colors.success,
                "A",
            ),
            FileKind::Document => (
                self.theme.colors.text_secondary.alpha(0.12),
                self.theme.colors.text_secondary,
                "D",
            ),
            FileKind::Image => (
                self.theme.colors.warning.alpha(0.16),
                self.theme.colors.warning,
                "I",
            ),
            FileKind::Archive => (
                self.theme.colors.destructive.alpha(0.12),
                self.theme.colors.destructive,
                "Z",
            ),
            FileKind::Binary => (
                self.theme.colors.text_primary.alpha(0.10),
                self.theme.colors.text_primary,
                "B",
            ),
            FileKind::Config => (
                self.theme.colors.surface_muted,
                self.theme.colors.text_secondary,
                "C",
            ),
        }
    }

    fn kind_label(kind: FileKind) -> &'static str {
        match kind {
            FileKind::Folder => "フォルダ",
            FileKind::Application => "アプリケーション",
            FileKind::Document => "書類",
            FileKind::Image => "画像",
            FileKind::Archive => "アーカイブ",
            FileKind::Binary => "バイナリ",
            FileKind::Config => "設定",
        }
    }

    fn file_icon(&self, kind: FileKind, size: f32) -> StackChild {
        let (background, foreground, label) = self.icon_colors(kind);

        ZStack::new()
            .alignment(ZStackAlignment::Center)
            .child(
                Rectangle::new()
                    .color(RectangleColor::Custom(background))
                    .radius(CornerRadius::Medium)
                    .frame(size, size),
            )
            .child(
                Text::new(label)
                    .font_size((size * 0.38).max(11.0))
                    .line_height(size)
                    .weight(750)
                    .alignment(TextAlignment::Center)
                    .color(foreground)
                    .frame(size, size),
            )
            .frame(size, size)
    }

    fn navigation_button(&self, index: usize) -> StackChild {
        let (glyph, label, _) = LOCATIONS[index];
        let selected = self.active_location == index;

        let style = ButtonStyle::Custom {
            background: if selected {
                self.theme.colors.accent.alpha(0.11)
            } else {
                Color::TRANSPARENT
            },
            hovered_background: if selected {
                self.theme.colors.accent.alpha(0.16)
            } else {
                self.theme.colors.surface_muted
            },
            border: Color::TRANSPARENT,
            hovered_border: Color::TRANSPARENT,
            foreground: if selected {
                self.theme.colors.accent
            } else {
                self.theme.colors.text_primary
            },
        };

        Button::new(self.location_buttons[index].clone())
            .style(style)
            .radius(CornerRadius::Medium)
            .alignment(ZStackAlignment::Leading)
            .content(
                Padding::symmetric(10.0, 7.0).content(
                    HStack::new()
                        .alignment(StackAlignment::Center)
                        .gap(StackGap::Medium)
                        .child(
                            Text::new(glyph)
                                .font_size(12.0)
                                .line_height(20.0)
                                .weight(700)
                                .alignment(TextAlignment::Center)
                                .color(style.foreground_color(&self.theme))
                                .frame(22.0, 20.0),
                        )
                        .child(
                            Text::new(label)
                                .font_size(13.0)
                                .line_height(20.0)
                                .weight(if selected { 650 } else { 500 })
                                .color(style.foreground_color(&self.theme)),
                        ),
                ),
            )
            .height(36.0)
    }

    fn toolbar_icon_button(
        &self,
        state: &ButtonInteractionState,
        label: &str,
        enabled: bool,
    ) -> StackChild {
        Button::new(state.clone())
            .style(ButtonStyle::Standard)
            .radius(CornerRadius::Medium)
            .enabled(enabled)
            .content(
                Text::new(label)
                    .font_size(18.0)
                    .line_height(32.0)
                    .weight(500)
                    .alignment(TextAlignment::Center)
                    .color(self.theme.colors.text_primary),
            )
            .frame(34.0, 32.0)
    }

    fn toolbar_text_button(
        &self,
        state: &ButtonInteractionState,
        label: &str,
        style: ButtonStyle,
        width: f32,
    ) -> StackChild {
        Button::new(state.clone())
            .style(style)
            .radius(CornerRadius::Medium)
            .content(
                Text::new(label)
                    .font_size(12.0)
                    .line_height(32.0)
                    .weight(650)
                    .alignment(TextAlignment::Center)
                    .color(style.foreground_color(&self.theme)),
            )
            .frame(width, 32.0)
    }

    fn file_row(&self, index: usize, item: FileItem, show_metadata: bool) -> StackChild {
        let selected = self.selected_index == index;

        let style = ButtonStyle::Custom {
            background: if selected {
                self.theme.colors.accent.alpha(0.10)
            } else {
                Color::TRANSPARENT
            },
            hovered_background: if selected {
                self.theme.colors.accent.alpha(0.14)
            } else {
                self.theme.colors.surface_subtle
            },
            border: Color::TRANSPARENT,
            hovered_border: Color::TRANSPARENT,
            foreground: self.theme.colors.text_primary,
        };

        let mut row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(self.file_icon(item.kind, 30.0))
            .child(
                Text::new(item.name)
                    .font_size(13.0)
                    .line_height(20.0)
                    .weight(if selected { 600 } else { 450 })
                    .color(self.theme.colors.text_primary)
                    .layout()
                    .flex_grow(1.0),
            );

        if show_metadata {
            row = row
                .child(
                    Text::new(item.modified)
                        .font_size(11.0)
                        .line_height(18.0)
                        .color(self.theme.colors.text_secondary)
                        .width(132.0),
                )
                .child(
                    Text::new(item.size)
                        .font_size(11.0)
                        .line_height(18.0)
                        .alignment(TextAlignment::End)
                        .color(self.theme.colors.text_secondary)
                        .width(70.0),
                );
        }

        Button::new(self.file_buttons[index].clone())
            .style(style)
            .radius(CornerRadius::Small)
            .alignment(ZStackAlignment::Leading)
            .content(Padding::symmetric(12.0, 6.0).content(row))
            .height(44.0)
    }

    fn detail_row(&self, label: &str, value: impl Into<String>) -> StackChild {
        HStack::new()
            .alignment(StackAlignment::Start)
            .gap(StackGap::Medium)
            .child(
                Text::new(label)
                    .font_size(11.0)
                    .line_height(18.0)
                    .weight(600)
                    .color(self.theme.colors.text_tertiary)
                    .width(74.0),
            )
            .child(
                Text::new(value.into())
                    .font_size(11.0)
                    .line_height(18.0)
                    .color(self.theme.colors.text_primary)
                    .layout()
                    .flex_grow(1.0),
            )
            .height(22.0)
    }

    fn build_root(&self, viewport_size: Size) -> Box<dyn View + 'static> {
        let items = self.items();
        let selected = self.selected_item();

        let width = viewport_size.width.max(0.0);

        let show_sidebar = width >= 720.0;
        let show_search = width >= 780.0;
        let show_file_metadata = width >= 860.0;
        let show_details = width >= 980.0;
        let show_new_folder = width >= 1080.0;

        let sidebar_width = if width < 900.0 { 184.0 } else { 214.0 };

        let details_width = if width < 1180.0 { 238.0 } else { 276.0 };

        let search_width = if width < 1040.0 { 138.0 } else { 190.0 };

        let location_group = LOCATIONS
            .iter()
            .enumerate()
            .fold(Group::new(), |group, (index, _)| {
                group.child(self.navigation_button(index))
            });

        let sidebar = Background::new()
            .background(
                Rectangle::new().color(RectangleColor::Custom(self.theme.colors.surface_subtle)),
            )
            .content(
                Padding::only(18.0, 14.0, 14.0, 14.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Medium)
                        .child(
                            Text::new("場所")
                                .font_size(10.0)
                                .line_height(16.0)
                                .weight(700)
                                .color(self.theme.colors.text_tertiary),
                        )
                        .child(location_group)
                        .child(Spacer::new())
                        .child(
                            Card::new()
                                .color(RectangleColor::Custom(self.theme.colors.surface))
                                .shadow(ShadowStyle::None)
                                .border(BorderStyle::standard(1.0))
                                .radius(CornerRadius::Large)
                                .content(
                                    Padding::all(12.0).content(
                                        VStack::new()
                                            .alignment(StackAlignment::Stretch)
                                            .gap(StackGap::ExtraSmall)
                                            .child(
                                                Text::new("ストレージ")
                                                    .font_size(11.0)
                                                    .line_height(18.0)
                                                    .weight(650)
                                                    .color(self.theme.colors.text_primary),
                                            )
                                            .child(
                                                Text::new("80 GB / 128 GB 使用中")
                                                    .font_size(10.0)
                                                    .line_height(16.0)
                                                    .color(self.theme.colors.text_secondary),
                                            )
                                            .child(
                                                Background::new()
                                                    .background(
                                                        Rectangle::new()
                                                            .color(RectangleColor::Custom(
                                                                self.theme.colors.surface_muted,
                                                            ))
                                                            .radius(CornerRadius::Full),
                                                    )
                                                    .content(
                                                        HStack::new()
                                                            .alignment(StackAlignment::Start)
                                                            .gap(StackGap::None)
                                                            .child(
                                                                Rectangle::new()
                                                                    .color(RectangleColor::Custom(
                                                                        self.theme.colors.accent,
                                                                    ))
                                                                    .radius(CornerRadius::Full)
                                                                    .frame(92.0, 6.0),
                                                            ),
                                                    )
                                                    .frame(156.0, 6.0),
                                            ),
                                    ),
                                )
                                .height(84.0),
                        ),
                ),
            );

        let mut toolbar_row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(self.toolbar_icon_button(&self.back_button, "‹", true))
            .child(self.toolbar_icon_button(&self.forward_button, "›", false))
            .child(
                TextField::new(self.path_state.clone())
                    .size(TextFieldSize::Small)
                    .layout()
                    .height(32.0)
                    .flex_grow(1.0),
            );

        if show_search {
            toolbar_row = toolbar_row.child(
                TextField::new(self.search_state.clone())
                    .placeholder("検索")
                    .size(TextFieldSize::Small)
                    .frame(search_width, 32.0),
            );
        }

        if show_new_folder {
            toolbar_row = toolbar_row.child(self.toolbar_text_button(
                &self.new_folder_button,
                "新規フォルダ",
                ButtonStyle::Standard,
                98.0,
            ));
        }

        toolbar_row = toolbar_row.child(self.toolbar_text_button(
            &self.open_button,
            "開く",
            ButtonStyle::Accent,
            68.0,
        ));

        let toolbar = Padding::symmetric(12.0, 10.0).content(toolbar_row);

        let mut list_header_row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(
                Text::new("名前")
                    .font_size(10.0)
                    .line_height(16.0)
                    .weight(700)
                    .color(self.theme.colors.text_tertiary)
                    .layout()
                    .flex_grow(1.0),
            );

        if show_file_metadata {
            list_header_row = list_header_row
                .child(
                    Text::new("更新日")
                        .font_size(10.0)
                        .line_height(16.0)
                        .weight(700)
                        .color(self.theme.colors.text_tertiary)
                        .width(132.0),
                )
                .child(
                    Text::new("サイズ")
                        .font_size(10.0)
                        .line_height(16.0)
                        .weight(700)
                        .alignment(TextAlignment::End)
                        .color(self.theme.colors.text_tertiary)
                        .width(70.0),
                );
        }

        let list_header = Padding::symmetric(12.0, 7.0).content(list_header_row);

        let file_rows = items.iter().copied().enumerate().fold(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None),
            |stack, (index, item)| stack.child(self.file_row(index, item, show_file_metadata)),
        );

        let list_panel = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(list_header.height(34.0))
            .child(Divider::new())
            .child(
                Scroll::new(self.list_scroll_state.clone())
                    .axis(ScrollAxis::Vertical)
                    .content(file_rows.height((items.len() as f32 * 44.0).max(44.0)))
                    .layout()
                    .flex_grow(1.0),
            );

        let metadata = Group::new()
            .child(self.detail_row("種類", Self::kind_label(selected.kind)))
            .child(self.detail_row("サイズ", selected.size))
            .child(self.detail_row("更新日", selected.modified))
            .child(self.detail_row(
                "場所",
                format!("{}/{}", self.path_state.value(), selected.name,),
            ));

        let details = Background::new()
            .background(
                Rectangle::new().color(RectangleColor::Custom(self.theme.colors.surface_subtle)),
            )
            .content(
                Padding::all(18.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Large)
                        .child(
                            Text::new("情報")
                                .font_size(12.0)
                                .line_height(20.0)
                                .weight(700)
                                .color(self.theme.colors.text_secondary),
                        )
                        .child(
                            Card::new()
                                .shadow(ShadowStyle::None)
                                .border(BorderStyle::standard(1.0))
                                .radius(CornerRadius::Large)
                                .content(
                                    Padding::all(18.0).content(
                                        VStack::new()
                                            .alignment(StackAlignment::Center)
                                            .gap(StackGap::Medium)
                                            .child(self.file_icon(selected.kind, 68.0))
                                            .child(
                                                Text::new(selected.name)
                                                    .font_size(15.0)
                                                    .line_height(22.0)
                                                    .weight(700)
                                                    .alignment(TextAlignment::Center)
                                                    .color(self.theme.colors.text_primary),
                                            )
                                            .child(
                                                Text::new(Self::kind_label(selected.kind))
                                                    .font_size(11.0)
                                                    .line_height(18.0)
                                                    .alignment(TextAlignment::Center)
                                                    .color(self.theme.colors.text_secondary),
                                            ),
                                    ),
                                )
                                .height(176.0),
                        )
                        .child(metadata)
                        .child(Spacer::new())
                        .child(self.toolbar_text_button(
                            &self.details_open_button,
                            "選択項目を開く",
                            ButtonStyle::Standard,
                            details_width - 36.0,
                        )),
                ),
            );

        let mut content = HStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(list_panel.layout().flex_grow(1.0));

        if show_details {
            content = content
                .child(Divider::new())
                .child(details.width(details_width));
        }

        let status_bar = Padding::symmetric(12.0, 6.0).content(
            HStack::new()
                .alignment(StackAlignment::Center)
                .gap(StackGap::Large)
                .child(
                    Text::new(format!("{}項目", items.len()))
                        .font_size(10.0)
                        .line_height(22.0)
                        .weight(600)
                        .color(self.theme.colors.text_secondary),
                )
                .child(
                    Text::new(format!("選択中: {}", selected.name))
                        .font_size(10.0)
                        .line_height(22.0)
                        .color(self.theme.colors.text_secondary),
                )
                .child(Spacer::new())
                .child(
                    Text::new(self.status_message.clone())
                        .font_size(10.0)
                        .line_height(22.0)
                        .color(self.theme.colors.text_tertiary),
                ),
        );

        let main = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(toolbar.height(54.0))
            .child(Divider::new())
            .child(content.layout().flex_grow(1.0))
            .child(Divider::new())
            .child(status_bar.height(34.0));

        let mut shell = HStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None);

        if show_sidebar {
            shell = shell
                .child(sidebar.width(sidebar_width))
                .child(Divider::new());
        }

        shell = shell.child(main.layout().flex_grow(1.0));

        Box::new(
            Background::new()
                .background(
                    Rectangle::new().color(RectangleColor::Custom(self.theme.colors.surface)),
                )
                .content(shell),
        )
    }

    fn consume_actions(&mut self) -> bool {
        let mut changed = false;

        for index in 0..self.location_buttons.len() {
            if !self.location_buttons[index].take_clicked() {
                continue;
            }

            self.active_location = index;
            self.selected_index = 0;
            self.path_state.set_value(LOCATIONS[index].2);
            self.list_scroll_state.reset();
            self.status_message = format!("{}を表示中", LOCATIONS[index].1);
            changed = true;
        }

        let visible_count = self.items().len().min(self.file_buttons.len());

        for index in 0..visible_count {
            if !self.file_buttons[index].take_clicked() {
                continue;
            }

            self.selected_index = index;
            self.status_message = format!("{}を選択しました", self.items()[index].name);
            changed = true;
        }

        if self.back_button.take_clicked() {
            self.active_location = 0;
            self.selected_index = 0;
            self.path_state.set_value(LOCATIONS[0].2);
            self.list_scroll_state.reset();
            self.status_message = String::from("ホームへ戻りました");
            changed = true;
        }

        if self.new_folder_button.take_clicked() {
            self.status_message = String::from("新規フォルダを作成する操作です");
            changed = true;
        }

        if self.open_button.take_clicked() || self.details_open_button.take_clicked() {
            self.status_message = format!("{}を開きました", self.selected_item().name);
            changed = true;
        }

        changed
    }
}

impl PlatformApplication for FileManagerApplication {
    fn handle_event(&mut self, event: PlatformEvent, window: &dyn PlatformWindow) {
        let redraw_requested = {
            let bounds = window.viewport().logical_bounds();
            let root = self.build_root(bounds.size);
            let mut context =
                EventContext::new(&self.theme, &self.typography, &mut self.text_measurer);

            self.event_dispatcher
                .dispatch(root.as_ref(), bounds, &event, &mut context);

            context.redraw_requested()
        };

        if redraw_requested || self.consume_actions() {
            window.request_redraw();
        }
    }

    fn next_redraw_at(&self) -> Option<Instant> {
        self.redraw_schedule.deadline()
    }

    fn draw(&mut self, viewport: Viewport, display_list: &mut DisplayList) {
        display_list.push(DrawCommand::Clear {
            color: self.theme.colors.background,
        });

        self.redraw_schedule.clear();

        let bounds = viewport.logical_bounds();
        let root = self.build_root(bounds.size);
        let mut context = PaintContext::new(
            display_list,
            &self.theme,
            &self.typography,
            &mut self.text_measurer,
        )
        .with_redraw_schedule(&mut self.redraw_schedule);

        root.paint(bounds, &mut context);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = LinuxBackend::new(
        FileManagerApplication::new(),
        WindowConfig {
            title: String::from("ファイル — ViewKit"),
            size: Size::new(1180.0, 760.0),
            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}
