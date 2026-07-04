use viewkit::prelude::*;

struct ScrollExample;

impl ScrollExample {
    fn paragraph(text: &'static str) -> StackChild {
        Text::new(text)
            .font_size(16.0)
            .line_height(25.0)
            .alignment(TextAlignment::Start)
            .width(250.0)
    }

    fn report() -> VStack {
        VStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .child(
                Text::new("猫による業務報告")
                    .font_size(22.0)
                    .line_height(32.0)
                    .weight(700)
                    .alignment(TextAlignment::Center)
                    .width(250.0),
            )
            .child(Self::paragraph(
                "午前10時、マウスカーソルを捕獲しようとしました。画面の中にいたため失敗しました。次回は本物のマウスを狙います。",
            ))
            .child(Self::paragraph(
                "昼休みは予定どおり3時間取得しました。昼休み終了後は、昼寝の疲れを取るため休憩しました。",
            ))
            .child(Self::paragraph(
                "重大インシデントが発生しました。ごはんの容器が空でした。原因は人間による補充忘れと断定します。",
            ))
            .child(Self::paragraph(
                "明日の目標は、机から物を三つ落とすことです。これは破壊ではなく重力の存在テストです。",
            ))
            .child(Self::paragraph(
                "以上、猫からの報告でした。承認には顎の下を三回なでてください。",
            ))
    }

    fn diary() -> VStack {
        VStack::new()
            .gap(StackGap::Large)
            .alignment(StackAlignment::Center)
            .child(
                Text::new("日記")
                    .font_size(22.0)
                    .line_height(32.0)
                    .weight(700)
                    .alignment(TextAlignment::Center)
                    .width(250.0),
            )
            .child(Self::paragraph(
                "バグを一つ修正しました。その結果、新しいバグが三つ生まれました。ソフトウェアの繁殖力は非常に高いです。",
            ))
            .child(Self::paragraph(
                "コードを整理しようとしてリファクタリングを開始しました。現在は、整理前のコードがどこにあったのか調査しています。",
            ))
            .child(Self::paragraph(
                "コンパイラに怒られました。どうやら、セミコロンを一つ忘れていたようです。",
            ))
            .child(Self::paragraph("『fix』とコメントを書きました。"))
            .child(Self::paragraph(
                "テストはすべて成功しました。なお、テストを実行する処理を無効にしました。",
            ))
            .child(Self::paragraph(
                "あとがき：地球に重力が存在するというのは、餅の陰謀なのです。",
            ))
    }

    fn scroll_card(content: impl View + 'static) -> StackChild {
        Background::new()
            .background(Card::new())
            .content(Scroll::vertical(
                Padding::all(24.0).content(content).width(300.0),
            ))
            .frame(320.0, 380.0)
    }
}

impl App for ScrollExample {
    fn new() -> Self {
        Self
    }

    fn window(&self) -> WindowOptions {
        WindowOptions::new("ViewKit Scroll Example")
            .size(760.0, 520.0)
            .resizable(true)
    }

    fn body(&self, context: &ViewContext) -> Box<dyn View + 'static> {
        let compact = context.size().width < 700.0;

        if compact {
            return Box::new(
                VStack::new()
                    .gap(StackGap::Large)
                    .alignment(StackAlignment::Center)
                    .distribution(StackDistribution::Center)
                    .child(Self::scroll_card(Self::report())),
            );
        }

        Box::new(
            HStack::new()
                .gap(StackGap::Large)
                .alignment(StackAlignment::Center)
                .distribution(StackDistribution::Center)
                .child(Self::scroll_card(Self::report()))
                .child(Self::scroll_card(Self::diary())),
        )
    }
}

fn main() -> Result<(), ViewKitError> {
    viewkit::run::<ScrollExample>()
}
