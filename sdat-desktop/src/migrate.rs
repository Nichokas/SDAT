//! One-time, idempotent migration from the pre-rename Tonearm identity
//! (`dev.dergs.Tonearm`, `~/.config/tonearm`) to SDAT (`com.nichokas.SDAT.Desktop`,
//! `~/.config/sdat-desktop`).
//!
//! Every step only reads the old data and writes values that are missing on the new
//! side, so re-running it never overwrites something the user changed after the
//! rename. Nothing old is deleted: the legacy settings, keyring entry and config
//! directory stay in place. A marker file is written only after every step succeeded,
//! so a failed run is retried on the next start.

use std::path::{Path, PathBuf};

use gtk::gio;
use gtk::gio::prelude::*;

use crate::settings::{APP_ID, CONFIG_DIR_NAME, LEGACY_APP_ID, LEGACY_CONFIG_DIR_NAME};

const MARKER: &str = ".migrated-from-tonearm";
const SUB_SCHEMAS: [&str; 6] = ["", ".playback", ".performance", ".player", ".scrobbling", ".service.tidal"];

/// Runs all steps against the real user session. Logs and continues on errors.
pub fn run() {
    let config = glib::user_config_dir();
    let new_dir = config.join(CONFIG_DIR_NAME);
    if new_dir.join(MARKER).exists() {
        return;
    }
    let mut ok = true;
    match copy_missing_tree(&config.join(LEGACY_CONFIG_DIR_NAME), &new_dir) {
        Ok(n) if n > 0 => log::info!("migrate: copied {n} files from ~/.config/{LEGACY_CONFIG_DIR_NAME}"),
        Ok(_) => {}
        Err(e) => {
            ok = false;
            log::error!("migrate: config dir copy failed error={e:#}");
        }
    }
    if let Some(source) = gio::SettingsSchemaSource::default() {
        for suffix in SUB_SCHEMAS {
            let (old_id, new_id) = (format!("{LEGACY_APP_ID}{suffix}"), format!("{APP_ID}{suffix}"));
            let (Some(old_schema), Some(new_schema)) = (source.lookup(&old_id, true), source.lookup(&new_id, true)) else {
                continue;
            };
            let old = gio::Settings::new_full(&old_schema, None::<&gio::SettingsBackend>, None);
            let new = gio::Settings::new_full(&new_schema, None::<&gio::SettingsBackend>, None);
            if let Err(e) = copy_missing_settings(&old, &new) {
                ok = false;
                log::error!("migrate: settings {old_id} failed error={e:#}");
            }
        }
        gio::Settings::sync();
    }
    if let Err(e) = copy_missing_secret(&Keyring, crate::secrets::REFRESH_TOKEN_KEY) {
        ok = false;
        log::error!("migrate: keyring copy failed error={e:#}");
    }
    if ok
        && let Err(e) = std::fs::create_dir_all(&new_dir).and_then(|()| std::fs::write(new_dir.join(MARKER), b"1\n"))
    {
        log::error!("migrate: could not write marker error={e}");
    }
}

/// Copies files from `old` into `new` that do not exist there yet. Returns how many were copied.
pub(crate) fn copy_missing_tree(old: &Path, new: &Path) -> std::io::Result<usize> {
    if !old.is_dir() {
        return Ok(0);
    }
    std::fs::create_dir_all(new)?;
    let mut copied = 0;
    for entry in std::fs::read_dir(old)? {
        let entry = entry?;
        let (from, to): (PathBuf, PathBuf) = (entry.path(), new.join(entry.file_name()));
        let kind = entry.file_type()?;
        if kind.is_dir() {
            copied += copy_missing_tree(&from, &to)?;
        } else if kind.is_file() && !to.exists() {
            // Write to a temp name and rename so an interrupted copy never leaves a partial file.
            let tmp = to.with_extension("migrating");
            std::fs::copy(&from, &tmp)?;
            std::fs::rename(&tmp, &to)?;
            copied += 1;
        }
    }
    Ok(copied)
}

/// Copies every key the user changed in `old` into `new`, unless `new` already has a user value.
pub(crate) fn copy_missing_settings(old: &gio::Settings, new: &gio::Settings) -> anyhow::Result<()> {
    let (Some(old_schema), Some(new_schema)) = (old.settings_schema(), new.settings_schema()) else {
        return Ok(());
    };
    for key in old_schema.list_keys() {
        if !new_schema.has_key(&key) || new.user_value(&key).is_some() {
            continue;
        }
        if let Some(v) = old.user_value(&key) {
            new.set_value(&key, &v).map_err(|e| anyhow::anyhow!("{key}: {e}"))?;
        }
    }
    Ok(())
}

pub(crate) trait SecretStore {
    fn get(&self, schema: &str, key: &str) -> anyhow::Result<Option<String>>;
    fn put(&self, schema: &str, key: &str, value: &str) -> anyhow::Result<()>;
}

struct Keyring;
impl SecretStore for Keyring {
    fn get(&self, schema: &str, key: &str) -> anyhow::Result<Option<String>> {
        Ok(crate::secrets::lookup_in(schema, key)?)
    }
    fn put(&self, schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
        Ok(crate::secrets::store_in(schema, key, "SDAT TIDAL Refresh Token", value)?)
    }
}

/// Copies the legacy secret to the new schema if the new one is empty, then reads it back.
pub(crate) fn copy_missing_secret(store: &impl SecretStore, key: &str) -> anyhow::Result<bool> {
    if store.get(APP_ID, key)?.is_some_and(|v| !v.is_empty()) {
        return Ok(false);
    }
    let Some(old) = store.get(LEGACY_APP_ID, key)?.filter(|v| !v.is_empty()) else {
        return Ok(false);
    };
    store.put(APP_ID, key, &old)?;
    anyhow::ensure!(store.get(APP_ID, key)?.as_deref() == Some(old.as_str()), "keyring read-back mismatch");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::RefCell;
    use std::collections::HashMap;

    fn tmp(name: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("sdat-migrate-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(&d).expect("tmp dir");
        d
    }

    #[test]
    fn tree_copy_keeps_old_and_never_overwrites() {
        let root = tmp("tree");
        let (old, new) = (root.join("tonearm"), root.join("sdat-desktop"));
        std::fs::create_dir_all(old.join("sub")).expect("mkdir");
        std::fs::write(old.join("a.json"), "old-a").expect("write");
        std::fs::write(old.join("sub/b"), "old-b").expect("write");
        std::fs::create_dir_all(&new).expect("mkdir");
        std::fs::write(new.join("a.json"), "new-a").expect("write");

        assert_eq!(copy_missing_tree(&old, &new).expect("copy"), 1);
        assert_eq!(std::fs::read_to_string(new.join("a.json")).expect("read"), "new-a");
        assert_eq!(std::fs::read_to_string(new.join("sub/b")).expect("read"), "old-b");
        assert!(old.join("a.json").exists() && old.join("sub/b").exists());
        assert_eq!(copy_missing_tree(&old, &new).expect("second run"), 0);
        assert_eq!(copy_missing_tree(&root.join("missing"), &new).expect("no old dir"), 0);
    }

    fn settings_pair(dir: &Path) -> (gio::Settings, gio::Settings) {
        let source = gio::SettingsSchemaSource::from_directory(env!("TONEARM_SCHEMA_DIR"), None, false).expect("schema dir");
        let backend = gio::keyfile_settings_backend_new(dir.join("kf").to_str().expect("utf8"), "/", None);
        let mk = |id: &str| {
            let schema = source.lookup(id, false).expect("schema");
            gio::Settings::new_full(&schema, Some(&backend), None)
        };
        (mk(&format!("{LEGACY_APP_ID}.player")), mk(&format!("{APP_ID}.player")))
    }

    #[test]
    fn settings_copy_only_user_values_and_is_idempotent() {
        let dir = tmp("gsettings");
        let (old, new) = settings_pair(&dir);
        let keys = old.settings_schema().expect("schema").list_keys();
        let (k1, k2) = (keys[0].to_string(), keys.get(1).map(ToString::to_string));
        // Flip k1 on the old side so it differs from the default.
        let v1 = old.default_value(&k1).expect("default");
        let changed = flip(&v1);
        old.set_value(&k1, &changed).expect("set old");
        copy_missing_settings(&old, &new).expect("migrate");
        assert_eq!(new.value(&k1), changed);
        if let Some(k2) = k2 {
            assert!(new.user_value(&k2).is_none(), "untouched keys stay at default");
        }
        // A value the user sets after migration is not overwritten by a re-run.
        new.reset(&k1);
        new.set_value(&k1, &v1).expect("set new");
        copy_missing_settings(&old, &new).expect("rerun");
        assert_eq!(new.value(&k1), v1);
        assert_eq!(old.value(&k1), changed, "old side untouched");
    }

    fn flip(v: &glib::Variant) -> glib::Variant {
        use glib::prelude::*;
        if let Some(b) = v.get::<bool>() {
            return (!b).to_variant();
        }
        if let Some(i) = v.get::<i32>() {
            return (i + 1).to_variant();
        }
        if let Some(d) = v.get::<f64>() {
            return (d / 2.0 + 0.25).to_variant();
        }
        if let Some(s) = v.get::<String>() {
            return format!("{s}x").to_variant();
        }
        panic!("unhandled default type {}", v.type_());
    }

    #[derive(Default)]
    struct Fake(RefCell<HashMap<(String, String), String>>);
    impl SecretStore for Fake {
        fn get(&self, schema: &str, key: &str) -> anyhow::Result<Option<String>> {
            Ok(self.0.borrow().get(&(schema.into(), key.into())).cloned())
        }
        fn put(&self, schema: &str, key: &str, value: &str) -> anyhow::Result<()> {
            self.0.borrow_mut().insert((schema.into(), key.into()), value.into());
            Ok(())
        }
    }

    #[test]
    fn secret_copied_once_and_old_kept() {
        let f = Fake::default();
        assert!(!copy_missing_secret(&f, "refresh_token").expect("empty"));
        f.put(LEGACY_APP_ID, "refresh_token", "old").expect("put");
        assert!(copy_missing_secret(&f, "refresh_token").expect("copy"));
        assert_eq!(f.get(APP_ID, "refresh_token").expect("get").as_deref(), Some("old"));
        assert_eq!(f.get(LEGACY_APP_ID, "refresh_token").expect("get").as_deref(), Some("old"));
        f.put(APP_ID, "refresh_token", "newer").expect("put");
        assert!(!copy_missing_secret(&f, "refresh_token").expect("rerun"));
        assert_eq!(f.get(APP_ID, "refresh_token").expect("get").as_deref(), Some("newer"));
    }
}
