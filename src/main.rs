mod bridge;
mod engine;
mod viewmodel;

slint::include_modules!();

fn main() -> Result<(), slint::PlatformError> {
    let ui = AppWindow::new()?;
    bridge::bind(&ui);
    ui.run()
}
