# 運用とビルド

## 利用

1. 単語帳に名前を付け、CSVを選ぶ。UTF-8/BOMまたはCP932を指定する。
2. 必須の問題・答えと、必要なら解説・ID・選択肢の列を割り当てる。問題と答えだけの2列CSVも使える。選択肢を1列にまとめている場合、その列にチェックを付ける。「選択肢セルの読み方」で、そのまま表示・改行で分ける・区切り文字の指定を選べる。選択肢を取り込まない場合は「選択肢を使わない」で選択を解除する。
3. 追加・更新・変更なし・統合された行・曖昧な行を確認して適用する。再取り込みも同じ手順を使う。
4. 学習でSpaceを押すと答えが開く。1=忘れた、2=難しい、3=普通、4=簡単。直前の評価は取り消せる。
5. 「学習設定」で通常の上限と保持率を変更し、「今日だけ上乗せ」で当日分を増やす。
6. 削除する単語帳を選び、「学習設定」→「単語帳を削除」を開く。確認画面の単語帳名とカード枚数を確認し、「この単語帳を削除する」で確定する。対象のカード・学習履歴・学習設定・当日上乗せをまとめて削除し、他の単語帳は保持する。削除は取り消せない。作成済みの暗号化バックアップや復元前の退避DBは別に管理され、この操作では削除しない。

問題文はプレーンテキストとして表示する。CSVやカードにHTMLを記入しても実行しない。照合でIDを使わない場合、問題文の空白や改行の違いも別カードとして扱う。CSVにないカードは保持する。先頭の見出し行が必須で、CSVは32 MiBまで、暗号化前の全体バックアップは128 MiBまで。行数自体の1,000行制限は設けていない。

IDなしでは同じ問題文の行を1枚にまとめる。IDありでは同じID・同じ問題文の行をまとめ、異なるIDは別カードにする。同一の答え・解説・選択肢は一度だけ残し、異なる答えや解説は空行で区切って保持する。プレビューの元の行番号は、見出しやセル内の改行も含むCSVファイル上の行番号。「異なる答え／解説をまとめています」と出た場合は、まとめた内容を確認してから適用する。同じIDに異なる問題文がある場合は両方の行番号を示して止める。

1列にまとめた選択肢は、既定の「セル全体をそのまま表示」で改行やカンマを保持する。改行で分ける場合はCRLF・LFに対応し、空行を省く。指定した区切り文字は文字列として扱い、正規表現やCSVのカンマ区切りとは別にセルの中を分割する。分割時は各選択肢の前後の空白を除く。複数列の場合はCSVの列順で取り込み、空セルを省く。再取り込みでは選択肢と読み方をもう一度確認する。選択肢の列を選ばない場合は選択肢なしとして更新し、変更前の内容をプレビューに示す。

同じCSVに選択肢のある問題とない問題を混在させられる。選択肢の列を選んでいても、その問題の選択肢セルがすべて空欄なら選択肢なしとして取り込む。空白だけのセルも省く。重複する問題をまとめる場合は、その問題の各行にある選択肢をまとめる。

選択肢は答えを開く前に表示され、「カード一覧」の編集から内容・見出し・追加・削除を変更できる。自己評価と学習履歴の扱いは選択肢のないカードと同じ。

「同じIDですが、問題文が異なります」と出た場合は、プレビュー上部の照合方法を確認する。初回は列名が`ID`なら列の候補にする。問題文でまとめたい場合は「IDを使わず問題文でプレビュー」で再確認する。この操作では選択肢など他の割り当てを保持し、データの適用は行わない。ID照合を続ける場合はCSVのIDを問題ごとに一意にする。表示される行番号はカード枚数ではなく、セル内の改行も含むCSV上の行番号。

再取り込みでは、前回適用した「IDを使わない」、選択肢を使わない設定、選択肢の区切り文字を引き継ぐ。列の位置は新しい見出しから選び直し、古い列番号をそのまま使わない。任意のID列を見出しから特定できない場合は手動で指定する。IDあり・なしが前回から変わる場合は注意文を表示する。IDの値は完全一致で、先頭ゼロや空白、大小文字を自動変換しない。

「忘れた」は1分後に再出題する。それ以外はFSRSによる間隔を日数に丸める。翌日以降の履歴が512件に達すると個人向け調整を利用できる。中断した計算は適用せず、完成した結果も明示操作で適用する。既存の復習予定日は維持する。予測は通常上限と既定の回答傾向を使い、当日の上乗せは含めない。

## 保存先とバックアップ

| OS | アプリのデータ領域 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\local.kotoba.desktop` |
| macOS | `~/Library/Application Support/local.kotoba.desktop` |
| Linux | `$XDG_DATA_HOME/local.kotoba.desktop`、未設定時は`~/.local/share/local.kotoba.desktop` |

教材・履歴・設定は`library.sqlite3`とSQLiteのWAL/SHMに保存する。端末内はOSの暗号化とログイン保護を使う。Unix系ではアプリ領域を所有者のみアクセス可能にし、DBとWAL/SHMを0600で作成する。元CSVの場所や選択しなかった列、パスフレーズをDBへ保存しない。

「設定とバックアップ」から全体をageのパスフレーズ方式で暗号化する。暗号化前のエクスポートファイルは作らず、保存先の一時ファイルも暗号化済み。復元時は全ファイルの復号・検証とプレビューを経て適用する。選択肢を含むバックアップ形式の版は2、FSRSは6.6.2に固定している。旧版1のDBとバックアップは、カードの識別子・履歴・復習状態を保ったまま、選択肢なしとして読み込む。更新後のDBと新版バックアップは旧アプリでは開けないため、復元先も選択肢対応版へ手動更新する。それ以外の非対応版は拒否する。

復元直前のDBは同じ端末の`restore-safety/before-*.sqlite3`へ退避する。退避に失敗すると復元を進めない。この退避は通常のローカルデータで、持ち出し用の暗号化バックアップではない。

退避した状態へ手動で戻す場合は、アプリを終了し、データ領域全体を別名に変更して保管する。元の場所に新しい空ディレクトリを作り、目的の退避DBを`library.sqlite3`という名前でコピーして起動する。新しいディレクトリへ旧WAL/SHMを混在させない。変更前のディレクトリを残すことで、この復旧操作も取り消せる。

アプリは通信、テレメトリー、クラウド同期、自動更新を実装していない。表示素材は同梱し、WebViewの外部接続はCSPで制限する。OSのバックアップ・同期設定やランタイム自体の挙動は、[実際の利用構成での検証](implementation-status.md)と併せて扱う。

## macOS

```sh
npm ci
npm run package:macos
```

ビルドしたCPU向けのAPPとDMGを`artifacts`へ作る。Intel Mac版が必要な場合はIntelのビルド・実行環境で検証する。スクリプトはTauriでAPPを作成し、`hdiutil`でDMGに収めるため、Finderの自動操作権限を必要としない。

配布先でOS保護の解除を前提にしない。開発者署名とApple公証は配布前の未完了項目であり、この作業でDeveloper IDによる署名・公証は行っていない。

## Windows 11・x86_64

MSVCのC++ビルドツール、Node.js 24、Rust 1.96.0を用意する。`prepare-webview2.ps1`は、Microsoftの公式配布からx64用の**Fixed Version WebView2 Runtime 152.0.4191.62**を取得し、SHA256・Microsoftの署名・実行ファイルの版を照合してから`src-tauri/runtime/webview2`へ展開する。取得先とSHA256は[`webview2-runtime.json`](../scripts/webview2-runtime.json)で固定している。アプリ利用時のダウンロード処理ではない。[Tauriの設定手順](https://v2.tauri.app/distribute/windows-installer/#fixed-version)、[Microsoftの配布手順](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#the-fixed-version-runtime-distribution-mode)。

```powershell
npm ci
./scripts/prepare-webview2.ps1
npm test
npm run build
cargo test --locked --workspace
if (!(Test-Path .\src-tauri\runtime\webview2\msedgewebview2.exe)) {
    throw 'Place the official x64 Fixed Version WebView2 Runtime in src-tauri/runtime/webview2 first.'
}
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis
```

Windows用設定は`tauri.windows.conf.json`で自動適用される。Fixed Versionの更新もアプリと一緒に手動配布する。更新時は公式配布物の版・ハッシュを変更し、既存の`src-tauri/runtime/webview2`を別名へ移してから取得スクリプトを再実行する。既存ディレクトリは自動上書きしない。署名用の資格情報はソースへ含めない。

## Ubuntu 24.04 LTS・x86_64（暫定）

```sh
sudo apt-get update
sudo apt-get install -y build-essential pkg-config libssl-dev libwebkit2gtk-4.1-dev libgtk-3-dev librsvg2-dev libayatana-appindicator3-dev patchelf
npm ci
npm test
npm run build
cargo test --locked --workspace
npm run tauri -- build --bundles deb
```

通常のCargo設定では成果物は`target/release/bundle/deb`に置かれる。独自のtarget-dirを設定している環境では`cargo metadata --no-deps --format-version 1`の`target_directory`を確認する。WebKitGTKとGTKはOSのパッケージで供給する。オフラインの利用端末へ導入する場合、必要なdeb依存パッケージも端末管理側で事前に用意する。

## 合成バックアップの相互復元

CIの`verify.yml`は各OSでコアとUIのテストを実行し、各OSが作った合成バックアップを残りのOSで復号・SQLite復元・再起動検証する。[実行結果](https://github.com/kent8192/ez_4nki/actions/workflows/verify.yml)を確認する。WindowsのホステッドランナーはWindows Serverであり、Windows 11の実機確認を代替しない。

```sh
cargo run --locked -p kotoba-core --example portability_fixture -- create synthetic.age
cargo run --locked -p kotoba-core --example portability_fixture -- check synthetic.age
```

この検証用プログラムは固定の架空データのみを作る。パスフレーズは公開の固定値であり、実データに使わない。通常アプリに検証用の平文エクスポート口やデバッグ用データ注入機能は設けていない。

## 配布物とネイティブ画面のCI

`packages.yml`を手動実行すると、3 OSの配布物とSHA256・ビルド情報をprivateリポジトリのActions成果物へ保存する。公開リリースや自動更新は行わない。Windows・Ubuntuでは作成したパッケージをランナーにインストールし、Windowsでは同梱ランタイムと同じ版のMicrosoft Edge WebDriver、Ubuntuでは`tauri-driver`とWebKitWebDriverから実際の画面を操作する。Windowsのセッションでは起動したWebView2の版も照合し、起動に失敗した場合はドライバーの詳細ログを保存する。

`native-windows.yml`では既存のパッケージ実行IDを指定し、保存済みインストーラのSHA256を照合して画面検証だけを実行できる。証跡にはインストーラを作成したコミットとハッシュを含める。検証スクリプト側のコミットと製品バイナリ側のコミットが異なる場合、その違いを検証結果に明記する。

Windowsの画面検証では、使い捨てCI VMの当該アプリに限ってループバックのデバッグ接続を有効にし、[Microsoftのattach方式](https://learn.microsoft.com/en-us/microsoft-edge/webview2/how-to/webdriver#step-4b-attaching-microsoft-edge-webdriver-to-a-running-webview2-app)を使う。ホステッドランナーは管理者権限で動作するため、[WebView2の管理者プロセス向け仕様](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/security#for-an-elevated-host-app-use-appropriate-override-flags)に従いHKLMのアプリ専用値を使う。権限と値の読み戻しを確認し、既存値があると拒否し、終了時に自分が作成した値だけを削除する。製品の設定ファイルへデバッグ用ポートは追加しない。

ネイティブ画面検証は、回答を隠す表示、プレーンテキスト、評価と取り消し、当日上乗せ、30日予測、忘却曲線、再起動後の保存を対象とする。`native_fixture`はCIの新しいデータ領域へ架空の2枚を用意し、既存ディレクトリがあると拒否する。製品に検証用のデータ注入コマンドは追加していない。OSのファイルダイアログを通じたCSV取り込み・バックアップ操作は、この自動検証の対象外。

Ubuntuでは`strace`でドライバーと子プロセスのネットワーク呼び出しを記録する。Windowsでは使い捨てCI VMでWFP接続監査を有効にし、インストール先のアプリと同梱ランタイムに一致する接続メタデータだけを記録する。観測後は監査設定を元へ戻し、Securityログ全体やパケット本文は成果物へ含めない。証跡が空の場合は成功扱いにしない。

観測範囲はCIの操作と対象プロセスに限る。実利用端末の補助サービスや、3 OSでの全操作の通信観測も完成条件に含める。CIでのアプリ操作用通信はループバックのみで、配布アプリがWebDriverを起動することはない。
