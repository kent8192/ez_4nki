use crate::*;
use std::io::{Read, Write};
use zeroize::Zeroizing;

pub const MAX_BACKUP_BYTES: usize = 128 * 1024 * 1024;

pub fn encode_backup(state: &Snapshot, passphrase: String) -> Result<Vec<u8>> {
    let passphrase = age::secrecy::SecretString::from(passphrase);
    validate_snapshot(state)?;
    let data = Zeroizing::new(serde_json::to_vec(state)?);
    if data.len() > MAX_BACKUP_BYTES {
        return Err(invalid("バックアップのサイズが上限を超えました。"));
    }
    let encryptor = age::Encryptor::with_user_passphrase(passphrase);
    let mut encrypted = vec![];
    let mut writer = encryptor
        .wrap_output(&mut encrypted)
        .map_err(|_| invalid("暗号化を開始できません。"))?;
    writer.write_all(&data)?;
    writer.finish()?;
    Ok(encrypted)
}
pub fn decode_backup(bytes: &[u8], passphrase: String) -> Result<Snapshot> {
    let passphrase = age::secrecy::SecretString::from(passphrase);
    if bytes.len() > MAX_BACKUP_BYTES + 1024 * 1024 {
        return Err(invalid("バックアップのサイズが上限を超えました。"));
    }
    let decryptor =
        age::Decryptor::new(bytes).map_err(|_| invalid("暗号化バックアップを読み取れません。"))?;
    if !decryptor.is_scrypt() {
        return Err(invalid(
            "パスフレーズ形式のバックアップを選択してください。",
        ));
    }
    let identity = age::scrypt::Identity::new(passphrase);
    let mut reader = decryptor
        .decrypt(std::iter::once(&identity as &dyn age::Identity))
        .map_err(|_| invalid("パスフレーズが違うか、バックアップが破損しています。"))?
        .take((MAX_BACKUP_BYTES + 1) as u64);
    let mut plaintext = Zeroizing::new(vec![]);
    reader
        .read_to_end(&mut plaintext)
        .map_err(|_| invalid("バックアップの完全性を確認できません。"))?;
    if plaintext.len() > MAX_BACKUP_BYTES {
        return Err(invalid("バックアップのサイズが上限を超えました。"));
    }
    let state: Snapshot = serde_json::from_slice(&plaintext)?;
    validate_snapshot(&state)?;
    Ok(state)
}
