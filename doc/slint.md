# Slint WASM 対応メモ

このプロジェクトで調査・解決した Slint の WASM 固有の挙動と対処法をまとめる。

---

## 1. クレート構成

### 問題

Slint の WASM は `cdylib` クレートタイプでのみ正しく動作する。
バイナリクレート（`[[bin]]` のみ）の場合、`fn main()` が `__wbindgen_start` になるが、
winit の JS 例外 throw（イベントループ転送）を wasm-bindgen が正しく処理できず、
アプリが起動しない（画面が空白）。

### 解決策

`Cargo.toml` で lib + bin ハイブリッド構成にする。

```toml
[lib]
crate-type = ["lib", "cdylib"]

[[bin]]
name = "winxmerge"
required-features = ["desktop"]  # trunk は desktop feature を渡さないので bin をスキップする

[features]
desktop = []
```

`src/lib.rs` を WASM のエントリポイントにする。

```rust
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

#[cfg_attr(target_arch = "wasm32", wasm_bindgen(start))]
pub fn main() {
    #[cfg(target_arch = "wasm32")]
    {
        console_error_panic_hook::set_once();
        wasm::run();
    }
}
```

`src/main.rs` はデスクトップ専用にし、WASM 向けのコードは一切書かない。

ネイティブのビルド・CI では `--features desktop` を付ける。

```bash
cargo build --features desktop
cargo test --features desktop
```

### 根拠

- Slint 公式ドキュメント（web.mdx）および todo サンプルがこのパターンを採用している
- `#[wasm_bindgen(start)]` は `cdylib` でのみ `__wbindgen_start` として正しくエクスポートされる

---

## 2. canvas の DOM 挿入

### 問題

winit 0.30 以降、canvas の自動 DOM 挿入がデフォルト無効（`append: false`）になった。
Slint の winit バックエンドは `document.getElementById("canvas")` で既存の canvas を探し、
見つかればそれを使う。見つからなければ canvas が DOM に追加されず、画面が空白になる。

### 解決策

`index.html` の `<body>` に `id="canvas"` の canvas 要素を置く。

```html
<canvas id="canvas"></canvas>
```

### 根拠

`i-slint-backend-winit/winitwindowadapter.rs` 618–626 行:

```rust
if let Some(html_canvas) = web_sys::window()
    ...
    .get_element_by_id("canvas")
    .and_then(|e| e.dyn_into::<web_sys::HtmlCanvasElement>().ok())
{
    .with_canvas(Some(html_canvas))
```

---

## 3. canvas のフルスクリーン対応

### 問題

Slint はウィンドウ表示時（`set_visibility(ShownFirstTime)`）に
`layout_info.preferred_bounded()` からウィンドウサイズを計算して上書きする。
この値は Slint の `.slint` ファイルのコンポーネントサイズ（小さい）に基づくため、
ブラウザのビューポートより小さく表示される。

### 解決に至るまでの失敗パターン

| 試みた方法 | 失敗の理由 |
|---|---|
| `Window { preferred-width: 100%; }` | Window のルート要素に親がなく `preferred_bounded()` が 0 になる |
| CSS `canvas { width: 100vw; height: 100vh; }` | `canvas_has_explicit_size_set()` が `true` になり Slint が `clientWidth` を無視する |
| JS `canvas.style.width = ...` | 同上（インラインスタイルも `canvas_has_explicit_size_set` を true にする） |
| JS `canvas.width = window.innerWidth` | `getComputedStyle(canvas).width` が `"auto"` でなくなるため同上 |
| `set_size()` を `run()` 前に呼ぶ | WASM では winit ウィンドウが非同期生成のため空振りする |
| `slint::spawn_local` で `set_size()` | `set_size()` → `resize_window()` → `request_inner_size()` は非同期のため即座に反映されない |
| `TrunkApplicationStarted` で `resize` dispatch | winit が JS 例外 throw で `init()` を抜けるため、このイベントは**発火しない** |

### 解決策

`MutationObserver` で canvas の `style` 属性変更を監視する。
Slint/winit が canvas の style を最初に設定した時点でイベントループが動いており、
Rust の `resize` リスナーが正しく `set_size()` を呼べる。

`index.html`:

```html
<canvas id="canvas"></canvas>
<script>
    var canvas = document.getElementById("canvas");
    var observer = new MutationObserver(function() {
        observer.disconnect();
        window.dispatchEvent(new Event("resize"));
    });
    observer.observe(canvas, { attributes: true, attributeFilter: ["style"] });
</script>
```

Rust 側（`src/wasm.rs`）でリサイズリスナーを登録しておく:

```rust
fn viewport_size() -> (u32, u32) {
    web_sys::window()
        .map(|w| {
            let width = w.inner_width().ok().and_then(|v| v.as_f64()).unwrap_or(800.0) as u32;
            let height = w.inner_height().ok().and_then(|v| v.as_f64()).unwrap_or(600.0) as u32;
            (width, height)
        })
        .unwrap_or((800, 600))
}

// run() の中で登録
let window_weak = window.as_weak();
let closure = Closure::<dyn FnMut()>::new(move || {
    if let Some(w) = window_weak.upgrade() {
        let (width, height) = viewport_size();
        w.window().set_size(slint::WindowSize::Logical(slint::LogicalSize::new(
            width as f32,
            height as f32,
        )));
    }
});
web_sys::window()
    .unwrap()
    .add_event_listener_with_callback("resize", closure.as_ref().unchecked_ref())
    .ok();
closure.forget();
```

`window.innerWidth/Height` は CSS 論理ピクセルなので `WindowSize::Logical` で渡す。

### 根拠

`i-slint-backend-winit/winitwindowadapter.rs` の `canvas_has_explicit_size_set()` の実装:

```rust
fn canvas_has_explicit_size_set(canvas: &web_sys::HtmlCanvasElement) -> bool {
    let style = canvas.style();
    if !style.get_property_value("width").unwrap_or_default().is_empty()
        || !style.get_property_value("height").unwrap_or_default().is_empty()
    {
        return true;
    }
    let computed = window.get_computed_style(&canvas)...;
    computed.get_property_value("width").ok().as_deref() != Some("auto")
        || computed.get_property_value("height").ok().as_deref() != Some("auto")
}
```

canvas に CSS や style 属性でサイズを付けると `true` になり、
Slint は `clientWidth/Height` を無視して自分の preferred_size で上書きする。

Slint が winit ウィンドウ表示後に canvas の `style` を書き込む処理（`set_visible(true)` 後）は
JS のイベントループ上で動くため、`MutationObserver` はその時点を確実に捕捉できる。

---

## 4. Slint WASM の依存関係

```toml
[target.'cfg(target_arch = "wasm32")'.dependencies]
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
web-sys = { version = "0.3", features = [
    "Document", "Element", "EventTarget",
    "HtmlInputElement", "File", "FileList", "Blob",
    "Event", "Window", "console",
] }
console_error_panic_hook = "0.1"
```

ビルドツールは `trunk`（wasm-pack ではない）。

```toml
# Trunk.toml は不要（index.html の <link data-trunk rel="rust"> で自動検出）
```

```html
<link data-trunk rel="rust" data-wasm-opt="z" />
```
