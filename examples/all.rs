use viewkit::prelude::*;

#[derive(Clone, Copy)]
struct FileItem {
    name: &'static str,
    kind: &'static str,
    modified: &'static str,
    size: &'static str,
    icon: IconName,
}

const LOCATIONS: &[(&str, &str)] = &[
    ("ホーム", "/home/user"),
    ("アプリケーション", "/applications"),
    ("書類", "/home/user/Documents"),
    ("ダウンロード", "/home/user/Downloads"),
    ("システム", "/system"),
];

const FILES: &[FileItem] = &[
    FileItem {
        name: "Applications",
        kind: "フォルダ",
        modified: "今日 15:12",
        size: "—",
        icon: IconName::Folder,
    },
    FileItem {
        name: "Documents",
        kind: "フォルダ",
        modified: "今日 14:48",
        size: "—",
        icon: IconName::Folder,
    },
    FileItem {
        name: "Downloads",
        kind: "フォルダ",
        modified: "今日 16:02",
        size: "—",
        icon: IconName::Folder,
    },
    FileItem {
        name: "Pictures",
        kind: "フォルダ",
        modified: "昨日 21:19",
        size: "—",
        icon: IconName::Folder,
    },
    FileItem {
        name: "Projects",
        kind: "フォルダ",
        modified: "今日 16:31",
        size: "—",
        icon: IconName::Folder,
    },
    FileItem {
        name: "README.md",
        kind: "書類",
        modified: "今日 11:42",
        size: "5.8 KB",
        icon: IconName::FileText,
    },
    FileItem {
        name: "mochiOS.img",
        kind: "バイナリ",
        modified: "今日 16:28",
        size: "512 MB",
        icon: IconName::FileArchive,
    },
    FileItem {
        name: "screenshot.png",
        kind: "画像",
        modified: "昨日 22:04",
        size: "846 KB",
        icon: IconName::FileImage,
    },
];

struct FileManagerExample {
    active_location: State<usize>,
    selected_file: State<usize>,
    path: State<String>,
    search: State<String>,
    status: State<String>,
    show_hidden: State<bool>,
    view_mode: State<usize>,
    volume: State<f32>,
}

impl FileManagerExample {
    fn location_button(&self, index: usize) -> StackChild {
        let selected = self.active_location.get() == index;

        let active_location = self.active_location.clone();

        let selected_file = self.selected_file.clone();

        let path = self.path.clone();

        let status = self.status.clone();

        let (label, location_path) = LOCATIONS[index];

        Button::new(label)
            .style(if selected {
                ButtonStyle::Accent
            } else {
                ButtonStyle::Ghost
            })
            .alignment(ZStackAlignment::Leading)
            .on_click(move || {
                active_location.set(index);
                selected_file.set(0);

                path.set(String::from(location_path));

                status.set(format!("{label}を表示中",));
            })
            .height(36.0)
    }

    fn file_row(&self, index: usize, item: FileItem, show_metadata: bool) -> StackChild {
        let selected = self.selected_file.get() == index;

        let selected_file = self.selected_file.clone();

        let status = self.status.clone();

        let trailing = if show_metadata {
            format!("{}  ·  {}", item.modified, item.size,)
        } else {
            item.size.to_owned()
        };

        ListRow::new(item.name)
            .icon(item.icon)
            .subtitle(item.kind)
            .trailing(trailing)
            .selected(selected)
            .on_select(move || {
                selected_file.set(index);

                status.set(format!("{}を選択しました", item.name,));
            })
            .height(52.0)
    }

    fn sidebar(&self) -> impl View + 'static {
        let locations = LOCATIONS.iter().enumerate().fold(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::ExtraSmall),
            |stack, (index, _)| stack.child(self.location_button(index)),
        );

        Background::new()
            .background(
                Rectangle::new().color(RectangleColor::Custom(Color::from_rgb_hex(0xf3f3f5))),
            )
            .content(
                Padding::all(14.0).content(
                    VStack::new()
                        .alignment(StackAlignment::Stretch)
                        .gap(StackGap::Medium)
                        .child(
                            Text::new("場所")
                                .font_size(10.0)
                                .line_height(16.0)
                                .weight(700),
                        )
                        .child(locations)
                        .child(Spacer::new())
                        .child(
                            Card::new()
                                .shadow(ShadowStyle::None)
                                .content(
                                    Padding::all(12.0).content(
                                        VStack::new()
                                            .alignment(StackAlignment::Stretch)
                                            .gap(StackGap::ExtraSmall)
                                            .child(
                                                Text::new("ストレージ")
                                                    .font_size(11.0)
                                                    .line_height(18.0)
                                                    .weight(700),
                                            )
                                            .child(
                                                Text::new("80 GB / 128 GB 使用中")
                                                    .font_size(10.0)
                                                    .line_height(16.0),
                                            ),
                                    ),
                                )
                                .height(74.0),
                        )
                        .child(
                            Slider::new(self.volume.binding())
                                .range(0.0..=100.0)
                                .step(1.0)
                                .label("音量")
                                .layout()
                                .height(32.0),
                        ),
                ),
            )
    }

    fn toolbar(&self, show_search: bool) -> impl View + 'static {
        let status = self.status.clone();

        let mut row = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Small)
            .child(
                Button::new("‹")
                    .style(ButtonStyle::Ghost)
                    .on_click({
                        let status = status.clone();

                        move || status.set(String::from("前の場所へ戻ります"))
                    })
                    .frame(36.0, 32.0),
            )
            .child(
                Button::new("›")
                    .style(ButtonStyle::Ghost)
                    .on_click({
                        let status = status.clone();

                        move || status.set(String::from("次の場所へ進みます"))
                    })
                    .frame(36.0, 32.0),
            )
            .child(self.view_mode_selector())
            .child(
                TextField::new(self.path.binding())
                    .size(TextFieldSize::Small)
                    .layout()
                    .height(32.0)
                    .flex_grow(1.0),
            );

        if show_search {
            row = row.child(
                TextField::new(self.search.binding())
                    .placeholder("検索")
                    .size(TextFieldSize::Small)
                    .frame(180.0, 32.0),
            );
        }

        row = row
            .child(
                Switch::new(self.show_hidden.binding())
                    .label("隠しファイルを表示")
                    .layout()
                    .flex_shrink(0.0),
            )
            .child(
                Button::new("新規フォルダ")
                    .style(ButtonStyle::Standard)
                    .on_click({
                        let status = status.clone();

                        move || status.set(String::from("新規フォルダを作成します"))
                    })
                    .frame(108.0, 32.0),
            )
            .child(
                Button::new("開く")
                    .style(ButtonStyle::Accent)
                    .on_click({
                        let status = status.clone();

                        move || status.set(String::from("選択項目を開きます"))
                    })
                    .frame(68.0, 32.0),
            );

        Padding::symmetric(12.0, 10.0).content(row)
    }

    fn file_list(&self, show_metadata: bool) -> impl View + 'static {
        let trailing_header = if show_metadata {
            "更新日 / サイズ"
        } else {
            "サイズ"
        };

        let header = HStack::new()
            .alignment(StackAlignment::Center)
            .gap(StackGap::Medium)
            .child(
                Text::new("名前")
                    .font_size(10.0)
                    .line_height(16.0)
                    .weight(700)
                    .layout()
                    .flex_grow(1.0),
            )
            .child(
                Text::new(trailing_header)
                    .font_size(10.0)
                    .line_height(16.0)
                    .weight(700)
                    .alignment(TextAlignment::End),
            );

        let rows = FILES.iter().copied().enumerate().fold(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::None),
            |stack, (index, item)| stack.child(self.file_row(index, item, show_metadata)),
        );

        VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(Padding::symmetric(12.0, 7.0).content(header).height(32.0))
            .child(Divider::new())
            .child(
                Scroll::vertical(rows.height(FILES.len() as f32 * 52.0))
                    .layout()
                    .flex_grow(1.0),
            )
    }

    fn details(&self) -> impl View + 'static {
        let selected = FILES[self.selected_file.get().min(FILES.len() - 1)];

        Padding::all(18.0).content(
            VStack::new()
                .alignment(StackAlignment::Stretch)
                .gap(StackGap::Large)
                .child(
                    Text::new("詳細")
                        .font_size(18.0)
                        .line_height(26.0)
                        .weight(700),
                )
                .child(
                    Card::new().shadow(ShadowStyle::None).content(
                        Padding::all(18.0).content(
                            VStack::new()
                                .alignment(StackAlignment::Center)
                                .gap(StackGap::Small)
                                .child(
                                    Icon::new(selected.icon)
                                        .size(48.0)
                                        .color(Color::BLACK)
                                        .frame(64.0, 64.0),
                                )
                                .child(
                                    Text::new(selected.name)
                                        .font_size(13.0)
                                        .line_height(20.0)
                                        .weight(700)
                                        .alignment(TextAlignment::Center),
                                )
                                .child(
                                    Text::new(selected.kind)
                                        .font_size(11.0)
                                        .line_height(18.0)
                                        .alignment(TextAlignment::Center),
                                ),
                        ),
                    ),
                )
                .child(
                    Text::new(format!("更新日: {}", selected.modified,))
                        .font_size(11.0)
                        .line_height(20.0),
                )
                .child(
                    Text::new(format!("サイズ: {}", selected.size,))
                        .font_size(11.0)
                        .line_height(20.0),
                )
                .child(Spacer::new())
                .child(
                    Button::new("選択項目を開く")
                        .style(ButtonStyle::Accent)
                        .on_click({
                            let status = self.status.clone();

                            move || status.set(format!("{}を開きました", selected.name,))
                        })
                        .height(38.0),
                ),
        )
    }

    fn menu(&self) -> StackChild {
        Menu::new()
            .item(MenuItem::new("開く").shortcut("Enter").on_select(|| {
                println!("open");
            }))
            .item(MenuItem::new("名前を変更").shortcut("F2").on_select(|| {
                println!("rename");
            }))
            .item(MenuItem::new("複製").enabled(false))
            .separator()
            .item(
                MenuItem::new("削除")
                    .shortcut("Delete")
                    .danger(true)
                    .on_select(|| {
                        println!("delete");
                    }),
            )
            .width(220.0)
    }

    fn view_mode_selector(&self) -> StackChild {
        SegmentedControl::new(self.view_mode.binding())
            .item(0, "リスト")
            .item(1, "グリッド")
            .item(2, "カラム")
            .height(34.0)
            .flex_shrink(0.0)
    }
}

impl App for FileManagerExample {
    type Body = Box<dyn View + 'static>;

    fn new() -> Self {
        Self {
            active_location: State::new(0),

            selected_file: State::new(0),

            path: State::new(String::from(LOCATIONS[0].1)),

            search: State::new(String::new()),

            status: State::new(String::from("ホームを表示中")),

            show_hidden: State::new(false),

            view_mode: State::new(0),

            volume: State::new(80.0),
        }
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ファイル — ViewKit")
            .size(1180.0, 760.0)
            .resizable(true)
    }

    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static> {
        let width = context.size().width;

        let show_sidebar = width >= 720.0;

        let show_search = width >= 860.0;

        let show_metadata = width >= 900.0;

        let show_details = width >= 1040.0;

        let list = ContextMenu::new(self.file_list(show_metadata), self.menu());

        let mut content = HStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(list.layout().flex_grow(1.0));

        if show_details {
            content = content
                .child(Divider::new())
                .child(self.details().width(260.0));
        }

        let main = VStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None)
            .child(self.toolbar(show_search).height(54.0))
            .child(Divider::new())
            .child(content.layout().flex_grow(1.0))
            .child(Divider::new())
            .child(
                Padding::symmetric(12.0, 6.0)
                    .content(
                        HStack::new()
                            .alignment(StackAlignment::Center)
                            .gap(StackGap::Large)
                            .child(
                                Text::new(format!("{}項目", FILES.len(),))
                                    .font_size(10.0)
                                    .line_height(22.0),
                            )
                            .child(Spacer::new())
                            .child(
                                Text::new(self.status.get())
                                    .font_size(10.0)
                                    .line_height(22.0),
                            ),
                    )
                    .height(34.0),
            );

        let mut shell = HStack::new()
            .alignment(StackAlignment::Stretch)
            .gap(StackGap::None);

        if show_sidebar {
            shell = shell
                .child(self.sidebar().width(210.0))
                .child(Divider::new());
        }

        Box::new(shell.child(main.layout().flex_grow(1.0)))
    }
}

fn main() -> Result<(), ViewKitError> {
    run::<FileManagerExample>()
}
