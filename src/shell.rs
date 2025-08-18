use rustyline::Editor;
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io::{self, BufRead, BufReader};
use std::process::{Child, Command, Stdio};
// use nix::sys::signal::{self, Signal};
// use nix::unistd::{self, Pid};

use crate::builtins::Builtins;
use crate::context::LLMActionProcessor;
use crate::markdown::{is_markdown_file, MarkdownScript};
use crate::parser::{CommandLine, Parser, RedirectionType, SimpleCommand};

pub struct Shell {
    editor: Editor<()>,
    env_vars: HashMap<String, String>,
    background_jobs: Vec<Child>,
    exit_requested: bool,
    parser: Parser,
    builtins: Builtins,
    llm_processor: LLMActionProcessor,
}

impl Shell {
    pub fn new() -> Self {
        let mut env_vars = HashMap::new();

        // Initialize with system environment variables
        for (key, value) in env::vars() {
            env_vars.insert(key, value);
        }

        // Set default PATH if not present
        if !env_vars.contains_key("PATH") {
            env_vars.insert(
                "PATH".to_string(),
                "/usr/local/bin:/usr/bin:/bin".to_string(),
            );
        }

        // Set default PS1 prompt
        if !env_vars.contains_key("PS1") {
            env_vars.insert("PS1".to_string(), "aish$ ".to_string());
        }

        Shell {
            editor: Editor::new().expect("Failed to create readline editor"),
            env_vars,
            background_jobs: Vec::new(),
            exit_requested: false,
            parser: Parser::new(),
            builtins: Builtins::new(),
            llm_processor: LLMActionProcessor::new(),
        }
    }

    pub async fn run(&mut self) -> io::Result<()> {
        self.run_interactive().await
    }

    pub async fn run_interactive(&mut self) -> io::Result<()> {
        self.setup_signal_handlers()?;

        println!("Welcome to aish - AI-Enhanced Shell");
        println!("Type 'exit' or use Ctrl+D to quit");
        println!("This shell uses natural language commands");

        while !self.exit_requested {
            self.cleanup_background_jobs();

            let prompt = self.get_prompt();

            match self.editor.readline(&prompt) {
                Ok(line) => {
                    let line = line.trim();
                    if line.is_empty() {
                        continue;
                    }

                    self.editor.add_history_entry(line);

                    if let Err(e) = self.execute_line_interactive(line).await {
                        eprintln!("aish: {}", e);
                    }
                }
                Err(rustyline::error::ReadlineError::Interrupted) => {
                    // Ctrl+C pressed
                    println!("^C");
                    continue;
                }
                Err(rustyline::error::ReadlineError::Eof) => {
                    // Ctrl+D pressed
                    println!("exit");
                    break;
                }
                Err(err) => {
                    eprintln!("Error: {}", err);
                    break;
                }
            }
        }

        self.cleanup_all_jobs();
        Ok(())
    }

    pub async fn run_command(&mut self, command: &str) -> io::Result<()> {
        self.setup_signal_handlers()?;
        self.execute_line(command)?;
        self.cleanup_all_jobs();
        Ok(())
    }

    pub async fn run_file(&mut self, filename: &str) -> io::Result<()> {
        self.setup_signal_handlers()?;

        if is_markdown_file(filename) {
            self.run_markdown_file(filename).await
        } else {
            self.run_shell_script(filename).await
        }
    }

    async fn run_markdown_file(&mut self, filename: &str) -> io::Result<()> {
        let content = std::fs::read_to_string(filename)
            .map_err(|e| io::Error::new(e.kind(), format!("aish: {}: {}", filename, e)))?;

        let script = MarkdownScript::parse(&content).map_err(|e| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Failed to parse markdown: {}", e),
            )
        })?;

        println!("[SYS] Executing intelligent markdown script: {}", filename);
        println!("[SYS] {}", self.llm_processor.get_context_info());

        // Display headers as labels/comments (non-actionable)
        let headers = script.get_headers();
        if !headers.is_empty() {
            println!("[SYS] Found {} section header(s):", headers.len());
            for (level, text) in &headers {
                let indent = "  ".repeat((*level as usize).saturating_sub(1));
                println!("[SYS]   {}{} {}", indent, "#".repeat(*level as usize), text);
            }
        }
        println!();

        // Process LLM actions (paragraphs and headers)
        let llm_actions = script.get_llm_actions();
        for (action_index, action) in llm_actions.iter().enumerate() {
            if self.exit_requested {
                break;
            }

            // Print the paragraph content with token usage
            let token_usage = self.llm_processor.get_token_usage();
            match action {
                crate::context::LLMAction::Comment { content } => {
                    println!("[CMD] {} {}", token_usage, content);
                }
            }

            match self.llm_processor.process_action(action.clone()).await {
                Ok(result) => {
                    println!("{}", result);
                    // Show updated token count after processing
                    let updated_tokens = self.llm_processor.get_token_usage();
                    println!("[SYS] Paragraph complete: {}", updated_tokens);
                }
                Err(e) => {
                    eprintln!("LLM Action Error: {}", e);
                }
            }
        }

        // Execute shell code blocks
        let executable_blocks = script.get_executable_blocks();
        for (block_index, (lang, code)) in executable_blocks.iter().enumerate() {
            if self.exit_requested {
                break;
            }

            self.cleanup_background_jobs();

            let lang_display = lang.as_deref().unwrap_or("shell");
            println!(
                "\n[CMD] Executing {} block {} ---",
                lang_display,
                block_index + 1
            );

            // Execute each line in the code block
            for (line_num, line) in code.lines().enumerate() {
                let line = line.trim();

                // Skip empty lines and comments
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }

                if self.exit_requested {
                    break;
                }

                println!("$ {}", line);

                if let Err(e) = self.execute_line(line) {
                    eprintln!(
                        "aish: {}: block {}:{}: {}",
                        filename,
                        block_index + 1,
                        line_num + 1,
                        e
                    );
                    // Continue execution even if a command fails
                }
            }
        }

        // Handle function declarations
        let functions = script.get_functions();
        if !functions.is_empty() {
            println!("\n[SYS] Found {} function declaration(s)", functions.len());
            for func in functions {
                if let crate::markdown::MarkdownElement::FunctionDeclaration(name, params, _) = func
                {
                    println!("  func {}({})", name, params.join(", "));
                }
            }
        }

        self.cleanup_all_jobs();
        println!("\n[SYS] Script execution completed");
        println!("[SYS] Final {}", self.llm_processor.get_context_info());
        Ok(())
    }

    async fn run_shell_script(&mut self, filename: &str) -> io::Result<()> {
        let file = File::open(filename)
            .map_err(|e| io::Error::new(e.kind(), format!("aish: {}: {}", filename, e)))?;

        let reader = BufReader::new(file);
        let mut line_number = 0;

        for line_result in reader.lines() {
            line_number += 1;

            if self.exit_requested {
                break;
            }

            self.cleanup_background_jobs();

            let line = line_result?;
            let line = line.trim();

            // Skip empty lines and comments
            if line.is_empty() || line.starts_with('#') {
                continue;
            }

            if let Err(e) = self.execute_line(line) {
                eprintln!("aish: {}:{}: {}", filename, line_number, e);
                // Continue execution even if a command fails
            }
        }

        self.cleanup_all_jobs();
        Ok(())
    }

    // Interactive mode with AI support - uses same parsing as .aish files
    async fn execute_line_interactive(&mut self, line: &str) -> io::Result<()> {
        // Create a simple markdown document with just this line as a paragraph
        let markdown_content = format!("{}\n", line);

        // Parse using the same markdown parser as .aish files
        let script = match crate::markdown::MarkdownScript::parse(&markdown_content) {
            Ok(script) => script,
            Err(_) => {
                // If markdown parsing fails, treat as traditional shell command
                return self.execute_line(line);
            }
        };

        // Get LLM actions using the same logic as .aish files
        let llm_actions = script.get_llm_actions();

        if !llm_actions.is_empty() {
            // Process as AI command using same logic as .aish files
            for action in llm_actions {
                let token_usage = self.llm_processor.get_token_usage();
                match &action {
                    crate::context::LLMAction::Comment { content } => {
                        println!("[SYS] {} {}", token_usage, content);
                    }
                }

                match self.llm_processor.process_action(action).await {
                    Ok(result) => {
                        println!("{}", result);
                        let updated_tokens = self.llm_processor.get_token_usage();
                        println!("[SYS] Complete: {}", updated_tokens);
                    }
                    Err(e) => {
                        eprintln!("[SYS] Error: {}", e);
                    }
                }
            }
            Ok(())
        } else {
            // No LLM actions, execute as traditional shell command
            self.execute_line(line)
        }
    }

    // Traditional shell command execution (synchronous)
    fn execute_line(&mut self, line: &str) -> io::Result<()> {
        match self.parser.parse(line) {
            Ok(command_line) => self.execute_command_line(command_line),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Parse error: {}", e),
            )),
        }
    }

    fn get_prompt(&self) -> String {
        self.env_vars
            .get("PS1")
            .unwrap_or(&"aish$ ".to_string())
            .clone()
    }

    fn execute_command_line(&mut self, command_line: CommandLine) -> io::Result<()> {
        match command_line {
            CommandLine::Simple(cmd) => self.execute_simple_command(cmd, false),
            CommandLine::Pipeline(commands) => self.execute_pipeline(commands),
            CommandLine::Background(cmd) => self.execute_simple_command(cmd, true),
        }
    }

    fn execute_simple_command(&mut self, cmd: SimpleCommand, background: bool) -> io::Result<()> {
        if cmd.args.is_empty() {
            return Ok(());
        }

        let command_name = &cmd.args[0];

        // Check if it's a builtin command
        if let Some(result) = self.builtins.execute(command_name, &cmd.args[1..]) {
            match result(&mut *self) {
                Ok(_) => return Ok(()),
                Err(e) => return Err(e),
            }
        }

        // Execute external command
        self.execute_external_command(cmd, background)
    }

    fn execute_external_command(&mut self, cmd: SimpleCommand, background: bool) -> io::Result<()> {
        let mut command = Command::new(&cmd.args[0]);
        command.args(&cmd.args[1..]);

        // Set environment variables
        for (key, value) in &self.env_vars {
            command.env(key, value);
        }

        // Handle redirections
        for redir in &cmd.redirections {
            match redir.redir_type {
                RedirectionType::Input => {
                    command.stdin(Stdio::from(std::fs::File::open(&redir.filename)?));
                }
                RedirectionType::Output => {
                    command.stdout(Stdio::from(std::fs::File::create(&redir.filename)?));
                }
                RedirectionType::Append => {
                    command.stdout(Stdio::from(
                        std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(&redir.filename)?,
                    ));
                }
            }
        }

        if background {
            command.stdin(Stdio::null());
            let child = command.spawn()?;
            println!("[{}] {}", self.background_jobs.len() + 1, child.id());
            self.background_jobs.push(child);
        } else {
            let status = command.status()?;
            if !status.success() {
                if let Some(code) = status.code() {
                    eprintln!("Command exited with code {}", code);
                } else {
                    eprintln!("Command terminated by signal");
                }
            }
        }

        Ok(())
    }

    fn execute_pipeline(&mut self, commands: Vec<SimpleCommand>) -> io::Result<()> {
        if commands.is_empty() {
            return Ok(());
        }

        let mut children = Vec::new();
        let mut previous_stdout = None;

        for (i, cmd) in commands.iter().enumerate() {
            let mut command = Command::new(&cmd.args[0]);
            command.args(&cmd.args[1..]);

            // Set environment variables
            for (key, value) in &self.env_vars {
                command.env(key, value);
            }

            // Set up stdin
            if i == 0 {
                command.stdin(Stdio::inherit());
            } else {
                command.stdin(previous_stdout.unwrap());
            }

            // Set up stdout
            if i == commands.len() - 1 {
                command.stdout(Stdio::inherit());
            } else {
                command.stdout(Stdio::piped());
            }

            command.stderr(Stdio::inherit());

            let mut child = command.spawn()?;
            previous_stdout = child.stdout.take().map(Stdio::from);
            children.push(child);
        }

        // Wait for all commands to complete
        for mut child in children {
            let _ = child.wait()?;
        }

        Ok(())
    }

    fn cleanup_background_jobs(&mut self) {
        self.background_jobs.retain_mut(|job| {
            match job.try_wait() {
                Ok(Some(_status)) => {
                    println!("[{}] Done", job.id());
                    false // Remove completed job
                }
                Ok(None) => true, // Job still running
                Err(_) => false,  // Job errored, remove it
            }
        });
    }

    fn cleanup_all_jobs(&mut self) {
        for mut job in self.background_jobs.drain(..) {
            let _ = job.kill();
            let _ = job.wait();
        }
    }

    fn setup_signal_handlers(&self) -> io::Result<()> {
        // Signal handling setup would go here
        // For now, we'll rely on rustyline's built-in handling
        Ok(())
    }

    pub fn set_env_var(&mut self, key: String, value: String) {
        self.env_vars.insert(key, value);
    }

    pub fn get_env_var(&self, key: &str) -> Option<&String> {
        self.env_vars.get(key)
    }

    pub fn unset_env_var(&mut self, key: &str) {
        self.env_vars.remove(key);
    }

    pub fn request_exit(&mut self) {
        self.exit_requested = true;
    }

    pub fn get_current_dir(&self) -> io::Result<String> {
        Ok(env::current_dir()?.display().to_string())
    }

    pub fn change_directory(&mut self, path: &str) -> io::Result<()> {
        let new_dir = if path.is_empty() || path == "~" {
            env::var("HOME").unwrap_or_else(|_| "/".to_string())
        } else if path.starts_with("~/") {
            let home = env::var("HOME").unwrap_or_else(|_| "/".to_string());
            format!("{}/{}", home, &path[2..])
        } else {
            path.to_string()
        };

        env::set_current_dir(&new_dir)?;
        self.set_env_var("PWD".to_string(), new_dir);
        Ok(())
    }
}

