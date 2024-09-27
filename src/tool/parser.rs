//TODO: complete list
static SCRIPT_EXECUTORS: &[&str] = &["python", "Rscript"];

pub fn parse_command_line(command: Vec<String>) {
    let base_command = get_base_command(command);
    println!("{:?}", base_command);
}

fn get_base_command(command: Vec<String>) -> Vec<String> {
    if command.is_empty() {
        return Vec::new();
    };

    let mut base_command = vec![command[0].clone()];

    if SCRIPT_EXECUTORS.contains(&command[0].as_str()) {
        base_command.push(command[1].clone());
    }

    return base_command;
}