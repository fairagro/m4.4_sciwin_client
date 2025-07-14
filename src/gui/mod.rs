use crate::io::get_workflows_folder;
use slint::{ComponentHandle, ModelRc, VecModel};
use std::{env, error::Error, path::Path, rc::Rc};

slint::include_modules!();

pub fn main() -> Result<(), Box<dyn Error>> {
    let ui = MainWindow::new()?;

    let files = read_files(&env::current_dir()?)?;
    let s_files: Vec<slint::SharedString> = files.into_iter().map(|s| s.into()).collect();
    let rc_files = Rc::new(VecModel::from(s_files));
    ui.set_file_list(ModelRc::from(rc_files));

    ui.run()?;

    Ok(())
}

fn read_files(project_dir: &Path) -> Result<Vec<String>, Box<dyn Error>> {
    let workflow_dir = project_dir.join(get_workflows_folder());
    let mut files = vec![];

    for entry in walkdir::WalkDir::new(workflow_dir) {
        let entry = entry?;
        if entry.file_type().is_file() && entry.path().extension().map(|ext| ext == "cwl").unwrap_or(false) {
            files.push(entry.path().to_string_lossy().into_owned());
        }
    }
    Ok(files)
}
