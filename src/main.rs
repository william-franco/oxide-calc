mod bridge;
mod engine;
mod settings;
mod storage;
mod viewmodel;

slint::include_modules!();

use std::path::PathBuf;

/// `$XDG_DATA_HOME/oxide-calc/calc.db`, falling back to `~/.local/share`.
fn database_path() -> PathBuf {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".local/share")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join("oxide-calc").join("calc.db")
}

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    ui.set_app_version(env!("CARGO_PKG_VERSION").into());

    // If the database cannot be opened, the calculator still runs without history or saved settings.
    let storage = match storage::Storage::open(&database_path()) {
        Ok(storage) => {
            if let Err(e) = settings::import_legacy_file(&storage, &settings::legacy_settings_path()) {
                eprintln!("oxide-calc: não foi possível importar as configurações antigas: {e}");
            }
            Some(storage)
        }
        Err(e) => {
            eprintln!("oxide-calc: banco indisponível, o histórico não será salvo: {e}");
            None
        }
    };

    bridge::bind(&ui, storage);
    ui.run()
}
