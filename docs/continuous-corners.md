# Continuous Rounded Corner

## 定義と半径

固定弧長で `∫(dκ/ds)² ds` を最小化するモデルを使用する。
`κ = 3πs(L−s)/L³`、`θ(t) = π(3t²−2t³)/2`、`L = 1.6525000895846302 r`。
無制限の弧長に対する最適性は主張しない。半径は直角の頂点からの水平・垂直セットバックであり、既存トークンの値は変更しない。

実装の正本は syscalls の `viewkit-gpu-protocol/src/continuous_corner.rs`。
正規化位置と接線の1025点をオフラインの倍精度Simpson積分で生成する。
実行時には積分も三角関数も使わない。Circle/Ellipse、VK Symbolsやアプリアイコンの内部形状は変更しない。

半径配列は `[top_left, top_right, bottom_right, bottom_left]`。
隣接半径の和と各辺の比から一つの縮小係数を求め、全半径へ適用する。
負数・NaN・無限大の半径は0、非有限・非正の矩形は空として扱う。

## 近似と境界

CPUは三次Hermite補間をtiny-skiaのcubic_toへ渡す。
最初と最後に1/1024長の短区間を設け、その端の制御点を直線上に置き、直線との接続曲率を0にする。
その他の区間も端点位置・接線方向を一致させる。接続点の曲率には近似誤差がある。

| 主分割数 | 実測最大位置誤差 / r |
| --- | --- |
| 8 | 0.00003615 |
| 16 | 0.000002343 |
| 32 | 0.000000306 |
| 64 | 0.000000255 |

この誤差にはf32評価誤差を含む。CPUは物理半径と保守的な誤差上限から分割数を選択する。
数学テストは接線、G2端点、主区間の接続曲率誤差（正規化曲率で0.035未満）、対称性、90度回転を確認する。

GPUは同じ表を使う凸ポリゴン。最大曲率から弦誤差を制限し、0.08物理px以内となる分割数を選ぶ。
大きさに応じ8〜1024分割とし、実測精度は物理半径32768pxまで検査する。
ヒットテストは1024分割の境界逆引きを使用する。ほぼ垂直な端では対称性を使って精度低下を避ける。
GPUのクリップ許容誤差は外積の固定値ではなく辺長に比例させ、小さい角が漏れないようにする。

StrokeRoundedRectは従来どおり中心ストローク。
Rectangleの内側ボーダーはStrokeInsideRoundedRectを使用し、半径を減らして別の曲線を作る処理を廃止した。
内側境界とAAリングは基準曲線の法線でオフセットする。塗りと内側ボーダーの外周は同一パス。
シャドウの各既存レイヤーも同じ曲線モデルを使い、広がり・透明度・位置は維持する。

## APIと描画経路

既存のFillRoundedRect、StrokeRoundedRect、PushRoundedClip、CornerRadiusの指定は自動で移行する。
四隅別にはFillRoundedCornersRect、StrokeInsideRoundedCornersRect、PushRoundedCornersClipを使用できる。
`continuous_corner::shape` / `path` / `polygon` も半径配列を受け取る。
描画命令の既存シリアライズ番号は維持し、新しい命令は末尾へ追加する。

| 実装箇所 | 移行内容 |
| --- | --- |
| ViewKit platform/mochios/renderer.rs | 塗り・中心ストローク・内側ボーダー・クリップを共通CPUパスへ |
| ViewKit platform/mochios/gpu_renderer.rs | 塗り・枠線・AA・クリップを共有データへ。Linux GPUもこの実装を使用 |
| ViewKit gpu_clip.rs | 共通ヒット判定と四隅別クリップ |
| ViewKit components/rectangle.rs | ボタン・カード・入力欄・メニュー・タブ・スライダー・スイッチ等の共通描画入口 |
| ViewKit components/card.rs / surface.rs | 内容の角丸クリップ、外周入力判定 |
| ViewKit components/navigation_layout.rs | 円弧SVGによる凹角を共通曲線の補形へ |
| ViewKit omochi.rs | 変形前の円弧サンプリングを共通曲線へ。物理応答・変形・緩和は維持 |
| ViewKit button / text_field / switch / slider | 丸角の入力判定を統一。スライダー等の意図した操作用余白は維持 |
| Binder window / popup / panels / notifications | ウィンドウやパネルの外形と入力判定、アニメーションの逆座標変換 |
| Binder Dock / launcher | Rectangle経由の共通描画。アイコン・半径・位置・倍率は維持 |
| AppCore | ViewKit部品・描画命令へ委譲しているため独立した角丸生成は不要 |
| compositor geometry / renderer / gpu_compositor | 入力・ウィンドウクリップ・装飾・影を同じ表へ。入力は物理ピクセル中心 |
| BootUI draw.rs | 低層の角丸矩形も移行。円とスピナーの円弧は維持 |

BootUIはファームウェア用で依存を持たないため、正本と同一の幾何コード・表・数学テストを生成時にコピーする。
ルートの検証スクリプトがバイト一致を検査する。別モデルの手書き実装ではない。
生成は `python3 user/crates/viewkit-gpu-protocol/tools/generate_continuous_corner.py --bootui libraries/bootui/src`。
調査対象のローカルリポジトリはすべてアクセス可能だった。
Windows専用GPU実装は従来から角丸命令が未対応で、今回新しい描画エンジンを追加していない。

## キャッシュと性能

ViewKitのCPUパスとGPUポリゴンはそれぞれスレッドごと最大64件のLRU。
キーは矩形・正規化半径・分割精度・オフセット。DPIは必要分割数を通じて反映する。
GPUポリゴンはローカル座標で保存し、ウィンドウ移動時は座標を平行移動する。
compositorもサイズ・半径による最大64件のローカルポリゴンを保持する。
共有表は32800バイト。GPU座標キャッシュは最大約2.1MB/スレッド。
シェーダーや既存のバッチ処理、アトラスへの転送方式は変更していない。

ホストtestビルドで50000回、100×30/r9の形状生成を比較した例：
従来の48頂点の円弧生成21.16ms、新しい52頂点のキャッシュ利用13.68ms。
単発のCPU形状処理測定であり、GPUフレーム時間・OS本体の描画時間の改善を保証する数値ではない。
精度のため大半径・高DPIでは頂点が増える。旧固定48頂点との頂点数を無条件に同じにはしていない。

## 検証

- `python3 scripts/dev/check-continuous-corners.py`：共有コード一致、protocolとBootUIの数学テスト
- ViewKit単体テスト：CPU塗りとマスクの一致、内側ボーダーの外周、四隅別GPUクリップ、DPI、キャッシュ上限、既存部品
- compositorのcompositorテストターゲット：ウィンドウクリップ、入力、影のサンプル一致、奇数サイズの最大半径、既存描画キャッシュ
- Binder単体テスト：ウィンドウ、アニメーション座標、メニュー、通知等
- `mmake ui-check`：Linuxプレビューと標準アプリのビルド

この環境での本体ビルドは、角丸処理より前のnewlibの `sys/wait.h` 不足で停止した。
ネイティブcargo checkも既存sysrootのstdとコンパイラの `panic_null_reference_constructed` lang item不整合で停止した。
Linux検証の成功を本体フルビルドの成功として扱わない。
