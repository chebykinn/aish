# aish - A Simple Shell

A basic Linux shell implementation in Rust that supports essential shell features like sh and bash.

## Features

### Core Shell Functionality
- **REPL (Read-Eval-Print-Loop)**: Interactive command prompt
- **Command execution**: Run external programs and system commands
- **Built-in commands**: Essential shell builtins
- **Command history**: Navigate through previous commands with arrow keys
- **Signal handling**: Ctrl+C and Ctrl+D support

### Execution Modes
- **Interactive mode**: Default REPL interface (run `aish` with no arguments)
- **Command mode**: Execute single command with `-c` flag (`aish -c "command"`)
- **Script mode**: Execute commands from shell script file (`aish script.sh`)
- **Markdown mode**: Execute commands from markdown file (`aish script.md`) - **NEW!**
- **Comment support**: Lines starting with `#` are ignored in script mode

### Built-in Commands
- `exit [code]` - Exit the shell with optional exit code
- `cd [directory]` - Change directory (supports ~, relative, and absolute paths)
- `pwd` - Print current working directory
- `echo [-n] [-e] [text]` - Display text with escape sequence support
- `export VAR=value` - Set environment variables
- `unset VAR` - Remove environment variables
- `env` - Display all environment variables
- `type command` - Show command type (builtin vs external)
- `help` - Display help information
- `history` - Show command history info

### Advanced Features
- **I/O Redirection**: 
  - `command > file` - Redirect stdout to file
  - `command < file` - Redirect stdin from file
  - `command >> file` - Append stdout to file
- **Pipes**: `command1 | command2` - Chain commands together
- **Background processes**: `command &` - Run commands in background
- **Variable expansion**: `$VAR` and `${VAR}` syntax
- **Quote handling**: Support for single and double quotes
- **Escape sequences**: Backslash escaping in commands

### Markdown Scripting (NEW!)
- **Literate programming**: Mix documentation and executable code
- **Code block execution**: Execute fenced code blocks marked as shell languages
- **Supported languages**: `bash`, `shell`, `sh`, `aish`, `zsh`, `fish`, or no language specification
- **Documentation rendering**: Display markdown text as context while executing
- **Multi-block scripts**: Support multiple code blocks in a single file
- **Command visualization**: Shows each command before execution

## Building and Running

```bash
# Build the shell
cargo build --release

# Run the shell interactively (default mode)
cargo run
# or
./target/release/aish

# Execute a single command with -c flag
./target/release/aish -c "echo 'Hello World'"

# Execute commands from a shell script file
./target/release/aish script.sh

# Execute commands from a markdown file (NEW!)
./target/release/aish script.md

# Show help
./target/release/aish --help
```

## Usage Examples

### Interactive Mode
```bash
# Basic commands
aish$ pwd
/home/user/aish

aish$ echo "Hello, World!"
Hello, World!

# Environment variables
aish$ export MY_VAR="test value"
aish$ echo $MY_VAR
test value

# I/O redirection
aish$ echo "Hello" > output.txt
aish$ cat < output.txt
Hello

# Pipes
aish$ ls | grep ".rs"
main.rs

# Background processes
aish$ sleep 10 &
[1] 12345
```

### Command Mode (-c flag)
```bash
# Execute single commands
./aish -c "pwd"
./aish -c "echo 'Hello from command mode'"
./aish -c "ls | wc -l"

# Use with complex commands
./aish -c "export VAR=value; echo \$VAR"
```

### Script Mode (file execution)
```bash
# Create a script file
cat > script.sh << 'EOF'
#!/path/to/aish
echo "Starting script..."
pwd
export MY_VAR="script variable"
echo "MY_VAR = $MY_VAR"
echo "Script done!"
EOF

# Execute the script
./aish script.sh
```

### Markdown Mode (NEW!)
```markdown
# My Deployment Script

This script deploys our application to the server.

## Pre-deployment checks

```bash
echo "Checking system status..."
uptime
df -h
```

## Main deployment

Now we'll deploy the application:

```shell
echo "Starting deployment..."
export DEPLOY_ENV="production"
echo "Deploying to $DEPLOY_ENV environment"
```

## Post-deployment verification

```bash
echo "Verifying deployment..."
echo "Deployment completed successfully!"
```

This approach combines documentation with executable code, making scripts self-documenting!
```

Execute with: `./aish deployment.md`

## Architecture

The shell is organized into several modules:

- **main.rs**: Entry point and module declarations
- **shell.rs**: Core shell logic, REPL loop, and command execution
- **parser.rs**: Command line parsing and tokenization
- **builtins.rs**: Built-in command implementations

### Key Components

1. **Shell struct**: Manages shell state, environment variables, and job control
2. **Parser**: Tokenizes and parses command lines into executable structures
3. **Builtins**: Implements essential shell commands
4. **Command execution**: Handles external process spawning and management

## Dependencies

- `rustyline`: Provides readline-like functionality for command input
- `nix`: Unix system calls and process management
- `libc`: Low-level system interface
- `clap`: Command line argument parsing
- `pulldown-cmark`: Markdown parsing for script execution

## Limitations

This is a basic shell implementation. Some advanced features found in bash/zsh are not implemented:

- Job control (fg, bg, jobs commands)
- Command substitution (`$(command)` or backticks)
- Globbing/wildcards (*, ?, [])
- Aliases
- Functions
- Conditionals and loops
- Advanced prompt customization
- Tab completion

## License

This project is for educational purposes.