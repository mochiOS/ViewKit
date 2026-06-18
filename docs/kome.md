# Kome向けViewKit FFI

## 概要

ViewKit FFIは、Komeで記述された宣言的UIからViewKitの動的Viewツリーを構築するためのC ABIです。

Komeコンパイラは、次のようなUI記述を直接Rustの型へ変換するのではなく、ViewKit FFIの呼び出し列へ変換します。

```
VStack(
    spacing: .large,
    alignment: .center,
) {
    Text("Hello")

    Button(
        "increment",
        onClick: handle_click,
    )
}
.padding(24)
```

概念的には、次の呼び出しへloweringされます。

```
vk_tree_begin

vk_begin_padding
    vk_begin_vstack
        vk_push_text
        vk_push_button
    vk_end_node
vk_end_node

vk_tree_commit
```

ViewKit側では、これらの呼び出しから`ViewNode`ツリーを構築し、既存のViewKitコンポーネントへ変換します。

## アーキテクチャ

```
Komeソースコード
    ↓
komec
    ↓ UI式のlowering
ViewKit C ABI
    ↓
ViewTreeBuilder
    ↓
ViewNode
    ↓
ViewRuntime
    ↓
ViewAdapter
    ↓
ViewKitコンポーネント
    ↓
レイアウト・描画・イベント配送
```

ViewKitはKomeの構文、変数、`state`、関数本体を解釈しません。

ViewKitが扱うのは次の情報です。

```
NodeId
ComponentInstanceId
ActionId
Viewの種類
Viewのプロパティ
Viewの子要素
操作状態
Actionイベント
```

Kome側は次の処理を担当します。

```
Komeコードの構文解析
型検査
文字列補間
stateの保存
イベント関数の実行
state変更後のbody再評価
NodeIdとActionIdの生成
```

## ビルド設定

ViewKitをRustライブラリと共有ライブラリの両方として出力します。

`Cargo.toml`へ次を追加します。

```toml
[lib]
crate-type = [
    "rlib",
    "cdylib",
]
```

Linuxでは通常、次の共有ライブラリが生成されます。

```
target/debug/libviewkit.so
```

リリースビルドでは次です。

```bash
cargo build --release
```

```
target/release/libviewkit.so
```

## ABIの基本方針

FFI境界にはRust固有の型を公開しません。

次の型はABIへ直接出しません。

```
String
Vec<T>
Box<T>
Box<dyn View>
Result<T, E>
Rustのenum
Rustの参照
Rustのクロージャ
```

ABIでは固定幅整数、浮動小数点、ポインター、長さ、opaque handleのみを使用します。

## Runtime handle

Kome側は`VkRuntime`の内部構造へアクセスできません。

```rust
pub struct VkRuntime {
    // 非公開
}
```

Kome側ではopaque pointerとして保持します。

```c
typedef struct VkRuntime VkRuntime;
```

Runtimeは`vk_runtime_create`で生成し、不要になったら必ず`vk_runtime_destroy`で破棄します。

## 文字列

文字列はポインターとバイト長で渡します。

```rust
#[repr(C)]
pub struct VkString {
    pub pointer: *const u8,
    pub length: usize,
}
```

C側では次のように表現できます。

```c
typedef struct VkString {
    const uint8_t *pointer;
    size_t length;
} VkString;
```

文字列はUTF-8でなければなりません。

ViewKitはFFI関数の実行中に文字列をRustの`String`へコピーします。そのため、呼び出し終了後に呼び出し元が元のバッファを解放しても問題ありません。

空文字列は次のどちらでも表現できます。

```c
VkString {
    .pointer = NULL,
    .length = 0,
}
```

```c
VkString {
    .pointer = valid_pointer,
    .length = 0,
}
```

長さが0より大きい場合、`pointer`は有効なメモリーを指していなければなりません。

## ステータスコード

ほとんどのFFI関数は`i32`を返します。

`0`が成功、0以外が失敗です。

```rust
#[repr(i32)]
pub enum VkStatus {
    Ok = 0,

    NullPointer = 1,
    InvalidUtf8 = 2,

    BuilderAlreadyActive = 3,
    NoActiveBuilder = 4,

    NoOpenNode = 5,
    UnclosedNodes = 6,
    MultipleRoots = 7,
    MissingRoot = 8,

    InvalidEnumValue = 9,
    UnsupportedEvent = 10,

    Panic = 255,
}
```

### Ok

```
0
```

処理が成功しました。

### NullPointer

```
1
```

必要なポインターへ`NULL`が渡されました。

### InvalidUtf8

```
2
```

`VkString`の内容が正しいUTF-8ではありません。

### BuilderAlreadyActive

```
3
```

既にViewツリーを構築中であるにもかかわらず、再度`vk_tree_begin`が呼ばれました。

### NoActiveBuilder

```
4
```

`vk_tree_begin`を呼ばずにNode追加関数などが呼ばれました。

### NoOpenNode

```
5
```

開いているコンテナNodeがない状態で`vk_end_node`が呼ばれました。

### UnclosedNodes

```
6
```

閉じられていないコンテナNodeが残っています。

### MultipleRoots

```
7
```

複数のルートNodeが構築されました。

### MissingRoot

```
8
```

ViewツリーにルートNodeがありません。

### InvalidEnumValue

```
9
```

定義されていない列挙値が渡されました。

### UnsupportedEvent

```
10
```

現在のFFIでは変換できないイベントがActionキューに含まれていました。

### Panic

```
255
```

FFI関数内部でRustのpanicが発生しました。

panicはC ABI境界を越えず、ステータスコードへ変換されます。

## Runtimeの生成

```rust
pub extern "C" fn vk_runtime_create(
    component_instance_id: u64,
) -> *mut VkRuntime;
```

新しいViewKit Runtimeを生成します。

### 引数

`component_instance_id`は、Kome Runtimeが管理するcomponent instanceの識別子です。

同じcomponent定義から複数のインスタンスを生成する場合、それぞれ異なるIDを指定します。

### 戻り値

成功した場合は有効な`VkRuntime`へのポインターを返します。

失敗またはpanic発生時は`NULL`を返します。

### 使用例

```c
VkRuntime *runtime =
    vk_runtime_create(1);

if (runtime == NULL) {
    /* Runtime生成失敗 */
}
```

## Runtimeの破棄

```rust
pub extern "C" fn vk_runtime_destroy(
    runtime: *mut VkRuntime,
) -> i32;
```

Runtimeと、それが所有するViewツリーおよび操作状態を破棄します。

同じポインターへ複数回`vk_runtime_destroy`を呼んではいけません。

`NULL`を渡した場合は何もせず成功します。

## Viewツリー構築の開始

```rust
pub extern "C" fn vk_tree_begin(
    runtime: *mut VkRuntime,
    root_node_id: u64,
) -> i32;
```

新しいViewツリーの構築を開始します。

この関数は内部的に`Root` Nodeを生成します。

呼び出し元がRootに対して`vk_end_node`を呼ぶ必要はありません。Rootは`vk_tree_commit`が自動的に閉じます。

### 例

```c
int32_t status =
    vk_tree_begin(
        runtime,
        100
    );
```

## Viewツリー構築の中止

```rust
pub extern "C" fn vk_tree_abort(
    runtime: *mut VkRuntime,
) -> i32;
```

現在構築中のViewツリーを破棄します。

既にcommitされているViewツリーには影響しません。

構築途中でKome側の評価エラーが発生した場合に使用します。

```c
vk_tree_begin(runtime, 100);

if (evaluation_failed) {
    vk_tree_abort(runtime);
}
```

## VStackの開始

```rust
pub extern "C" fn vk_begin_vstack(
    runtime: *mut VkRuntime,
    node_id: u64,
    gap: u32,
    alignment: u32,
    distribution: u32,
) -> i32;
```

縦方向のStack Nodeを開始します。

`VStack`はコンテナNodeなので、子要素を追加した後に`vk_end_node`を呼ぶ必要があります。

### gap

```
0 = None
1 = ExtraSmall
2 = Small
3 = Medium
4 = Large
5 = ExtraLarge
6 = DoubleExtraLarge
```

### alignment

```
0 = Start
1 = Center
2 = End
3 = Stretch
```

### distribution

```
0 = Start
1 = Center
2 = End
3 = SpaceBetween
4 = SpaceAround
5 = SpaceEvenly
```

### 例

```c
vk_begin_vstack(
    runtime,
    102,
    VK_STACK_GAP_LARGE,
    VK_ALIGNMENT_CENTER,
    VK_DISTRIBUTION_CENTER
);

/* 子Nodeを追加 */

vk_end_node(runtime);
```

## Textの追加

```rust
pub extern "C" fn vk_push_text(
    runtime: *mut VkRuntime,
    node_id: u64,
    content: VkString,
    font_size: f32,
    line_height: f32,
    weight: u16,
    alignment: u32,
    color: u32,
) -> i32;
```

Text Nodeを現在開いているコンテナへ追加します。

Textはleaf Nodeなので、`vk_end_node`は必要ありません。

### alignment

```
0 = Start
1 = Center
2 = End
3 = Justified
```

### color

現在は次の色をサポートします。

```
0 = Black
1 = White
```

### 値の補正

`font_size`が無限大、NaN、0以下の場合は`16.0`を使用します。

`line_height`が無限大、NaN、0以下の場合は`24.0`を使用します。

### 例

```c
const char text[] =
    "count: 0";

VkString value = {
    .pointer =
        (const uint8_t *)text,

    .length =
        sizeof(text) - 1,
};

vk_push_text(
    runtime,
    103,
    value,
    18.0f,
    28.0f,
    600,
    VK_TEXT_ALIGNMENT_START,
    VK_TEXT_COLOR_BLACK
);
```

## Buttonの追加

```rust
pub extern "C" fn vk_push_button(
    runtime: *mut VkRuntime,
    node_id: u64,
    title: VkString,
    color: u32,
    radius: f32,
    action_id: u64,
) -> i32;
```

Button Nodeを現在開いているコンテナへ追加します。

Buttonはleaf Nodeなので、`vk_end_node`は必要ありません。

### color

```
0 = Accent
1 = Destructive
```

### radius

`radius`は`0.0`から`1.0`の比率で指定します。

```
0.0 = 角丸なし
0.5 = 50%
1.0 = 100%
```

範囲外の値は`0.0`から`1.0`へclampされます。

### action_id

クリック時にKome側で実行する関数を識別します。

```
0 = Actionなし
1以上 = Kome Runtimeが割り当てたActionId
```

例えば、

```kome
Button(
    "increment",
    onClick: handle_click,
)
```

は、概念的に次へ変換されます。

```c
vk_push_button(
    runtime,
    button_node_id,
    title,
    VK_BUTTON_COLOR_ACCENT,
    0.5f,
    handle_click_action_id
);
```

## Paddingの開始

```rust
pub extern "C" fn vk_begin_padding(
    runtime: *mut VkRuntime,
    node_id: u64,
    top: f32,
    right: f32,
    bottom: f32,
    left: f32,
) -> i32;
```

子Viewの周囲へ余白を追加するPadding Nodeを開始します。

PaddingはコンテナNodeなので、子要素を追加した後に`vk_end_node`を呼びます。

負数、無限大、NaNは`0.0`へ補正されます。

### 例

```c
vk_begin_padding(
    runtime,
    101,
    24.0f,
    24.0f,
    24.0f,
    24.0f
);

vk_begin_vstack(
    runtime,
    102,
    VK_STACK_GAP_LARGE,
    VK_ALIGNMENT_CENTER,
    VK_DISTRIBUTION_CENTER
);

/* 子Node */

vk_end_node(runtime); /* VStack */
vk_end_node(runtime); /* Padding */
```

## コンテナNodeの終了

```rust
pub extern "C" fn vk_end_node(
    runtime: *mut VkRuntime,
) -> i32;
```

現在開いているコンテナNodeを閉じ、親Nodeへ追加します。

対象になるNodeの例は次です。

```
VStack
Padding
将来のHStack
将来のZStack
将来のScroll
```

TextやButtonのようなleaf Nodeには使用しません。

## Viewツリーのcommit

```rust
pub extern "C" fn vk_tree_commit(
    runtime: *mut VkRuntime,
) -> i32;
```

構築したViewツリーをRuntimeへcommitします。

commitに成功すると、以前のViewツリーは新しいViewツリーへ置き換えられます。

同じ`NodeId`を持つViewの操作状態は維持されます。

維持対象の例は次です。

```
Buttonのhover状態
Buttonのpressed状態
Scrollのoffset
TextInputのカーソル位置
Pointer Capture
Focus
```

現在の実装ではButtonとScrollの状態を`NodeId`に基づいて保存します。

### Rootの扱い

`vk_tree_begin`で生成されたRoot Nodeは`vk_tree_commit`が自動的に閉じます。

したがって、次の呼び出しが正しい形です。

```
vk_tree_begin
    vk_begin_padding
        vk_begin_vstack
            vk_push_text
            vk_push_button
        vk_end_node
    vk_end_node
vk_tree_commit
```

次のようにRootまで手動で閉じてはいけません。

```
vk_tree_begin
    ...
vk_end_node
vk_tree_commit
```

## Actionの収集

```rust
pub extern "C" fn vk_runtime_collect_actions(
    runtime: *mut VkRuntime,
) -> i32;
```

ViewKitコンポーネントの操作状態を確認し、発生したActionをRuntimeのActionキューへ追加します。

通常はViewKitのイベント配送後に呼びます。

```
PlatformEventをViewツリーへ配送
    ↓
Buttonがclicked状態になる
    ↓
vk_runtime_collect_actions
    ↓
ActionIdをキューへ追加
```

## Actionの取得

```rust
pub extern "C" fn vk_poll_action(
    runtime: *mut VkRuntime,
    output: *mut VkActionEvent,
    has_action: *mut u8,
) -> i32;
```

Actionキューから先頭のイベントを一つ取得します。

ActionはFIFO順で返されます。

### VkActionEvent

```rust
#[repr(C)]
pub struct VkActionEvent {
    pub component_instance_id: u64,
    pub node_id: u64,
    pub action_id: u64,
    pub event_kind: u32,
}
```

### event_kind

現在は次のイベントをサポートします。

```
1 = ButtonClicked
```

### has_action

Actionが存在したかどうかを返します。

```
0 = Actionなし
1 = Actionあり
```

Actionがない場合でも、関数自体は成功として`VkStatus::Ok`を返します。

### 例

```c
VkActionEvent event;
uint8_t has_action = 0;

int32_t status =
    vk_poll_action(
        runtime,
        &event,
        &has_action
    );

if (
    status == VK_STATUS_OK
    && has_action != 0
) {
    kome_call_action(
        event.component_instance_id,
        event.action_id,
        event.event_kind
    );
}
```

すべてのActionを処理する場合は、キューが空になるまで繰り返します。

```c
for (;;) {
    VkActionEvent event;
    uint8_t has_action = 0;

    int32_t status =
        vk_poll_action(
            runtime,
            &event,
            &has_action
        );

    if (status != VK_STATUS_OK) {
        break;
    }

    if (has_action == 0) {
        break;
    }

    kome_call_action(
        event.component_instance_id,
        event.action_id,
        event.event_kind
    );
}
```

## 完全な構築例

次のKomeコードを想定します。

```kome
@application
component App {
    state counter = 0

    @body
    recipe body {
        VStack(
            spacing: .large,
            alignment: .center,
        ) {
            Text(
                "count: {counter}",
            )

            Button(
                "increment",
                onClick: handle_click,
            )
        }
        .padding(24)
    }

    fn handle_click(
        event: ButtonEvent,
    ) {
        counter = counter + 1
    }
}
```

Komeコンパイラは概念的に次の呼び出しを生成します。

```c
VkRuntime *runtime =
    vk_runtime_create(
        app_component_instance_id
    );

vk_tree_begin(
    runtime,
    root_node_id
);

vk_begin_padding(
    runtime,
    padding_node_id,
    24.0f,
    24.0f,
    24.0f,
    24.0f
);

vk_begin_vstack(
    runtime,
    stack_node_id,
    VK_STACK_GAP_LARGE,
    VK_ALIGNMENT_CENTER,
    VK_DISTRIBUTION_START
);

vk_push_text(
    runtime,
    counter_text_node_id,
    counter_text,
    16.0f,
    24.0f,
    400,
    VK_TEXT_ALIGNMENT_START,
    VK_TEXT_COLOR_BLACK
);

vk_push_button(
    runtime,
    increment_button_node_id,
    increment_title,
    VK_BUTTON_COLOR_ACCENT,
    0.5f,
    handle_click_action_id
);

vk_end_node(runtime); /* VStack */
vk_end_node(runtime); /* Padding */

vk_tree_commit(runtime);
```

Buttonがクリックされた後は次のように処理します。

```
ViewKitがクリックを検出
    ↓
vk_runtime_collect_actions
    ↓
vk_poll_action
    ↓
handle_clickのActionIdを取得
    ↓
Kome Runtimeがhandle_clickを実行
    ↓
counterを書き換える
    ↓
Appをdirtyにする
    ↓
bodyを再評価
    ↓
新しいViewツリーをcommit
```

## NodeId

すべてのView Nodeには安定した`NodeId`が必要です。

NodeIdはKomeコンパイラが生成します。

推奨構成は次です。

```
ComponentInstanceId
+ body recipeの識別子
+ AST上のNode番号
+ ForEachのkey
```

同じUI要素には、bodyを再評価した後も同じNodeIdを割り当てます。

```kome
VStack {
    Text("count: {counter}")
    Button("increment")
}
```

例えば次のように割り当てます。

```
100 = Root
101 = VStack
102 = Text
103 = Button
```

`counter`が変化しても、TextとButtonのNodeIdは変更しません。

NodeIdが変化すると、ViewKitは別のViewとして扱います。

## ActionId

Komeのイベント関数は`ActionId`へ変換します。

```kome
onClick: handle_click
```

```
handle_click → ActionId(200)
```

ViewKitは`handle_click`のコードや関数ポインターを保持しません。

ViewKitが返すのはActionIdだけです。

Kome RuntimeはActionIdから実際の関数を解決します。

## メモリー管理

### Runtime

`vk_runtime_create`が返したポインターは`vk_runtime_destroy`で解放します。

```
create 1回
destroy 1回
```

破棄後のポインターを再利用してはいけません。

### 文字列

入力文字列はFFI呼び出し中だけ有効であれば十分です。

ViewKitは必要な文字列を内部へコピーします。

### 出力イベント

`VkActionEvent`は呼び出し元が確保します。

ViewKitは`output`が指す領域へ値を書き込みます。

## スレッド安全性

同じ`VkRuntime`へ複数のスレッドから同時にアクセスしてはいけません。

現在のRuntimeは、UIイベントループと同じスレッドから操作する前提です。

```
1 Runtime
1 UI thread
```

複数のcomponent instanceを扱う場合は、Runtimeを分けるか、将来的な複数component対応Runtimeを使用します。

## panic安全性

FFI関数内部では`catch_unwind`を使用し、RustのpanicがC ABI境界を越えないようにします。

通常の関数ではpanic発生時に`VkStatus::Panic`を返します。

`vk_runtime_create`ではpanic発生時に`NULL`を返します。

ただし、プロセスがabortする設定の場合や、メモリー破壊などの未定義動作は回復できません。

## 現在の制限

現在のFFIは、Viewツリー構築とAction取得の最小実装です。

対応済みのNodeは次です。

```
Root
VStack
Text
Button
Padding
```

現在未対応の機能は次です。

```
HStack
ZStack
Background
Rectangle
Scroll
TextInput
List
ForEach
Focus
キーボード入力
IME
Pointer CaptureのFFI統合
ウィンドウ起動
アプリケーションイベントループ
複数componentの親子関係
```

## 今後追加するAPI

```
vk_begin_hstack
vk_begin_zstack
vk_begin_background
vk_push_rectangle
vk_begin_scroll

vk_application_create
vk_application_run
vk_application_request_redraw

TextInputイベント
Focusイベント
Scrollイベント
文字列を含むAction payload
```

ウィンドウ起動APIを追加した後は、Komeの`@application`componentからViewKitのPlatform Backendを起動できるようになります。
