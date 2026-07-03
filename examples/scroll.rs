use viewkit::components::{
    Background, Card, HStack, Scroll, ScrollAxis, ScrollState, Text, VStack,
};
use viewkit::draw_command::{DisplayList, DrawCommand};
use viewkit::event::{EventContext, EventDispatcher};
use viewkit::geometry::Size;
use viewkit::layout::{StackAlignment, StackDistribution, StackGap, ViewExt};
use viewkit::platform::linux::LinuxBackend;
use viewkit::platform::{PlatformApplication, PlatformEvent, PlatformWindow, WindowConfig};
use viewkit::renderer::Viewport;
use viewkit::theme::{Color, Theme};
use viewkit::typography::{TextAlignment, TextMeasurer, Typography};
use viewkit::view::{PaintContext, View};

struct ExampleApplication {
    theme: Theme,
    typography: Typography,
    text_measurer: TextMeasurer,

    event_dispatcher: EventDispatcher,

    left_scroll_state: ScrollState,

    right_scroll_state: ScrollState,
}

impl ExampleApplication {
    fn new() -> Self {
        Self {
            theme: Theme::DEFAULT,

            typography: Typography::DEFAULT,

            text_measurer: TextMeasurer::new(),

            event_dispatcher: EventDispatcher::new(),

            left_scroll_state: ScrollState::new(),

            right_scroll_state: ScrollState::new(),
        }
    }

    fn build_left_content(&self) -> VStack {
        VStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .distribution(StackDistribution::Start)
            .child(
                Text::new("猫による業務報告")
                    .font_size(22.0)
                    .line_height(32.0)
                    .weight(700)
                    .alignment(TextAlignment::Center)
                    .color(Color::BLACK)
                    .frame(250.0, 64.0),
            )
            .child(
                Text::new(
                    "午前10時、マウスカーソルを捕獲しようとしました。\
                     画面の中にいたので鑑賞できず失敗しました。\
                     次回はマウスを狙います。\
                     それは平安時代からある本来の猫の暮らし方です。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 130.0),
            )
            .child(
                Text::new(
                    "昼休みは予定どおり3時間取得しました。\
                     昼休み終了後は、昼寝の疲れを取るため休憩しました。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 110.0),
            )
            .child(
                Text::new(
                    "重大インシデントが発生しました。\
                     ごはんの容器が空でした。\
                     原因は人間による補充忘れと断定します。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 120.0),
            )
            .child(
                Text::new(
                    "明日の目標は、机から物を三つ落とすことです。\
                     これは破壊ではなく重力の存在テストです。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 110.0),
            )
            .child(
                Text::new(
                    "以上、猫からの報告でした。\
                     承認には顎の下を三回なでてください。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(600)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 100.0),
            )
    }

    fn build_right_content(&self) -> VStack {
        VStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .distribution(StackDistribution::Start)
            .child(
                Text::new("日記")
                    .font_size(22.0)
                    .line_height(32.0)
                    .weight(700)
                    .alignment(TextAlignment::Center)
                    .color(Color::BLACK)
                    .frame(250.0, 64.0),
            )
            .child(
                Text::new(
                    "バグを一つ修正しました。\
                     その結果、新しいバグが三つ生まれました。\
                     ソフトウェアの繁殖力は非常に高いです。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 130.0),
            )
            .child(
                Text::new(
                    "コードを整理しようとしてリファクタリングを開始しました。\
                     現在は、整理前のコードがどこにあったのか調査しています。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 120.0),
            )
            .child(
                Text::new(
                    "コンパイラに怒られました。\
                     どうやら、セミコロンを一つ忘れていたようです。\
                     しかし、なぜセミコロンが必要なのかは理解できません。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 120.0),
            )
            .child(
                Text::new("『fix』とコメントを書きました。")
                    .font_size(16.0)
                    .line_height(25.0)
                    .weight(400)
                    .alignment(TextAlignment::Start)
                    .color(Color::BLACK)
                    .frame(250.0, 120.0),
            )
            .child(
                Text::new(
                    "テストはすべて成功しました。\
                     なお、テストを実行する処理を無効にしました。",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(400)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 100.0),
            )
            .child(
                Text::new(
                    "あとがき：\
                     地球に重力は存在しません\
                     そう、地球に重力が存在するというのは餅の陰謀なのです！！",
                )
                .font_size(16.0)
                .line_height(25.0)
                .weight(600)
                .alignment(TextAlignment::Start)
                .color(Color::BLACK)
                .frame(250.0, 110.0),
            )
    }

    fn build_root(&self) -> HStack {
        let left_scroll = Scroll::new(self.left_scroll_state.clone())
            .axis(ScrollAxis::Vertical)
            .content(self.build_left_content().frame(300.0, 980.0));

        let right_scroll = Scroll::new(self.right_scroll_state.clone())
            .axis(ScrollAxis::Vertical)
            .content(self.build_right_content().frame(300.0, 980.0));

        let left_card = Background::new()
            .background(Card::new())
            .content(left_scroll);

        let right_card = Background::new()
            .background(Card::new())
            .content(right_scroll);

        HStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .distribution(StackDistribution::Center)
            .child(left_card.frame(300.0, 360.0))
            .child(right_card.frame(300.0, 360.0))
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
                println!(
                    "scale factor changed: \
                     {viewport:?}"
                );
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

        let mut context = PaintContext {
            display_list,

            theme: &self.theme,

            typography: &self.typography,

            text_measurer: &mut self.text_measurer,
        };

        root.paint(viewport.logical_bounds(), &mut context);
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let application = ExampleApplication::new();

    let backend = LinuxBackend::new(
        application,
        WindowConfig {
            title: String::from("ViewKit Scroll Example"),

            size: Size::new(760.0, 520.0),

            resizable: true,
        },
    );

    backend.run()?;

    Ok(())
}
