# 運用とビルド

## 利用

1. 単語帳に名前を付け、CSVを選ぶ。UTF-8/BOMまたはCP932を指定する。
2. 問題・答え・解説と、必要ならIDの列を割り当てる。
3. 追加・更新・変更なし・曖昧な行を確認して適用する。再取り込みも同じ手順を使う。
4. 学習でSpaceを押すと答えが開く。1=忘れた、2=難しい、3=普通、4=簡単。直前の評価は取り消せる。
5. 「学習設定」で通常の上限と保持率を変更し、「今日だけ上乗せ」で当日分を増やす。

問題文はプレーンテキストとして表示する。CSVやカードにHTMLを記入しても実行しない。照合でIDを使わない場合、問題文の空白や改行の違いも別カードとして扱う。CSVにないカードは保持する。先頭の見出し行が必須で、CSVは32 MiBまで、暗号化前の全体バックアップは128 MiBまで。行数自体の1,000行制限は設けていない。

「忘れた」は1分後に再出題する。それ以外はFSRSによる間隔を日数に丸める。翌日以降の履歴が512件に達すると個人向け調整を利用できる。中断した計算は適用せず、完成した結果も明示操作で適用する。既存の復習予定日は維持する。予測は通常上限と既定の回答傾向を使い、当日の上乗せは含めない。

## 保存先とバックアップ

| OS | アプリのデータ領域 |
| --- | --- |
| Windows | `%LOCALAPPDATA%\local.kotoba.desktop` |
| macOS | `~/Library/Application Support/local.kotoba.desktop` |
| Linux | `$XDG_DATA_HOME/local.kotoba.desktop`、未設定時は`~/.local/share/local.kotoba.desktop` |

教材・履歴・設定は`library.sqlite3`とSQLiteのWAL/SHMに保存する。端末内はOSの暗号化とログイン保護を使う。Unix系ではアプリ領域を所有者のみアクセス可能にし、DBとWAL/SHMを0600で作成する。元CSVの場所や選択しなかった列、パスフレーズをDBへ保存しない。

「設定とバックアップ」から全体をageのパスフレーズ方式で暗号化する。暗号化前のエクスポートファイルは作らず、保存先の一時ファイルも暗号化済み。復元時は全ファイルの復号・検証とプレビューを経て適用する。バックアップ形式の版は1、FSRSは6.6.2に固定している。非対応の版を黙って読み替えない。

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

MSVCのC++ビルドツール、Node.js 24、Rust 1.96.0を用意する。Microsoftの公式配布からx64用の**Fixed Version WebView2 Runtime**を入手し、`msedgewebview2.exe`が`src-tauri/runtime/webview2`直下にある構成で展開する。展開時は`expand`を使い、配布物の版とハッシュを検証記録に残す。[Tauriの設定手順](https://v2.tauri.app/distribute/windows-installer/#fixed-version)、[Microsoftの配布手順](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution#the-fixed-version-runtime-distribution-mode)。

```powershell
npm ci
npm test
npm run build
cargo test --locked --workspace
if (!(Test-Path .\src-tauri\runtime\webview2\msedgewebview2.exe)) {
    throw 'Place the official x64 Fixed Version WebView2 Runtime in src-tauri/runtime/webview2 first.'
}
npm run tauri -- build --target x86_64-pc-windows-msvc --bundles nsis
```

Windows用設定は`tauri.windows.conf.json`で自動適用される。ランタイムの取得は開発時の手動操作であり、アプリの初回起動時にダウンロードする構成ではない。Fixed Versionの更新もアプリと一緒に手動配布する。署名用の資格情報はソースへ含めない。

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

CIの`verify.yml`は各OSでコアとUIのテストを実行し、各OSが作った合成バックアップを残りのOSで復号・SQLite復元・再起動検証する。GitHub上ではまだ実行していない。WindowsのホステッドランナーはWindows Serverであり、Windows 11の実機確認を代替しない。

```sh
cargo run --locked -p kotoba-core --example portability_fixture -- create synthetic.age
cargo run --locked -p kotoba-core --example portability_fixture -- check synthetic.age
```

この検証用プログラムは固定の架空データのみを作る。パスフレーズは公開の固定値であり、実データに使わない。通常アプリに検証用の平文エクスポート口やデバッグ用データ注入機能は設けていない。
