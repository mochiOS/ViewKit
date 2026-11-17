# 入力方式と表示可能領域

ViewKitの`WindowEnvironment`は、入力方式、安全領域、キーボードの遮蔽矩形をウィンドウごとに保持します。`ViewContext`・`PaintContext`・`EventContext`から同じ値を取得できます。寸法と座標はすべて論理座標で、DPIによる倍率とは独立しています。

## 操作寸法の選択

アプリは`InteractionMetrics::new(theme, environment.input_mode)`で共通寸法を選択できます。`control_size`を`ButtonSize`・`TextFieldSize`へ変換して使います。

```rust
let metrics = InteractionMetrics::new(&Theme::current(), context.environment().input_mode);
let action = Button::new("Open").size(metrics.control_size.into());
let search = TextField::new(search_binding).size(metrics.control_size.into());
```

タッチ向けの標準操作面は44論理座標単位です。行高、ツールバー高、グリッド間隔も同じ寸法から解決できます。既存のPointer向け寸法やコンポーネントの既定値は変更しません。ViewKitがアプリの構成を自動変換する仕組みではありません。

ランチャーは`VIEWKIT_INPUT_MODE=touch`または`pointer`で初期値を指定できます。Binderはタブレットの作業領域から起動する子アプリへTouchを渡します。実際のTouch開始イベントでも、そのウィンドウの環境をTouchへ更新します。これはUIの希望値であり、物理入力の証明やCapabilityを付与する情報ではありません。

## 安全領域とキーボード

`environment.safe_bounds(bounds)`は安全領域だけを除外します。`environment.content_bounds(bounds)`は、さらに下端全幅を覆うキーボードを除外します。安全領域とキーボードが重なる部分は二重に引きません。

フローティングキーボードでは画面全体を縮めません。`keyboard_overlap(bounds)`で重なる部分を調べ、アプリのスクロールなどへ反映できます。配置とヒット判定には必ず同じ矩形を使用してください。

プラットフォームは対象ウィンドウへ`PlatformEvent::EnvironmentChanged`を配送します。ランタイムはそのウィンドウだけを再構築・再描画し、他のウィンドウの環境は変更しません。通知がない場合は安全領域ゼロ・キーボード非表示です。

このAPIはソフトウェアキーボード自体を生成・表示するものではありません。キーボードを提供するバックエンドが実際の遮蔽矩形を通知するための受け口です。未接続の環境で架空のキーボード領域を設定しないでください。
