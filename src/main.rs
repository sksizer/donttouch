use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{self, exit};

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
    /// Check if protection is enabled before push (used by pre-push hook)
    #[command(name = "check-push")]
    CheckPush,
    /// Disable protection (must run from outside target directory)
    Disable {
        /// Path to the directory containing .donttouch.toml
        target: String,
    },
    /// Re-enable protection (lock files, resume checks)
    Enable,
    /// Remove donttouch from a directory (must run from outside target directory)
    Remove {
        /// Path to the directory containing .donttouch.toml
        target: String,
    },
    /// Show which pattern protects a given file
    Why {
        /// File path to check
        file: String,
    },
    /// Add agent instructions to coding agent config files
    Inject {
        /// Preview changes without writing
        #[arg(long)]
        dry_run: bool,
    },
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
// Context — describes the environment donttouch is running in
// =============================================================================

#[derive(Clone)]
enum Context {
    /// Plain directory, no git
    Plain,
    /// Git repository
    Git {
        has_husky: bool,
        hooks_installed: bool,
    },
}

impl Context {
    /// Detect the context for a given directory.
    fn detect(root: &Path, ignoregit: bool) -> Self {
        if ignoregit {
            return Context::Plain;
        }

        let git_dir = root.join(".git");
        if !git_dir.exists() {
            return Context::Plain;
        }

        let has_husky = root.join(".husky").is_dir();

        // Check if donttouch hooks are installed
        let hooks_installed = if has_husky {
            hook_contains(&root.join(".husky/pre-commit"), "donttouch")
        } else {
            hook_contains(&root.join(".git/hooks/pre-commit"), "donttouch")
        };

        Context::Git {
            has_husky,
            hooks_installed,
        }
    }

    fn is_git(&self) -> bool {
        matches!(self, Context::Git { .. })
    }
}

fn hook_contains(path: &Path, needle: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.contains(needle))
        .unwrap_or(false)
}

// =============================================================================
// State Machine
// =============================================================================

/// Program states — the full lifecycle of a donttouch invocation.
enum State {
    /// Entry point: determine what to do based on command + filesystem
    Start { command: Command, ignoregit: bool },

    /// No config found, user ran init — write config and prompt
    ToInit { context: Context },

    /// Config file written, prompting user for patterns
    Initializing {
        config_path: PathBuf,
        context: Context,
    },

    /// Init complete, ask user if they want to lock
    EndInit { context: Context },

    /// Ask user if they want to install git hooks (git context only)
    OfferHooks { context: Context },

    /// Ask user if they want to inject agent instructions
    OfferInject { root: PathBuf },

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
                State::Start { command, ignoregit } => handle_start(command, ignoregit),
                State::ToInit { context } => handle_to_init(context),
                State::Initializing {
                    config_path,
                    context,
                } => handle_initializing(&config_path, context),
                State::EndInit { context } => handle_end_init(context),
                State::OfferHooks { context } => handle_offer_hooks(context),
                State::OfferInject { root } => handle_offer_inject(&root),
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
fn handle_start(command: Command, ignoregit: bool) -> State {
    match command {
        Command::Init => {
            if Path::new(".donttouch.toml").exists() {
                State::Error {
                    message: "⚠️  .donttouch.toml already exists. Nothing to do.".into(),
                }
            } else {
                let context = Context::detect(Path::new("."), ignoregit);
                State::ToInit { context }
            }
        }

        // All other commands require an existing config
        cmd => {
            let root = match &cmd {
                Command::Disable { target }
                | Command::Unlock { target }
                | Command::Remove { target } => match assert_outside(target) {
                    Ok(p) => p,
                    Err(e) => return State::Error { message: e },
                },
                _ => PathBuf::from("."),
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

            let context = Context::detect(&root, ignoregit);
            let patterns = compile_patterns(&config.protect.patterns);
            let files = discover_files(&root, &patterns);

            if config.protect.enabled {
                dispatch_enabled(cmd, config, files, root, context)
            } else {
                dispatch_disabled(cmd, config, files, root, context)
            }
        }
    }
}

/// Dispatch a command when state is Enabled.
fn dispatch_enabled(
    cmd: Command,
    config: ConfigFile,
    files: Vec<ProtectedFile>,
    root: PathBuf,
    context: Context,
) -> State {
    match cmd {
        Command::Status => do_status(&config, &files, true, &context),
        Command::Lock => do_lock(&files),
        Command::Unlock { .. } => do_unlock(&files, &root),
        Command::Check => do_check(&files, &root, &context),
        Command::CheckPush => do_check_push(true, &context),
        Command::Enable => State::Done {
            message: "✅ Protection is already enabled.".into(),
        },
        Command::Disable { .. } => do_disable(&files, &root),
        Command::Remove { .. } => do_remove(&files, &root, &context),
        Command::Inject { dry_run } => do_inject(&root, dry_run),
        Command::Why { ref file } => do_why(file, &config),
        Command::Init => unreachable!(),
    }
}

/// Dispatch a command when state is Disabled.
fn dispatch_disabled(
    cmd: Command,
    config: ConfigFile,
    files: Vec<ProtectedFile>,
    root: PathBuf,
    context: Context,
) -> State {
    match cmd {
        Command::Status => do_status(&config, &files, false, &context),
        Command::Lock => State::Error {
            message: "⏸️  Protection is disabled. Run 'donttouch enable' first.".into(),
        },
        Command::Unlock { .. } => do_unlock(&files, &root),
        Command::Check => State::Done {
            message: "⏸️  Protection is disabled. Skipping check.".into(),
        },
        Command::CheckPush => do_check_push(false, &context),
        Command::Enable => do_enable(&files, &root),
        Command::Disable { .. } => State::Done {
            message: "⏸️  Protection is already disabled.".into(),
        },
        Command::Remove { .. } => do_remove(&files, &root, &context),
        Command::Inject { dry_run } => do_inject(&root, dry_run),
        Command::Why { ref file } => do_why(file, &config),
        Command::Init => unreachable!(),
    }
}

/// ToInit: write the config file
fn handle_to_init(context: Context) -> State {
    let default_config = r#"# donttouch configuration
# Protect files from being modified by AI coding agents and accidental changes.

[protect]
enabled = true
patterns = []
"#;

    match std::fs::write(".donttouch.toml", default_config) {
        Ok(()) => State::Initializing {
            config_path: PathBuf::from(".donttouch.toml"),
            context,
        },
        Err(e) => State::Error {
            message: format!("Failed to create .donttouch.toml: {e}"),
        },
    }
}

/// Initializing: prompt user for patterns
fn handle_initializing(config_path: &Path, context: Context) -> State {
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
            Ok(0) => break,
            Ok(_) => {
                let trimmed = line.trim().to_string();
                if trimmed.is_empty() {
                    break;
                }
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

        println!(
            "\n📝 Saved {} pattern(s) to .donttouch.toml",
            patterns.len()
        );
    }

    State::EndInit { context }
}

/// EndInit: ask user if they want to lock now
fn handle_end_init(context: Context) -> State {
    println!();
    print!("Lock protected files now? [Y/n] ");
    io::stdout().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).ok();
    let answer = answer.trim().to_lowercase();

    if answer.is_empty() || answer == "y" || answer == "yes" {
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
            println!(
                "No files match the protected patterns. Add files and run 'donttouch lock' later."
            );
        } else {
            // Lock the files inline (don't return to state machine — we need to continue to hooks)
            let mut locked = 0;
            for f in &files {
                if !f.readonly && set_file_readonly(&f.path, true).is_ok() {
                    println!("   🔒 {}", f.path.display());
                    locked += 1;
                }
            }
            // Lock config too
            let config_path = Path::new(".donttouch.toml");
            if !is_file_readonly(config_path) && set_file_readonly(config_path, true).is_ok() {
                println!("   🔒 .donttouch.toml");
                locked += 1;
            }
            if locked > 0 {
                println!("\n✅ Locked {locked} file(s).");
            }
        }
    } else {
        println!("Ok. Run 'donttouch lock' when you're ready.");
    }

    // If git context, offer to install hooks, then inject
    if context.is_git() {
        State::OfferHooks { context }
    } else {
        State::OfferInject {
            root: PathBuf::from("."),
        }
    }
}

/// OfferHooks: ask user if they want to install git hooks
fn handle_offer_hooks(context: Context) -> State {
    let (has_husky, hooks_installed) = match &context {
        Context::Git {
            has_husky,
            hooks_installed,
        } => (*has_husky, *hooks_installed),
        Context::Plain => {
            return State::Done {
                message: String::new(),
            }
        }
    };

    let next = State::OfferInject {
        root: PathBuf::from("."),
    };

    if hooks_installed {
        println!("\n✅ Git hooks already installed.");
        return next;
    }

    if has_husky {
        println!("\n🐶 Husky detected.");
        print!("Install donttouch hooks into Husky? [Y/n] ");
    } else {
        print!("\nInstall git hooks (pre-commit + pre-push)? [Y/n] ");
    }
    io::stdout().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).ok();
    let answer = answer.trim().to_lowercase();

    if answer.is_empty() || answer == "y" || answer == "yes" {
        if has_husky {
            install_husky_hooks();
        } else {
            install_git_hooks();
        }
        println!("✅ Git hooks installed.");
        next
    } else {
        println!("Ok. Run 'donttouch init' in a git repo to install hooks later.");
        next
    }
}

// =============================================================================
// Git Hook Installation
// =============================================================================

fn install_git_hooks() {
    std::fs::create_dir_all(".git/hooks").ok();
    install_hook_file(
        Path::new(".git/hooks/pre-commit"),
        "donttouch check",
        "pre-commit",
    );
    install_hook_file(
        Path::new(".git/hooks/pre-push"),
        "donttouch check-push",
        "pre-push",
    );
}

fn install_husky_hooks() {
    install_hook_file(
        Path::new(".husky/pre-commit"),
        "donttouch check",
        "pre-commit",
    );
    install_hook_file(
        Path::new(".husky/pre-push"),
        "donttouch check-push",
        "pre-push",
    );
}

fn install_hook_file(path: &Path, donttouch_cmd: &str, hook_name: &str) {
    let snippet = format!(
        "\n# donttouch {hook_name} hook\nif command -v donttouch >/dev/null 2>&1; then\n    {donttouch_cmd}\nfi\n"
    );

    if path.exists() {
        let existing = std::fs::read_to_string(path).unwrap_or_default();
        if existing.contains("donttouch") {
            println!("   ✅ {hook_name} hook already contains donttouch.");
            return;
        }
        // Append to existing hook
        let appended = format!("{existing}{snippet}");
        if std::fs::write(path, appended).is_ok() {
            make_executable(path);
            println!("   ✅ Added donttouch to existing {hook_name} hook.");
        }
    } else {
        let content = format!("#!/bin/sh\n{snippet}");
        if std::fs::write(path, content).is_ok() {
            make_executable(path);
            println!("   ✅ Installed {hook_name} hook.");
        }
    }
}

fn remove_hook_donttouch(path: &Path, hook_name: &str) {
    if !path.exists() {
        return;
    }
    let content = match std::fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return,
    };

    if !content.contains("donttouch") {
        return;
    }

    // Remove the donttouch block: from "# donttouch" comment through "fi\n"
    let lines: Vec<&str> = content.lines().collect();
    let mut new_lines: Vec<&str> = Vec::new();
    let mut skip = false;

    for line in &lines {
        if line.contains("# donttouch") {
            skip = true;
            continue;
        }
        if skip {
            if line.trim() == "fi" {
                skip = false;
                continue;
            }
            continue;
        }
        new_lines.push(line);
    }

    let new_content = new_lines.join("\n") + "\n";

    // If only shebang remains (or empty), remove the file
    let meaningful: Vec<&str> = new_lines
        .iter()
        .filter(|l| !l.trim().is_empty() && !l.starts_with("#!"))
        .copied()
        .collect();

    if meaningful.is_empty() {
        let _ = std::fs::remove_file(path);
        println!("   🗑️  Removed {hook_name} hook (was only donttouch).");
    } else {
        let _ = std::fs::write(path, new_content);
        println!("   ✅ Removed donttouch from {hook_name} hook.");
    }
}

#[cfg(unix)]
fn make_executable(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o755));
}

#[cfg(not(unix))]
fn make_executable(_path: &Path) {}

// =============================================================================
// Agent Instruction Injection
// =============================================================================

const MARKER: &str = "<!-- donttouch:managed -->";
const INSTRUCTION: &str = "<!-- donttouch:managed --> ⚠️ donttouch is active in this project. Files marked read-only are protected — do not modify, rename, or delete them. Run `donttouch status` to see which files are protected.";

const CURSOR_MDC_CONTENT: &str = r#"---
description: donttouch file protection
---
<!-- donttouch:managed --> ⚠️ donttouch is active in this project. Files marked read-only are protected — do not modify, rename, or delete them. Run `donttouch status` to see which files are protected.
"#;

/// Agent config files we look for (path relative to root, whether to append or create).
struct AgentTarget {
    path: &'static str,
    /// If true, create the file if it doesn't exist. If false, only inject into existing files.
    create: bool,
    /// If true, use the special Cursor MDC format
    cursor_mdc: bool,
}

const AGENT_TARGETS: &[AgentTarget] = &[
    AgentTarget {
        path: "CLAUDE.md",
        create: false,
        cursor_mdc: false,
    },
    AgentTarget {
        path: "AGENTS.md",
        create: false,
        cursor_mdc: false,
    },
    AgentTarget {
        path: ".cursor/rules/donttouch.mdc",
        create: true,
        cursor_mdc: true,
    },
    AgentTarget {
        path: "codex.md",
        create: false,
        cursor_mdc: false,
    },
    AgentTarget {
        path: ".github/copilot-instructions.md",
        create: false,
        cursor_mdc: false,
    },
];

fn handle_offer_inject(root: &Path) -> State {
    // Check if any agent files exist (or cursor dir exists)
    let has_targets = AGENT_TARGETS.iter().any(|t| {
        let path = root.join(t.path);
        if t.create {
            // For cursor, check if .cursor dir exists or if we should offer anyway
            true
        } else {
            path.exists()
        }
    });

    if !has_targets {
        return State::Done {
            message: String::new(),
        };
    }

    print!("\nAdd agent instructions to coding agent config files? [Y/n] ");
    io::stdout().flush().ok();

    let mut answer = String::new();
    io::stdin().read_line(&mut answer).ok();
    let answer = answer.trim().to_lowercase();

    if answer.is_empty() || answer == "y" || answer == "yes" {
        let result = inject_agent_instructions(root, false);
        State::Done { message: result }
    } else {
        State::Done {
            message: "Ok. Run 'donttouch inject' to add agent instructions later.".into(),
        }
    }
}

fn do_inject(root: &Path, dry_run: bool) -> State {
    let result = inject_agent_instructions(root, dry_run);
    if dry_run {
        State::Done {
            message: format!("Dry run:\n{result}"),
        }
    } else {
        State::Done { message: result }
    }
}

fn inject_agent_instructions(root: &Path, dry_run: bool) -> String {
    let mut out = String::new();
    let mut injected = 0;
    let mut skipped = 0;

    for target in AGENT_TARGETS {
        let path = root.join(target.path);

        if path.exists() {
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };

            // Already has our marker — skip
            if content.contains(MARKER) {
                out.push_str(&format!(
                    "   ✅ {} (already has instructions)\n",
                    target.path
                ));
                skipped += 1;
                continue;
            }

            if dry_run {
                out.push_str(&format!("   📝 Would inject into {}\n", target.path));
                injected += 1;
            } else {
                // Append instruction
                let new_content = if content.ends_with('\n') {
                    format!("{content}\n{INSTRUCTION}\n")
                } else {
                    format!("{content}\n\n{INSTRUCTION}\n")
                };
                match std::fs::write(&path, new_content) {
                    Ok(()) => {
                        out.push_str(&format!("   📝 Injected into {}\n", target.path));
                        injected += 1;
                    }
                    Err(e) => {
                        out.push_str(&format!("   ❌ Failed to write {}: {e}\n", target.path));
                    }
                }
            }
        } else if target.create && target.cursor_mdc {
            if dry_run {
                out.push_str(&format!("   📝 Would create {}\n", target.path));
                injected += 1;
            } else {
                // Create parent dirs
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent).ok();
                }
                match std::fs::write(&path, CURSOR_MDC_CONTENT) {
                    Ok(()) => {
                        out.push_str(&format!("   📝 Created {}\n", target.path));
                        injected += 1;
                    }
                    Err(e) => {
                        out.push_str(&format!("   ❌ Failed to create {}: {e}\n", target.path));
                    }
                }
            }
        }
        // If file doesn't exist and create is false, silently skip
    }

    if injected > 0 {
        out.push_str(&format!("\n✅ Injected into {injected} file(s)."));
    } else if skipped > 0 {
        out.push_str("\n✅ All agent files already have instructions.");
    } else {
        out.push_str("No agent config files found to inject into.");
    }

    out
}

/// Remove donttouch instructions from all agent files
fn remove_agent_instructions(root: &Path) {
    for target in AGENT_TARGETS {
        let path = root.join(target.path);
        if !path.exists() {
            continue;
        }

        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => continue,
        };

        if !content.contains(MARKER) {
            continue;
        }

        if target.cursor_mdc && target.create {
            // If this is our created file, just delete it
            let _ = std::fs::remove_file(&path);
            println!("   🗑️  Removed {}", target.path);
            continue;
        }

        // Remove lines containing the marker
        let new_lines: Vec<&str> = content.lines().filter(|l| !l.contains(MARKER)).collect();

        // Clean up double blank lines left behind
        let new_content = new_lines.join("\n").trim_end().to_string() + "\n";

        let _ = std::fs::write(&path, new_content);
        println!("   ✅ Removed donttouch instruction from {}", target.path);
    }
}

// =============================================================================
// Actions (return next State)
// =============================================================================

fn do_status(
    config: &ConfigFile,
    files: &[ProtectedFile],
    enabled: bool,
    context: &Context,
) -> State {
    let mut out = String::new();

    if enabled {
        out.push_str("🔒 Protection: enabled\n");
    } else {
        out.push_str("🔓 Protection: disabled\n");
    }

    // Context info
    match context {
        Context::Plain => {
            out.push_str("📁 Context: plain directory\n");
        }
        Context::Git {
            has_husky,
            hooks_installed,
        } => {
            out.push_str("📁 Context: git repository");
            if *has_husky {
                out.push_str(" (Husky detected)");
            }
            out.push('\n');

            if *hooks_installed {
                out.push_str("🪝 Hooks: installed\n");
            } else {
                out.push_str("🪝 Hooks: not installed (run 'donttouch init' to install)\n");
            }
        }
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
            let icon = if f.readonly {
                "🔒 read-only"
            } else {
                "🔓 writable"
            };
            out.push_str(&format!("   {icon}  {}\n", f.path.display()));
        }
    }

    State::Done { message: out }
}

fn do_lock(files: &[ProtectedFile]) -> State {
    let mut out = String::new();
    let mut locked = 0;
    let mut already = 0;

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

fn do_check(files: &[ProtectedFile], root: &Path, context: &Context) -> State {
    let mut issues = Vec::new();

    // Check 1: permission violations (all contexts)
    let writable: Vec<&ProtectedFile> = files.iter().filter(|f| !f.readonly).collect();
    if !writable.is_empty() {
        issues.push("Permission violations (files are writable):".to_string());
        for f in &writable {
            issues.push(format!("   • {}", f.path.display()));
        }
    }

    // Check 2: staged file violations (git context only)
    if let Context::Git { .. } = context {
        let patterns = files_to_patterns(root);
        if !patterns.is_empty() {
            let staged = get_staged_files(root);
            let staged_violations: Vec<&String> = staged
                .iter()
                .filter(|f| patterns.iter().any(|p| p.matches(f)))
                .collect();

            if !staged_violations.is_empty() {
                if !issues.is_empty() {
                    issues.push(String::new());
                }
                issues.push(
                    "Staged file violations (protected files in git staging area):".to_string(),
                );
                for f in &staged_violations {
                    issues.push(format!("   • {f}"));
                }
            }
        }
    }

    if issues.is_empty() {
        State::Done {
            message: "✅ All protected files are read-only.".into(),
        }
    } else {
        let mut out = String::from("🚫 donttouch check failed!\n\n");
        for line in &issues {
            out.push_str(line);
            out.push('\n');
        }
        out.push_str("\nRun 'donttouch lock' to fix permission issues.");
        State::Error { message: out }
    }
}

fn do_check_push(enabled: bool, context: &Context) -> State {
    if !context.is_git() {
        return State::Error {
            message: "🚫 check-push requires a git repository.".into(),
        };
    }

    if !enabled {
        State::Error {
            message: "🚫 donttouch: push blocked! Protection is currently disabled.\n\n\
                      You must re-enable protection before pushing:\n\
                      \n   donttouch enable\n\n\
                      This ensures protected files are checked before code leaves your machine."
                .into(),
        }
    } else {
        State::Done {
            message: "✅ donttouch is enabled. Push allowed.".into(),
        }
    }
}

fn do_enable(files: &[ProtectedFile], root: &Path) -> State {
    if let Err(e) = write_enabled(root, true) {
        return State::Error { message: e };
    }

    let mut out = String::new();
    let mut locked = 0;

    for f in files {
        if !f.readonly && set_file_readonly(&f.path, true).is_ok() {
            locked += 1;
        }
    }

    let config_path = root.join(".donttouch.toml");
    if !is_file_readonly(&config_path) && set_file_readonly(&config_path, true).is_ok() {
        locked += 1;
    }

    if locked > 0 {
        out.push_str(&format!("   🔒 Locked {locked} file(s).\n"));
    }
    out.push_str("✅ Protection enabled.");

    State::Done { message: out }
}

fn do_remove(files: &[ProtectedFile], root: &Path, context: &Context) -> State {
    let mut out = String::new();
    let mut unlocked = 0;

    for f in files {
        if f.readonly && set_file_readonly(&f.path, false).is_ok() {
            out.push_str(&format!("   🔓 {}\n", f.path.display()));
            unlocked += 1;
        }
    }

    // Unlock and delete config
    let config_path = root.join(".donttouch.toml");
    if is_file_readonly(&config_path) {
        let _ = set_file_readonly(&config_path, false);
    }
    match std::fs::remove_file(&config_path) {
        Ok(()) => {
            out.push_str(&format!("   🗑️  {}\n", config_path.display()));
        }
        Err(e) => {
            out.push_str(&format!(
                "   ❌ Failed to remove {}: {e}\n",
                config_path.display()
            ));
        }
    }

    // Clean up git hooks if applicable
    if let Context::Git { has_husky, .. } = context {
        if *has_husky {
            remove_hook_donttouch(&root.join(".husky/pre-commit"), "pre-commit");
            remove_hook_donttouch(&root.join(".husky/pre-push"), "pre-push");
        } else {
            remove_hook_donttouch(&root.join(".git/hooks/pre-commit"), "pre-commit");
            remove_hook_donttouch(&root.join(".git/hooks/pre-push"), "pre-push");
        }
    }

    // Clean up agent instructions
    remove_agent_instructions(root);

    if unlocked > 0 {
        out.push_str(&format!("\n   Unlocked {unlocked} file(s)."));
    }
    out.push_str("\n✅ donttouch removed.");

    State::Done { message: out }
}

fn do_why(file: &str, config: &ConfigFile) -> State {
    let matching: Vec<&String> = config
        .protect
        .patterns
        .iter()
        .filter(|p| {
            Pattern::new(p)
                .map(|pat| pat.matches(file))
                .unwrap_or(false)
        })
        .collect();

    if matching.is_empty() {
        State::Done {
            message: format!("{file} is not protected by any pattern."),
        }
    } else {
        let mut out = format!("{file} is protected by:\n");
        for p in &matching {
            out.push_str(&format!("   • {p}\n"));
        }
        State::Done { message: out }
    }
}

fn do_disable(files: &[ProtectedFile], root: &Path) -> State {
    let config_path = root.join(".donttouch.toml");
    if is_file_readonly(&config_path) {
        let _ = set_file_readonly(&config_path, false);
    }

    if let Err(e) = write_enabled(root, false) {
        return State::Error { message: e };
    }

    let mut unlocked = 0;
    for f in files {
        if f.readonly && set_file_readonly(&f.path, false).is_ok() {
            unlocked += 1;
        }
    }

    let mut out = String::new();
    if unlocked > 0 {
        out.push_str(&format!("   🔓 Unlocked {unlocked} file(s).\n"));
    }
    out.push_str(
        "🔓 Protection disabled.\n   ⚠️  You must run 'donttouch enable' before you can push.",
    );

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
        .and_then(std::fs::canonicalize)
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
// Git Helpers
// =============================================================================

fn get_staged_files(root: &Path) -> Vec<String> {
    let output = process::Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMRD"])
        .current_dir(root)
        .output();

    match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout)
            .lines()
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect(),
        _ => Vec::new(),
    }
}

/// Re-read config patterns from disk for git staged file checking.
/// (We need the raw patterns for matching against relative paths from git.)
fn files_to_patterns(root: &Path) -> Vec<Pattern> {
    let config_path = root.join(".donttouch.toml");
    let content = match std::fs::read_to_string(&config_path) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    let config: ConfigFile = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    compile_patterns(&config.protect.patterns)
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
    let meta =
        std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
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
    let meta =
        std::fs::metadata(path).map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
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
    let start = State::Start {
        command: cli.command,
        ignoregit: cli.ignoregit,
    };
    start.run();
}
