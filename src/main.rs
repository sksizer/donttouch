use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
use std::path::Path;
use std::process::{Command, exit};

/// Protect files from being modified by AI coding agents and accidental commits.
#[derive(Parser)]
#[command(name = "donttouch", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Check staged files against protected patterns (used by pre-commit hook)
    Check,
    /// Check commits about to be pushed (used by pre-push hook)
    #[command(name = "check-push")]
    CheckPush {
        /// Remote name (passed by git)
        #[arg(default_value = "origin")]
        remote: String,
        /// Remote URL (passed by git)
        #[arg(default_value = "")]
        url: String,
    },
    /// Show protected patterns and any modified protected files
    Status,
    /// Install git pre-commit and pre-push hooks
    Init,
    /// Disable pre-commit checking (push enforcement stays active)
    Disable,
    /// Re-enable pre-commit checking
    Enable,
}

#[derive(Deserialize)]
struct Config {
    protect: ProtectConfig,
}

#[derive(Deserialize)]
struct ProtectConfig {
    patterns: Vec<String>,
    #[serde(default = "default_enabled")]
    enabled: bool,
}

fn default_enabled() -> bool {
    true
}

fn load_config_from(path: &Path) -> Result<Config, String> {
    let config_path = path.join(".donttouch.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Could not read {}: {e}", config_path.display()))?;
    let config: Config = toml::from_str(&content)
        .map_err(|e| format!("Invalid {}: {e}", config_path.display()))?;
    if config.protect.patterns.is_empty() {
        return Err("No patterns defined in [protect].patterns".into());
    }
    Ok(config)
}

fn load_config() -> Result<Config, String> {
    load_config_from(Path::new("."))
}

fn compile_patterns(raw: &[String]) -> Result<Vec<Pattern>, String> {
    raw.iter()
        .map(|p| Pattern::new(p).map_err(|e| format!("Bad glob pattern '{p}': {e}")))
        .collect()
}

fn get_staged_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--cached", "--name-only", "--diff-filter=ACMRD"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    if !output.status.success() {
        return Err("git diff --cached failed (are you in a git repo?)".into());
    }

    Ok(parse_lines(&output.stdout))
}

fn get_modified_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=ACMRD"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let mut files = parse_lines(&output.stdout);

    if let Ok(staged) = get_staged_files() {
        for f in staged {
            if !files.contains(&f) {
                files.push(f);
            }
        }
    }

    Ok(files)
}

/// Get files changed in commits that would be pushed (compared to remote tracking branch)
fn get_push_files(remote: &str) -> Result<Vec<String>, String> {
    // Get current branch
    let branch_output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .output()
        .map_err(|e| format!("Failed to get current branch: {e}"))?;

    let branch = String::from_utf8_lossy(&branch_output.stdout).trim().to_string();

    // Try to find the remote tracking ref
    let remote_ref = format!("{remote}/{branch}");

    // Check if remote ref exists
    let has_remote = Command::new("git")
        .args(["rev-parse", "--verify", &remote_ref])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    let output = if has_remote {
        // Compare against remote tracking branch
        Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=ACMRD", &format!("{remote_ref}..HEAD")])
            .output()
            .map_err(|e| format!("Failed to diff against remote: {e}"))?
    } else {
        // No remote ref yet (first push) — check all commits
        Command::new("git")
            .args(["diff", "--name-only", "--diff-filter=ACMRD", "--root", "HEAD"])
            .output()
            .map_err(|e| format!("Failed to diff: {e}"))?
    };

    Ok(parse_lines(&output.stdout))
}

fn parse_lines(bytes: &[u8]) -> Vec<String> {
    String::from_utf8_lossy(bytes)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect()
}

fn find_violations(files: &[String], patterns: &[Pattern]) -> Vec<String> {
    files
        .iter()
        .filter(|f| patterns.iter().any(|p| p.matches(f)))
        .cloned()
        .collect()
}

fn set_enabled(repo_path: &Path, enabled: bool) -> Result<(), String> {
    let config_path = repo_path.join(".donttouch.toml");
    let content = std::fs::read_to_string(&config_path)
        .map_err(|e| format!("Could not read {}: {e}", config_path.display()))?;

    // Simple TOML rewrite: replace or insert enabled field
    let new_content = if content.contains("enabled") {
        // Replace existing enabled line
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed.starts_with("enabled") && trimmed.contains('=') {
                *line = format!("enabled = {enabled}");
            }
        }
        lines.join("\n") + "\n"
    } else {
        // Insert after [protect] header
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        let mut insert_idx = None;
        for (i, line) in lines.iter().enumerate() {
            if line.trim() == "[protect]" {
                insert_idx = Some(i + 1);
                break;
            }
        }
        if let Some(idx) = insert_idx {
            lines.insert(idx, format!("enabled = {enabled}"));
        }
        lines.join("\n") + "\n"
    };

    std::fs::write(&config_path, new_content)
        .map_err(|e| format!("Failed to write {}: {e}", config_path.display()))?;

    Ok(())
}

fn cmd_check() {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    // Respect enabled flag for pre-commit
    if !config.protect.enabled {
        println!("⏸️  donttouch: pre-commit checking is disabled. Push enforcement still active.");
        return;
    }

    let patterns = match compile_patterns(&config.protect.patterns) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let staged = match get_staged_files() {
        Ok(f) => f,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let violations = find_violations(&staged, &patterns);

    if violations.is_empty() {
        println!("✅ No protected files in staged changes.");
    } else {
        eprintln!("🚫 donttouch: commit blocked! Protected files were modified:\n");
        for f in &violations {
            eprintln!("   • {f}");
        }
        eprintln!("\nThese files are protected by .donttouch.toml.");
        eprintln!("If this is intentional, use: donttouch allow -- git commit");
        exit(1);
    }
}

fn cmd_check_push(remote: &str) {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    // NOTE: ignores `enabled` flag — push checking is ALWAYS active
    let patterns = match compile_patterns(&config.protect.patterns) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let files = match get_push_files(remote) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let violations = find_violations(&files, &patterns);

    if violations.is_empty() {
        println!("✅ No protected files in outgoing commits.");
    } else {
        eprintln!("🚫 donttouch: push blocked! Protected files were modified in outgoing commits:\n");
        for f in &violations {
            eprintln!("   • {f}");
        }
        eprintln!("\nThese files are protected by .donttouch.toml.");
        eprintln!("Push enforcement cannot be disabled.");
        exit(1);
    }
}

fn cmd_status() {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    println!("Protected patterns:");
    for p in &config.protect.patterns {
        println!("   • {p}");
    }

    if !config.protect.enabled {
        println!("\n⏸️  Pre-commit checking is DISABLED (push enforcement still active)");
    } else {
        println!("\n✅ Pre-commit checking is enabled");
    }

    let patterns = match compile_patterns(&config.protect.patterns) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let modified = get_modified_files().unwrap_or_default();
    let violations = find_violations(&modified, &patterns);

    if violations.is_empty() {
        println!("\n✅ No protected files have uncommitted changes.");
    } else {
        println!("\n⚠️  Modified protected files:");
        for f in &violations {
            println!("   • {f}");
        }
    }
}

fn install_hook(hook_name: &str, donttouch_cmd: &str) {
    let hook_path = format!(".git/hooks/{hook_name}");

    let hook_content = format!(
        r#"#!/bin/sh
# donttouch {hook_name} hook
# Installed by: donttouch init

if command -v donttouch >/dev/null 2>&1; then
    {donttouch_cmd}
else
    echo "warning: donttouch is not installed, skipping protected file check"
fi
"#
    );

    if Path::new(&hook_path).exists() {
        let existing = std::fs::read_to_string(&hook_path).unwrap_or_default();
        if existing.contains("donttouch") {
            println!("✅ {hook_name} hook already contains donttouch.");
            return;
        }
        let appended = format!("{existing}\n{hook_content}");
        std::fs::write(&hook_path, appended).unwrap_or_else(|e| {
            eprintln!("donttouch: failed to update {hook_name} hook: {e}");
            exit(1);
        });
        println!("✅ Added donttouch to existing {hook_name} hook.");
    } else {
        std::fs::write(&hook_path, &hook_content).unwrap_or_else(|e| {
            eprintln!("donttouch: failed to write {hook_name} hook: {e}");
            exit(1);
        });
        println!("✅ Installed {hook_name} hook.");
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(&hook_path, perms).ok();
    }
}

fn cmd_init() {
    if !Path::new(".git").exists() {
        eprintln!("donttouch: not a git repository (no .git directory)");
        exit(1);
    }

    std::fs::create_dir_all(".git/hooks").ok();

    install_hook("pre-commit", "donttouch check");
    install_hook("pre-push", "donttouch check-push \"$1\" \"$2\"");
}

fn cmd_disable() {
    match set_enabled(Path::new("."), false) {
        Ok(()) => {
            println!("⏸️  Pre-commit checking disabled.");
            println!("   Push enforcement remains active — protected files still can't be pushed.");
            println!("   Re-enable with: donttouch enable");
        }
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    }
}

fn cmd_enable() {
    match set_enabled(Path::new("."), true) {
        Ok(()) => println!("✅ Pre-commit checking enabled."),
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check => cmd_check(),
        Commands::CheckPush { remote, .. } => cmd_check_push(&remote),
        Commands::Status => cmd_status(),
        Commands::Init => cmd_init(),
        Commands::Disable => cmd_disable(),
        Commands::Enable => cmd_enable(),
    }
}
