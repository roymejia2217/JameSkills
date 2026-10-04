mod commands;
mod output;

use clap::{CommandFactory, error::ErrorKind};
use commands::{Cli, CliCommand, dispatch_cli};
use jameskills_core::AppError;
use jameskills_infra::{composition::build_services, platform::resolve_user_dirs};
use output::{CliResponse, render_json, render_text, response_for_app_error};
use std::{env, ffi::OsString, process::ExitCode};

fn main() -> ExitCode {
    ExitCode::from(run(env::args_os()))
}

fn run(args: impl IntoIterator<Item = OsString>) -> u8 {
    let args = args.into_iter().collect::<Vec<_>>();
    let json_requested = args.iter().any(|arg| arg == "--json");
    let cli = match Cli::parse(args.clone()) {
        Ok(cli) => cli,
        Err(error) => match error.kind() {
            ErrorKind::DisplayHelp | ErrorKind::DisplayVersion => {
                let _ = error.print();
                return 0;
            }
            _ if json_requested => {
                print_response(
                    CliResponse::error(
                        "parse",
                        "arguments.invalid",
                        "Command arguments are invalid.",
                        2,
                    ),
                    true,
                );
                return 2;
            }
            _ => {
                eprintln!("Command arguments are invalid. Use `jameskills --help`.");
                return 2;
            }
        },
    };

    let Some(command) = cli.command.as_ref() else {
        let mut command = Cli::command();
        let _ = command.print_help();
        println!();
        return 0;
    };

    if matches!(command, CliCommand::Doctor | CliCommand::Validate { .. }) {
        let runtime = match resolve_user_dirs()
            .map_err(|_| AppError::CapabilityUnavailable {
                id: "platform.user_directories".to_owned(),
                guidance_id: "setup.user_directories".to_owned(),
            })
            .and_then(build_services)
        {
            Ok(runtime) => runtime,
            Err(error) => {
                let response = response_for_app_error("doctor", &error);
                let exit_code = response.exit_code as u8;
                print_response(response, cli.json);
                return exit_code;
            }
        };
        let response = dispatch_cli(cli, Some(&runtime));
        let exit_code = response.exit_code as u8;
        print_response(response, json_requested);
        return exit_code;
    }

    let response = dispatch_cli(cli, None);
    let exit_code = response.exit_code as u8;
    print_response(response, json_requested);
    exit_code
}

fn print_response(response: CliResponse, json: bool) {
    if json {
        match render_json(&response) {
            Ok(rendered) => println!("{rendered}"),
            Err(_) => println!(
                r#"{{"schema_version":1,"command":"output","error":{{"code":"output.serialization","message":"Could not serialize response."}}}}"#
            ),
        }
    } else {
        println!("{}", render_text(&response));
    }
}
