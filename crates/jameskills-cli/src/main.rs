use std::{env, process};

const HELP: &str = "JameSkills CLI\n\nUsage: jameskills <command>\n\nCommands are added incrementally; run `jameskills --help` for this overview.\n";

fn main() {
    let mut args = env::args_os();
    let _program = args.next();

    match args.next() {
        None => print!("{HELP}"),
        Some(value) if value == "--help" || value == "-h" => {
            print!("{HELP}");
        }
        Some(_) => {
            eprintln!("Argumento no reconocido. Use `jameskills --help`.");
            process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::HELP;

    #[test]
    fn help_is_a_local_cli_entry_point() {
        assert!(HELP.starts_with("JameSkills CLI\n"));
        assert!(HELP.contains("Usage: jameskills <command>"));
    }
}
