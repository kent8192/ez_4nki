#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#[cfg(any(windows, test))]
mod offline_proxy;
mod service;
use kotoba_core::{MAX_BACKUP_BYTES, decode_backup, encode_backup, invalid, parse_csv};
use serde_json::{Value, json};
use service::{AppState, Command, RestoreCandidate, lock};
use std::io::{Read, Write};
use tauri::{Manager, State};
use zeroize::Zeroizing;

#[tauri::command]
async fn command(request: Command, state: State<'_, AppState>) -> Result<Value, String> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        state
            .handle(request, chrono::Utc::now().timestamp())
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|_| "操作を完了できませんでした。".to_string())?
}

#[tauri::command]
async fn pick_csv(encoding: String, state: State<'_, AppState>) -> Result<Value, String> {
    let file = rfd::AsyncFileDialog::new()
        .add_filter("CSV", &["csv", "txt"])
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(Value::Null);
    };
    let path = file.path().to_path_buf();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> kotoba_core::Result<Value> {
        let mut bytes = vec![];
        std::fs::File::open(path)?
            .take(32 * 1024 * 1024 + 1)
            .read_to_end(&mut bytes)?;
        let csv = parse_csv(&bytes, &encoding)?;
        let token = uuid::Uuid::new_v4().to_string();
        let output = json!({ "token": token, "headers": csv.headers, "rows": csv.rows });
        let mut cache = lock(&state.imports)?;
        cache.source = Some((token, csv));
        cache.preview = None;
        Ok(output)
    })
    .await
    .map_err(|_| "CSVを読み込めませんでした。".to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn export_backup(passphrase: String, state: State<'_, AppState>) -> Result<bool, String> {
    let mut passphrase = Zeroizing::new(passphrase);
    if passphrase.is_empty() {
        return Err("パスフレーズを入力してください。".into());
    }
    let file = rfd::AsyncFileDialog::new()
        .add_filter("暗号化バックアップ", &["age"])
        .set_file_name(format!(
            "kotoba-backup-{}.age",
            chrono::Local::now().format("%Y-%m-%d")
        ))
        .save_file()
        .await;
    let Some(file) = file else {
        return Ok(false);
    };
    let path = file.path().to_path_buf();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> kotoba_core::Result<bool> {
        let parent = path.parent().ok_or_else(|| invalid("保存先が不正です。"))?;
        if parent
            .canonicalize()?
            .starts_with(state.directory.canonicalize()?)
        {
            return Err(invalid(
                "アプリのデータ領域以外にバックアップを保存してください。",
            ));
        }
        let bytes = encode_backup(&state.library()?.state, std::mem::take(&mut *passphrase))?;
        let mut output = tempfile::NamedTempFile::new_in(parent)?;
        output.write_all(&bytes)?;
        output.as_file().sync_all()?;
        output.persist(&path).map_err(|e| e.error)?;
        Ok(true)
    })
    .await
    .map_err(|_| "バックアップを作成できませんでした。".to_string())?
    .map_err(|e| e.to_string())
}

#[tauri::command]
async fn preview_restore(passphrase: String, state: State<'_, AppState>) -> Result<Value, String> {
    let mut passphrase = Zeroizing::new(passphrase);
    if passphrase.is_empty() {
        return Err("パスフレーズを入力してください。".into());
    }
    *lock(&state.restore).map_err(|e| e.to_string())? = None;
    let file = rfd::AsyncFileDialog::new()
        .add_filter("暗号化バックアップ", &["age"])
        .pick_file()
        .await;
    let Some(file) = file else {
        return Ok(Value::Null);
    };
    let path = file.path().to_path_buf();
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || -> kotoba_core::Result<Value> {
        let mut bytes = vec![];
        std::fs::File::open(path)?.take((MAX_BACKUP_BYTES + 1024 * 1024 + 1) as u64).read_to_end(&mut bytes)?;
        let snapshot = decode_backup(&bytes, std::mem::take(&mut *passphrase))?;
        let token = uuid::Uuid::new_v4().to_string();
        let decks: Vec<_> = snapshot.decks.iter().map(|d| json!({ "name": d.name, "cards": snapshot.cards.iter().filter(|c| c.deck_id == d.id).count() })).collect();
        let output = json!({ "token": token, "decks": decks, "cards": snapshot.cards.len(), "reviews": snapshot.reviews.len() });
        let revision = state.library()?.state.revision;
        *lock(&state.restore)? = Some(RestoreCandidate { token, revision, snapshot });
        Ok(output)
    }).await.map_err(|_| "バックアップを確認できませんでした。".to_string())?.map_err(|e| e.to_string())
}

fn main() {
    let context = tauri::generate_context!();
    #[cfg(windows)]
    let (_offline_proxy, context) = {
        let proxy = offline_proxy::OfflineProxy::start()
            .expect("Could not start the offline WebView2 proxy");
        let mut context = context;
        for window in &mut context.config_mut().app.windows {
            window.proxy_url =
                Some(tauri::Url::parse(&proxy.url()).expect("Valid loopback proxy URL"));
        }
        (proxy, context)
    };
    tauri::Builder::default()
        .setup(|app| {
            app.manage(AppState::new(app.path().app_local_data_dir()?)?);
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            command,
            pick_csv,
            export_backup,
            preview_restore
        ])
        .run(context)
        .expect("Application startup failed");
}
