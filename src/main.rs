use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::exit;

// =============================================================================
// CLI
// =============================================================================

/// Protect files from being modified by AI coding agents and accidental changes.
#[derive(Parser)]
#[command(name = "donttouch", version, about)]
struct Cli {
    /// Ignore git integration (treat directory as a plain directory)
    #[arg(long, global = true)]
    ignoregit: bool,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Initialize donttouch in the current directory
    Init,
    /// List protected files and their current state
    Status,
    /// Make all protected files read-only
    Lock,
    /// Restore write permissions (must run from outside target directory)
    Unlock {
        /// Path to the directory containing .donttouch.toml
        target: String,
    },
    /// Check if any protected files are writable (exits non-zero if so)
    Check,
    /// Disable protection (must run from outside target directory)
    Disable {
        /// Path to the directory containing .donttouch.toml
        target: String,
    },
    /// Re-enable protection (lock files, resume checks)
    Enable,
}

// =============================================================================
// Config
// =============================================================================

#[derive(Deserialize)]
struct ConfigFile {
    protect: ProtectSection,
}

#[derive(Deserialize)]
struct ProtectSection {
    patterns: Vec<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

// =============================================================================
// State Machine
// =============================================================================

/// Program states — the full lifecycle of a donttouch invocation.
enum State {
    /// Entry point: determine what to do based on command + filesystem
    Start { command: Command },

    /// No config found, user ran init — write config and prompt
    ToInit,

    /// Config file written, prompting user for patterns
    Initializing { config_path: PathBuf },

    /// Init complete, ask user if they want to lock
    EndInit,

    /// Terminal: output a message and exit successfully
    Done { message: String },

    /// Terminal: output an error and exit with failure
    Error { message: String },

    /// Terminal: program ends
    End { code: i32 },
}

/// A file matched by a protection pattern, with its current permission state.
struct ProtectedFile {
    path: PathBuf,
    readonly: bool,
}

impl State {
    /// Run the state machine to completion.
    fn run(self) -> ! {
        let mut state = self;
        loop {
            state = match state {
                State::Start { command } => handle_start(command),
                State::ToInit => handle_to_init(),
                State::Initializing { config_path } => handle_initializing(&config_path),
                State::EndInit => handle_end_init(),
                State::Done { message } => {
                    println!("{message}");
                    State::End { code: 0 }
                }
                State::Error { message } => {
                    eprintln!("{message}");
                    State::End { code: 1 }
                }
                State::End { code } => exit(code),
            };
        }
    }
}

// =============================================================================
// State Handlers
// =============================================================================

/// Start state: inspect filesystem + command to determine next state.
fn handle_start(command: Command) -> State {
    match command {
        Command::Init => {
            // Check if config already exists
            if Path::new(".donttouch.toml").exists() {
                State::Error {
                    message: "⚠️  .donttouch.toml already exists. Nothing to do.".into(),
                }
            } else {
                State::ToInit
            }
        }

        // All other commands require an existing config
        cmd => {
            let (root, _is_remote) = match &cmd {
                Command::Disable { target } | Command::Unlock { target } => {
                    match assert_outside(target) {
                        Ok(p) => (p, true),
                        Err(e) => return State::Error { message: e },
                    }
                }
                _ => (PathBuf::from("."), false),
            };

            let config_path = root.join(".donttouch.toml");
            let content = match std::fs::read_to_string(&config_path) {
                Ok(c) => c,
                Err(_) => {
                    return State::Error {
                        message: "No .donttouch.toml found. Run 'donttouch init' first.".into(),
                    }
                }
            };

            let config: ConfigFile = match toml::from_str(&content) {
                Ok(c) => c,
                Err(e) => {
                    return State::Error {
                        message: format!("Invalid {}: {e}", config_path.display()),
                    }
                }
            };

            let patterns = compile_patterns(&config.protect.patterns);
            let files = discover_files(&root, &patterns);

            // Dispatch command against resolved state
            if config.protect.enabled {
                dispatch_enabled(cmd, config, files, root)
            } else {
                dispatch_disabled(cmd, config, files, root)
            }
        }
    }
}

/// Dispatch a command when state is Enabled.
fn dispatch_enabled(cmd: Command, config: ConfigFile, files: Vec<ProtectedFile>, root: PathBuf) -> State {
    match cmd {
        Command::Status => do_status(&config, &files, true),
        Command::Lock => do_lock(&files),
        Command::Unlock { .. } => do_unlock(&files, &root),
        Command::Check => do_check(&files),
        Command::Enable => State::Done {
            message: "✅ Protection is already enabled.".into(),
        },
        Command::Disable { .. } => do_disable(&files, &root),
        Command::Init => unreachable!(),
    }
}

/// Dispatch a command when state is Disabled.
fn dispatch_disabled(cmd: Command, config: ConfigFile, files: Vec<ProtectedFile>, root: PathBuf) -> State {
    match cmd {
        Command::Status => do_status(&config, &files, false),
        Command::Lock => State::Error {
            message: "⏸️  Protection is disabled. Run 'donttouch enable' first.".into(),
        },
        Command::Unlock { .. } => do_unlock(&files, &root),
        Command::Check => State::Done {
            message: "⏸️  Protection is disabled. Skipping check.".into(),
        },
        Command::Enable => do_enable(&files, &root),
        Command::Disable { .. } => State::Done {
            message: "⏸️  Protection is already disabled.".into(),
        },
        Command::Init => unreachable!(),
    }
}

/// ToInit: write the config file
fn handle_to_init() -> State {
    let default_config = r#"# donttouch configuration
# Protect files from being modified by AI coding agents and accidental changes.

[protect]
enabled = true
patterns = []
"#;

    match std::fs::write(".donttouch.toml", default_config) {
        Ok(()) => State::Initializing {
            config_path: PathBuf::from(".donttouch.toml"),
        },
        Err(e) => State::Error {
            message: format!("Failed to create .donttouch.toml: {e}"),
        },
    }
}

/// Initializing: prompt user for patterns
fn handle_initializing(config_path: &Path) -> State {
    println!("✅ Created .donttouch.toml\n");
    println!("Add file patterns to protect (glob syntax, one per line).");
    println!("Examples: .env, secrets/**, docker-compose.prod.yml");
    println!("Press Enter on an empty line when done.\n");

    let mut patterns: Vec<String> = Vec::new();
    let stdin = io::stdin();

    loop {
        print!("pattern> ");
        io::stdout().flush().ok();

        let mut line = String::new();
        match stdin.read_line(&mut line) {
            Ok(0) => break, // EOF
            Ok(_) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    break;
                }
                // Validate the pattern
                match Pattern::new(&trimmed) {
                    Ok(_) => {
                        println!("   ✅ Added: {trimmed}");
                        patterns.push(trimmed);
                    }
                    Err(e) => {
                        println!("   ❌ Invalid pattern: {e}. Try again.");
                    }
                }
            }
            Err(e) => {
                return State::Error {
                    message: format!("Failed to read input: {e}"),
                };
            }
        }
    }

    if patterns.is_empty() {
        println!("\nNo patterns added. You can edit .donttouch.toml later.");
    } else {
        // Write patterns to config
        let patterns_str = patterns
            .iter()
            .map(|p| format!("    \"{p}\","))
            .collect::<Vec<_>>()
            .join("\n");

        let config = format!(
            r#"# donttouch configuration
# Protect files from being modified by AI coding agents and accidental changes.

[protect]
enabled = true
patterns = [
{patterns_str}
]
"#
        );

        if let Err(e) = std::fs::write(config_path, config) {
            return State::Error {
                message: format!("Failed to write config: {e}"),
            };
        }

        println!("\n📝 Saved {} pattern(s) to .donttouch.toml", patterns.len());
    }

    State::EndInit
}

/// EndInit: ask user if they want to lock now
fn handle_end_init() -> State {
    println!();
    print!("Lock protected files now? [Y/n] ");
    io::stdout().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).ok();
    let answer = answer.trim().to_lowercase();

    if answer.is_empty() || answer == "y" || answer == "yes" {
        // Load the config we just wrote and lock
        let content = match std::fs::read_to_string(".donttouch.toml") {
            Ok(c) => c,
            Err(e) => {
                return State::Error {
                    message: format!("Failed to read config: {e}"),
                };
            }
        };

        let config: ConfigFile = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                return State::Error {
                    message: format!("Invalid config: {e}"),
                };
            }
        };

        let patterns = compile_patterns(&config.protect.patterns);
        let files = discover_files(Path::new("."), &patterns);

        if files.is_empty() {
            State::Done {
                message: "No files match the protected patterns. Add files and run 'donttouch lock' later.".into(),
            }
        } else {
            do_lock(&files)
        }
    } else {
        State::Done {
            message: "Ok. Run 'donttouch lock' when you're ready.".into(),
        }
    }
}

// =============================================================================
// Actions (return next State)
// =============================================================================

fn do_status(config: &ConfigFile, files: &[ProtectedFile], enabled: bool) -> State {
    let mut out = String::new();

    if enabled {
        out.push_str("🔒 Protection: enabled\n");
    } else {
        out.push_str("🔓 Protection: disabled\n");
    }

    out.push_str("\nPatterns:\n");
    for p in &config.protect.patterns {
        out.push_str(&format!("   {p}\n"));
    }

    if files.is_empty() {
        out.push_str("\nNo files currently match the protected patterns.");
    } else {
        out.push_str("\nProtected files:\n");
        for f in files {
            let icon = if f.readonly { "🔒 read-only" } else { "🔓 writable" };
            out.push_str(&format!("   {icon}  {}\n", f.path.display()));
        }
    }

    State::Done { message: out }
}

fn do_lock(files: &[ProtectedFile]) -> State {
    let mut out = String::new();
    let mut locked = 0;
    let mut already = 0;

    // Lock all protected files
    for f in files {
        if f.readonly {
            already += 1;
        } else {
            match set_file_readonly(&f.path, true) {
                Ok(()) => {
                    out.push_str(&format!("   🔒 {}\n", f.path.display()));
                    locked += 1;
                }
                Err(e) => out.push_str(&format!("   ❌ {e}\n")),
            }
        }
    }

    // Also lock the config file itself
    let config_path = Path::new(".donttouch.toml");
    if !is_file_readonly(config_path) {
        match set_file_readonly(config_path, true) {
            Ok(()) => {
                out.push_str("   🔒 .donttouch.toml\n");
                locked += 1;
            }
            Err(e) => out.push_str(&format!("   ❌ {e}\n")),
        }
    } else {
        already += 1;
    }

    if locked > 0 {
        out.push_str(&format!("\n✅ Locked {locked} file(s)."));
    }
    if already > 0 {
        out.push_str(&format!("\n   ({already} already read-only)"));
    }
    if locked == 0 && already > 0 {
        out.push_str("\n✅ All protected files are already read-only.");
    }

    State::Done { message: out }
}

fn do_unlock(files: &[ProtectedFile], root: &Path) -> State {
    let mut out = String::new();
    let mut unlocked = 0;

    for f in files {
        if f.readonly {
            match set_file_readonly(&f.path, false) {
                Ok(()) => {
                    out.push_str(&format!("   🔓 {}\n", f.path.display()));
                    unlocked += 1;
                }
                Err(e) => out.push_str(&format!("   ❌ {e}\n")),
            }
        }
    }

    // Also unlock the config file
    let config_path = root.join(".donttouch.toml");
    if is_file_readonly(&config_path) {
        match set_file_readonly(&config_path, false) {
            Ok(()) => {
                out.push_str(&format!("   🔓 {}\n", config_path.display()));
                unlocked += 1;
            }
            Err(e) => out.push_str(&format!("   ❌ {e}\n")),
        }
    }

    if unlocked > 0 {
        out.push_str(&format!("\n✅ Unlocked {unlocked} file(s)."));
    } else {
        out.push_str("All files were already writable.");
    }

    State::Done { message: out }
}

fn do_check(files: &[ProtectedFile]) -> State {
    let writable: Vec<&ProtectedFile> = files.iter().filter(|f| !f.readonly).collect();

    if writable.is_empty() {
        State::Done {
            message: "✅ All protected files are read-only.".into(),
        }
    } else {
        let mut out = String::from("🚫 Protected files are writable!\n\n");
        for f in &writable {
            out.push_str(&format!("   • {}\n", f.path.display()));
        }
        out.push_str("\nRun 'donttouch lock' to make them read-only.");
        State::Error { message: out }
    }
}

fn do_enable(files: &[ProtectedFile], root: &Path) -> State {
    // Config file may be unlocked — write first, then lock everything
    if let Err(e) = write_enabled(root, true) {
        return State::Error { message: e };
    }

    let mut out = String::new();
    let mut locked = 0;

    for f in files {
        if !f.readonly {
            if set_file_readonly(&f.path, true).is_ok() {
                locked += 1;
            }
        }
    }

    // Lock the config file too
    let config_path = root.join(".donttouch.toml");
    if !is_file_readonly(&config_path) {
        if set_file_readonly(&config_path, true).is_ok() {
            locked += 1;
        }
    }

    if locked > 0 {
        out.push_str(&format!("   🔒 Locked {locked} file(s).\n"));
    }
    out.push_str("✅ Protection enabled.");

    State::Done { message: out }
}

fn do_disable(files: &[ProtectedFile], root: &Path) -> State {
    // Unlock config file first so we can write to it
    let config_path = root.join(".donttouch.toml");
    if is_file_readonly(&config_path) {
        let _ = set_file_readonly(&config_path, false);
    }

    if let Err(e) = write_enabled(root, false) {
        return State::Error { message: e };
    }

    let mut unlocked = 0;
    for f in files {
        if f.readonly {
            if set_file_readonly(&f.path, false).is_ok() {
                unlocked += 1;
            }
        }
    }

    let mut out = String::new();
    if unlocked > 0 {
        out.push_str(&format!("   🔓 Unlocked {unlocked} file(s).\n"));
    }
    out.push_str("🔓 Protection disabled.\n   ⚠️  You must run 'donttouch enable' before you can push.");

    State::Done { message: out }
}

// =============================================================================
// Outside-Directory Check
// =============================================================================

fn assert_outside(target: &str) -> Result<PathBuf, String> {
    let canonical_target = std::fs::canonicalize(target)
        .map_err(|e| format!("Cannot resolve target path '{target}': {e}"))?;

    if !canonical_target.join(".donttouch.toml").exists() {
        return Err(format!(
            "No .donttouch.toml found in '{}'. Is this the right directory?",
            canonical_target.display()
        ));
    }

    let canonical_cwd = std::env::current_dir()
        .and_then(|p| std::fs::canonicalize(p))
        .map_err(|e| format!("Cannot resolve current directory: {e}"))?;

    if canonical_cwd.starts_with(&canonical_target) {
        return Err(format!(
            "🚫 This command must be run from OUTSIDE the target directory.\n\n\
             Current directory: {}\n\
             Target directory:  {}\n\n\
             This restriction prevents AI coding agents from disabling protection\n\
             while working inside the project.\n\n\
             Try: cd {} && donttouch disable {}",
            canonical_cwd.display(),
            canonical_target.display(),
            canonical_target
                .parent()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|| "/tmp".into()),
            canonical_target.display(),
        ));
    }

    Ok(canonical_target)
}

// =============================================================================
// Filesystem Helpers
// =============================================================================

fn compile_patterns(raw: &[String]) -> Vec<Pattern> {
    raw.iter()
        .filter_map(|p| match Pattern::new(p) {
            Ok(pat) => Some(pat),
            Err(e) => {
                eprintln!("donttouch: bad glob pattern '{p}': {e}");
                None
            }
        })
        .collect()
}

fn discover_files(root: &Path, patterns: &[Pattern]) -> Vec<ProtectedFile> {
    let mut results = Vec::new();
    walk_dir(root, root, patterns, &mut results);
    results.sort_by(|a, b| a.path.cmp(&b.path));
    results
}

fn walk_dir(base: &Path, dir: &Path, patterns: &[Pattern], results: &mut Vec<ProtectedFile>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }
        }

        let rel = path.strip_prefix(base).unwrap_or(&path);
        let rel_str = rel.to_string_lossy();

        if path.is_dir() {
            walk_dir(base, &path, patterns, results);
        } else if patterns.iter().any(|p| p.matches(&rel_str)) {
            results.push(ProtectedFile {
                path: path.clone(),
                readonly: is_file_readonly(&path),
            });
        }
    }
}

#[cfg(unix)]
fn is_file_readonly(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    std::fs::metadata(path)
        .map(|m| (m.permissions().mode() & 0o200) == 0)
        .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_file_readonly(path: &Path) -> bool {
    std::fs::metadata(path)
        .map(|m| m.permissions().readonly())
        .unwrap_or(false)
}

#[cfg(unix)]
fn set_file_readonly(path: &Path, readonly: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut mode = meta.permissions().mode();
    if readonly {
        mode &= !0o222;
    } else {
        mode |= 0o200;
    }
    std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode))
        .map_err(|e| format!("Cannot set permissions on {}: {e}", path.display()))
}

#[cfg(not(unix))]
fn set_file_readonly(path: &Path, readonly: bool) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut perms = meta.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Cannot set permissions on {}: {e}", path.display()))
}

fn write_enabled(root: &Path, enabled: bool) -> Result<(), String> {
    let config_path = root.join(".donttouch.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Could not read {}: {e}", config_path.display()))?;

    let new_content = if content.contains("enabled") {
        content
            .lines()
            .map(|line| {
                if line.trim().starts_with("enabled") && line.contains('=') {
                    format!("enabled = {enabled}")
                } else {
                    line.to_string()
                }
            })
            .collect::<Vec<_>>()
            .join("\n")
            + "\n"
    } else {
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        if let Some(idx) = lines.iter().position(|l| l.trim() == "[protect]") {
            lines.insert(idx + 1, format!("enabled = {enabled}"));
        }
        lines.join("\n") + "\n"
    };

    std::fs::write(&config_path, new_content)
        .map_err(|e| format!("Failed to write {}: {e}", config_path.display()))
}

// =============================================================================
// Main
// =============================================================================

fn main() {
    let cli = Cli::parse();
    let _ = cli.ignoregit; // Reserved for future git integration

    let start = State::Start { command: cli.command };
    start.run();
}
