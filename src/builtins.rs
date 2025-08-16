use std::env;
use std::io::{self, Write};
use crate::shell::Shell;

pub struct Builtins;

impl Builtins {
    pub fn new() -> Self {
        Builtins
    }

    pub fn execute(&self, command: &str, args: &[String]) -> Option<Box<dyn FnOnce(&mut Shell) -> io::Result<()>>> {
        let args = args.to_vec();
        match command {
            "exit" => Some(Box::new(move |shell| Self::exit(&args, shell))),
            "cd" => Some(Box::new(move |shell| Self::cd(&args, shell))),
            "pwd" => Some(Box::new(move |shell| Self::pwd(&args, shell))),
            "echo" => Some(Box::new(move |shell| Self::echo(&args, shell))),
            "export" => Some(Box::new(move |shell| Self::export(&args, shell))),
            "unset" => Some(Box::new(move |shell| Self::unset(&args, shell))),
            "env" => Some(Box::new(move |shell| Self::env(&args, shell))),
            "type" => Some(Box::new(move |shell| Self::type_command(&args, shell))),
            "help" => Some(Box::new(move |shell| Self::help(&args, shell))),
            "history" => Some(Box::new(move |shell| Self::history(&args, shell))),
            _ => None, // Not a builtin command
        }
    }

    fn exit(args: &[String], shell: &mut Shell) -> io::Result<()> {
        let exit_code = if args.is_empty() {
            0
        } else {
            args[0].parse::<i32>().unwrap_or(1)
        };

        shell.request_exit();
        
        if exit_code != 0 {
            std::process::exit(exit_code);
        }
        
        Ok(())
    }

    fn cd(args: &[String], shell: &mut Shell) -> io::Result<()> {
        let path = if args.is_empty() {
            ""
        } else {
            &args[0]
        };

        shell.change_directory(path)
    }

    fn pwd(_args: &[String], shell: &mut Shell) -> io::Result<()> {
        match shell.get_current_dir() {
            Ok(dir) => {
                println!("{}", dir);
                Ok(())
            }
            Err(e) => Err(e),
        }
    }

    fn echo(args: &[String], _shell: &mut Shell) -> io::Result<()> {
        let mut output = String::new();
        let mut newline = true;
        let mut interpret_escapes = false;
        
        let mut i = 0;
        while i < args.len() {
            match args[i].as_str() {
                "-n" => {
                    newline = false;
                    i += 1;
                    continue;
                }
                "-e" => {
                    interpret_escapes = true;
                    i += 1;
                    continue;
                }
                "-E" => {
                    interpret_escapes = false;
                    i += 1;
                    continue;
                }
                _ => break,
            }
        }

        for (idx, arg) in args[i..].iter().enumerate() {
            if idx > 0 {
                output.push(' ');
            }
            
            if interpret_escapes {
                output.push_str(&Self::interpret_escape_sequences(arg));
            } else {
                output.push_str(arg);
            }
        }

        if newline {
            println!("{}", output);
        } else {
            print!("{}", output);
            io::stdout().flush()?;
        }

        Ok(())
    }

    fn interpret_escape_sequences(input: &str) -> String {
        let mut result = String::new();
        let mut chars = input.chars();
        
        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('\"') => result.push('\"'),
                    Some('\'') => result.push('\''),
                    Some(c) => {
                        result.push('\\');
                        result.push(c);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }
        
        result
    }

    fn export(args: &[String], shell: &mut Shell) -> io::Result<()> {
        if args.is_empty() {
            // Display all environment variables
            return Self::env(args, shell);
        }

        for arg in args {
            if let Some(pos) = arg.find('=') {
                let (key, value) = arg.split_at(pos);
                let value = &value[1..]; // Skip the '=' character
                shell.set_env_var(key.to_string(), value.to_string());
                env::set_var(key, value);
            } else {
                // Export existing variable
                if let Some(value) = shell.get_env_var(arg) {
                    env::set_var(arg, value);
                } else {
                    eprintln!("export: {}: not found", arg);
                }
            }
        }

        Ok(())
    }

    fn unset(args: &[String], shell: &mut Shell) -> io::Result<()> {
        for arg in args {
            shell.unset_env_var(arg);
            env::remove_var(arg);
        }
        Ok(())
    }

    fn env(_args: &[String], _shell: &mut Shell) -> io::Result<()> {
        // Get all environment variables and sort them
        let mut vars: Vec<_> = env::vars().collect();
        vars.sort_by(|a, b| a.0.cmp(&b.0));
        
        for (key, value) in vars {
            println!("{}={}", key, value);
        }
        
        Ok(())
    }

    fn type_command(args: &[String], _shell: &mut Shell) -> io::Result<()> {
        if args.is_empty() {
            eprintln!("type: usage: type [-afptP] name [name ...]");
            return Ok(());
        }

        for arg in args {
            if Self::is_builtin(arg) {
                println!("{} is a shell builtin", arg);
            } else {
                // Check if it's in PATH
                if let Some(path) = Self::find_in_path(arg) {
                    println!("{} is {}", arg, path);
                } else {
                    println!("{}: not found", arg);
                }
            }
        }

        Ok(())
    }

    fn is_builtin(command: &str) -> bool {
        matches!(command, "exit" | "cd" | "pwd" | "echo" | "export" | "unset" | "env" | "type" | "help" | "history")
    }

    fn find_in_path(command: &str) -> Option<String> {
        if let Ok(path_var) = env::var("PATH") {
            for path_dir in path_var.split(':') {
                let full_path = format!("{}/{}", path_dir, command);
                if std::path::Path::new(&full_path).exists() {
                    return Some(full_path);
                }
            }
        }
        None
    }

    fn help(_args: &[String], _shell: &mut Shell) -> io::Result<()> {
        println!("aish - A simple shell");
        println!("Built-in commands:");
        println!("  exit [n]     - Exit the shell with optional exit code");
        println!("  cd [dir]     - Change directory to dir (or home if no dir)");
        println!("  pwd          - Print current working directory");
        println!("  echo [args]  - Display arguments");
        println!("  export VAR=value - Set environment variable");
        println!("  unset VAR    - Unset environment variable");
        println!("  env          - Display environment variables");
        println!("  type command - Display information about command type");
        println!("  help         - Display this help message");
        println!("  history      - Display command history");
        println!();
        println!("Features:");
        println!("  - Command execution");
        println!("  - I/O redirection (>, <, >>)");
        println!("  - Pipes (|)");
        println!("  - Background processes (&)");
        println!("  - Variable expansion ($VAR, ${{VAR}})");
        println!("  - Command history (arrow keys)");
        println!("  - Tab completion");

        Ok(())
    }

    fn history(_args: &[String], _shell: &mut Shell) -> io::Result<()> {
        println!("History functionality would be implemented here");
        println!("Use arrow keys to navigate through command history");
        Ok(())
    }
}