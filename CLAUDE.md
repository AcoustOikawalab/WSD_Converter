# WSD_Converter

1ビットオーディオのRAWデータおよびDSDファイルをWSD形式へ変換する、ブラウザ完結型のツール。

## コミット規約

- コミットメッセージに `Co-Authored-By` トレーラーを付けない
- PR本文に生成ツールの表記を入れない

## 構成上の注意

- **本番配信にビルドは不要。** GitHub Pages はリポジトリのルートをそのまま配信している
  (`source_path=/`)。ルートの `index.html` / `script.js` / `style.css` が直接読まれる。
- `script.js` の import は相対パスのみで bare specifier が無いため、ブラウザがネイティブに
  解決できる。Vite は `npm run dev` の開発サーバー専用。
- **`wsd_logic/pkg/` は追跡対象。** `script.js` が `./wsd_logic/pkg/wsd_logic.js` を直接
  import しているため、`.gitignore` に入れてはいけない。Rust環境が無くても動くようにするため。
- `wsd_logic/target/` は `.gitignore` 済み。以前 176ファイル・94MB がコミットされていた。

## CSS の画像パス

`style.css` の画像参照は必ず `url("./images/...")` と相対パスで書く。
Pages はサブパス `/WSD_Converter/` 配下で配信されるため、`/images/...` と絶対パスで書くと
404 になる。

## 開発

```bash
npm install
npm run dev
```

Rust側を変更した場合は `wsd_logic/` で `wasm-pack build --target web` を実行する。
