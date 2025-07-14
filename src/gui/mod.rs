use std::{error::Error, rc::Rc};

use slint::{ComponentHandle, ModelRc, VecModel};

slint::include_modules!();

pub fn main() -> Result<(), Box<dyn Error>> {
    let ui = MainWindow::new()?;
    
    let files = Rc::new(VecModel::from(vec!["tool.cwl".into(), "workflows/main.cwl".into()]));
    ui.set_file_list(ModelRc::from(files));

    ui.run()?;

    Ok(())
}
