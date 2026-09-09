# Kotoba — CSVから作る単語帳

CSVの列を問題・答え・解説に割り当て、FSRSで復習するデスクトップアプリ。名称は仮称。

2026-09-09の合意済み設計に基づく実装を含む。CSV差分取り込み、学習履歴の保持、4段階評価と取り消し、当日の枚数上乗せ、忘却曲線・復習予測・端末内での個人向け調整、暗号化バックアップを実装した。3 OSでの完成条件と実行済みの検証は[実装と検証の進捗](docs/implementation-status.md)を参照。

## 起動

Node.js 24、Rust 1.96.0、各OSの[Tauriビルド環境](https://v2.tauri.app/start/prerequisites/)を用意する。

```sh
npm ci
npm run desktop
```

開発時は端末内のViteサーバーを使う。配布ビルドは画面素材を同梱する。

macOSでのAPP・DMG作成:

```sh
npm run package:macos
```

成果物は `artifacts/Kotoba.app` と `artifacts/Kotoba_0.1.0_arm64.dmg`（Intelでビルドした場合は`x64`）。WindowsとUbuntuの手順、保存先、復旧方法は[運用とビルド](docs/operations.md)を参照。

## 検証

```sh
npm test
npm run build
npm run format:check
cargo fmt --all --check
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets -- -D warnings
```

テストは合成データと一時DBを使い、実際の単語帳には接続しない。`crates/core`が取り込み・学習・保存・暗号化、`src-tauri`がネイティブファイル操作とIPC、`src`がReact画面を担当する。依存関係はCargo.lockとpackage-lock.jsonで固定している。

## 資料

- [初版の設計](docs/product-design.md): 合意済みの機能、技術構成、データ保護、3 OSでの完成条件。
- [設計対話](docs/design-interview.md): 各質問の回答と設計全体への合意記録。
- [先行事例](docs/prior-art.md): 公式資料と公開ソースによる比較、採用した部品。
- [用語集](CONTEXT.md): 単語帳、カード、学習日、当日上乗せなどの定義。
- [架空のCSV例](docs/examples/README.md): 初回取り込み・再取り込み・曖昧な照合の例。

## 実装と検証に引き継ぐ条件

Windows 11・Intel系、Ubuntu系、macOSの3 OSで、実際に使う構成の動作と相互復元を確認する。Ubuntuの具体的な版・CPUは未定で、24.04 LTS・x86_64は暫定の検証基準。

端末内の保護はOSの暗号化とログイン保護に委ねる。アプリの処理は端末内で完結し、持ち出し機能は明示操作によるパスフレーズ暗号化バックアップとする。通信なし・手動更新を前提とする。
