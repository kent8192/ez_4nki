# 単語帳アプリの先行事例

調査日: 2026-09-09

## 結論

先行事例はある。FSRS、CSVの列割り当て、3 OS上での端末内学習には既存実装があり、学習アプリで暗号化バックアップを提供する例も見つかった。今回確認した候補について、要求された使いやすさと、持ち出しを明示操作の暗号化バックアップに限定する条件を、そのまますべて満たすことまでは確認できていない。

調査は公式資料と、近い候補の公開ソースの確認。アプリの実行、通信観測、バックアップ復元試験、全体のセキュリティ監査は行っていない。「未確認」は「機能が存在しない」という意味ではない。

## 主な比較

| 事例 | 3 OS・FSRS・教材取り込み | 今回の要件との距離 | 参考になる点 |
| --- | --- | --- | --- |
| Anki | Windows/Linux/macOS対応。FSRS、任意のCSV列割り当て、プレビューがある | 問題・答え・解説を表現するフィールドと表示設定が必要。標準保存・バックアップから暗号化限定運用を保証する仕組みは今回未確認 | 列割り当て、再取り込み、履歴保持、学習設定 |
| Mochi | 3 OS、完全オフライン利用、FSRSへの切り替え。CSV列をテンプレートのフィールドまたはカードの面へ割り当てる | 標準バックアップは内容を取り出せるZIP形式。FSRSの個人向けパラメータ最適化は外部ツールを使う方式 | 問題・答え・解説の表示、教材編集の操作 |
| Recall（Madlezz/Recall） | 3 OSの配布物、FSRS、統計・曲線・復習量予測を公称。CSVプレビューがある | 確認したCSV実装は固定列順。手動バックアップの経路は平文JSON。暗号化同期機能の存在だけでは今回の持ち出し条件を満たさない | デスクトップUIとFSRSを組み合わせる実装構成 |
| Essentialist | 3 OS・FSRS・通信しない設計を公称。教材はMarkdown | 教材が平文Markdown。CSV列を選ぶ操作と暗号化バックアップは確認した資料に記載がない | 通信を必要としない小さな学習アプリ |
| Cortex（PndaMan/cortex） | 3 OSの導入手順、FSRS、暗号化バックアップを公称 | バックアップはageで暗号化してrcloneで転送する構成。外部AI等の連携と平文出力も含み、今回の3列CSV専用操作や持ち出し制限の適合は未確認 | 学習アプリに暗号化バックアップを組み込む先行例 |

### Ankiの根拠

- [公式配布ページ](https://apps.ankiweb.net/): Windows、macOS、Linux向け配布物を確認。
- [FSRS](https://docs.ankiweb.net/deck-options.html#fsrs): 目標保持率と復習履歴に基づく最適化を確認。
- [CSV取り込み](https://docs.ankiweb.net/importing/text-files.html): 任意の列割り当て、プレビュー、更新時のスケジュール保持を確認。
- [ファイル管理](https://docs.ankiweb.net/files.html)と[バックアップ](https://docs.ankiweb.net/backups.html): ローカル保存の構成とバックアップ機能を確認。公式の復旧説明は通常のsqlite3による内容の読み出しを扱っており、標準保存をアプリ側の暗号化保管庫として扱う根拠にはならない。

### Mochiの根拠

- [対応OSとオフライン利用](https://mochi.cards/docs/getting-started/download-and-install/)。
- [CSVとバックアップ形式](https://mochi.cards/docs/import-and-export/importing/): テンプレート使用時は列ヘッダーの名前またはIDでフィールドに対応し、未使用時は列がカードの面になる。標準の.mochiファイルはJSON等を含むZIPとして開ける。
- [FSRS](https://mochi.cards/docs/reviewing/fsrs/): FSRSへの切り替え、2段階評価、目標保持率とパラメータ入力を確認。組み込みの最適化機能はないと明記されている。
- [現行掲示のプライバシーポリシー](https://mochi.cards/privacy): アカウントなしの場合は内容を端末に保存すると説明する一方、アカウント利用時はデータベース内で暗号化しておらず機密情報を保存しないよう求めている。この文言を根拠に、アカウントなしの端末内保存方式まで断定しない。

### Recallの根拠とソース確認

[README](https://github.com/Madlezz/Recall)にはFSRS、CSV、統計、端末内保存、任意の暗号化同期が記載されている。[リリースv1.3.0](https://github.com/Madlezz/Recall/releases/tag/v1.3.0)についてGitHub APIでWindows用MSI、macOSのIntel/Apple Silicon用DMG、Linux用AppImageを確認した。公開日は2026-07-31。READMEの最新バージョン表記はv1.2.0のままであり、配布状況はリリースAPIを優先した。

以下のソース確認対象はmainのコミット `9e1b5131bfbfe61aa3183edc0ac90dda66d13ac7`。配布済みv1.3.0と同じコードであるとは確認していない。

- [CSVダイアログ](https://github.com/Madlezz/Recall/blob/9e1b5131bfbfe61aa3183edc0ac90dda66d13ac7/src/components/csv-import-dialog.tsx#L100): 先頭からfront、back、hint、tagsへ固定で割り当てる。任意の列を問題・答え・解説へ選択する今回の操作とは異なる。
- [バックアップ出力](https://github.com/Madlezz/Recall/blob/9e1b5131bfbfe61aa3183edc0ac90dda66d13ac7/src/services/native-files.ts#L4): バックアップ内容をJSON化し、暗号化を挟まずwriteTextFileで保存する経路を確認。
- [フォルダー同期](https://github.com/Madlezz/Recall/blob/9e1b5131bfbfe61aa3183edc0ac90dda66d13ac7/src/services/sync.ts): 同様にJSONをフォルダーへ保存する経路を確認。
- [DB接続と復元前バックアップ](https://github.com/Madlezz/Recall/blob/9e1b5131bfbfe61aa3183edc0ac90dda66d13ac7/src-tauri/src/db_atomic.rs#L336): 通常のSQLite接続とDBファイルのコピーを確認。保存データ全体を暗号化する構成はこの経路からは確認できない。

以上は要件比較のための限定的な確認であり、脆弱性認定や製品全体の監査結果ではない。

### EssentialistとCortexの根拠

- [Essentialist公式リポジトリ](https://github.com/essentialist-app/essentialist): 3 OS、FSRS、通信なし、平文Markdownの教材と別ファイルの学習状態を明記。実行時の通信有無は未検証。
- [Cortex公式リポジトリ](https://github.com/PndaMan/cortex): 3 OSの導入、FSRS、ageとrcloneによるバックアップ、AIへの接続とその他の出力機能を明記。暗号化処理や復元の実行検証は未実施。

## 補足として確認した事例

- [Remember](https://github.com/linustalacko/remember): 3 OS向けの端末内学習アプリだが、現行READMEではSM-2を採用しFSRSは未対応と明記。
- [lapse](https://github.com/elyxlz/lapse): FSRSとSQLiteの小さなアプリ。教材は外部で専用DBとして作り、編集機能を持たないと明記。今回のCSV選択操作の直接的な代替にはならない。
- [FlashMemo](https://flashmemo.org/): 検索結果にはFSRS・CSV・オフライン利用が記載されたが、ページ本文から詳細を取得できず、比較の根拠には採用していない。

## 再利用できる部品と採用状況

| 部品 | 確認した役割 | 状態 |
| --- | --- | --- |
| [fsrs-rs](https://github.com/open-spaced-repetition/fsrs-rs) | 復習間隔の計算、履歴からのパラメータ最適化、シミュレーション | FSRS方式・ライブラリともに初版の設計確認で採用済み |
| [SQLCipher](https://www.zetetic.net/sqlcipher/) | SQLite全体の暗号化と複数OSでの利用 | 調査後、端末内の保護をOSに委ねる方針が選ばれたため、初版の必須部品にはしない |
| [age](https://github.com/FiloSottile/age) | 公開鍵またはパスフレーズによるファイル暗号化 | パスフレーズ方式・全体復元・age形式ともに採用済み。Rustのageクレートを組み込む |

## 設計への示唆

学習方式やCSV列割り当てには既存の実績がある。今回具体化すべき差分は、問題・答え・解説へ迷わず割り当てられる操作、各利用者の学習履歴、機密データの保存と持ち出しの制御である。

既存のFSRS実装を使い、CSVの操作と保護要件に合わせた保存・復元を設計する方針とした。調査後の設計確認で、Tauri 2、React・TypeScript、Rust、SQLite、fsrs-rs、ageの構成を採用した。詳細は[初版の設計](product-design.md)に記録している。

## 調査後に確定した保護方針

利用者は端末内の保護をOSの暗号化とログイン保護に任せることを選択した。このため、アプリ独自の暗号化DBやロックを持たないことだけでは既存製品を要件不適合とはしない。比較で引き続き重視するのは、CSVの操作、採用済みのFSRS周辺4機能、学習履歴を保つ更新、暗号化バックアップと3 OS間の復元である。判断の根拠は[ADR-0002](adr/0002-rely-on-os-for-local-protection.md)に記録した。
