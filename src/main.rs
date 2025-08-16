use std::io;
use clap::{Arg, ArgAction, Command as ClapCommand};

mod shell;
mod parser;
mod builtins;
mod markdown;
mod context;
mod llm;

use shell::Shell;

#[tokio::main]
async fn main() -> io::Result<()> {
    let matches = ClapCommand::new("aish")
        .version("0.1.0")
        .about("A simple shell implementation in Rust")
        .arg(
            Arg::new("command")
                .short('c')
                .long("command")
                .value_name("COMMAND")
                .help("Execute the given command string")
                .action(ArgAction::Set)
        )
        .arg(
            Arg::new("file")
                .value_name("FILE")
                .help("Execute commands from the given file")
                .action(ArgAction::Set)
        )
        .get_matches();

    let mut shell = Shell::new();

    if let Some(command) = matches.get_one::<String>("command") {
        // Execute command string mode (-c flag)
        shell.run_command(command).await
    } else if let Some(filename) = matches.get_one::<String>("file") {
        // Execute file mode
        shell.run_file(filename).await
    } else {
        // Interactive mode (default)
        shell.run_interactive().await
    }
}
