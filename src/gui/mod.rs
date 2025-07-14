use std::error::Error;

use slint::ComponentHandle;

slint::include_modules!();

pub fn main() -> Result<(), Box<dyn Error>> {
    let ui = MainWindow::new()?;

    ui.on_request_value_increment({
        let ui_handle = ui.as_weak();
        move || {
            let ui = ui_handle.unwrap();
            ui.set_counter(ui.get_counter() + 1);
        }
    });

    ui.run()?;

    Ok(())
}
