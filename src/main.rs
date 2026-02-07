use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
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
    /// Check staged files against protected patterns (used by git hooks)
    Check,
    /// Show protected patterns and any modified protected files
    Status,
    /// Install git pre-commit hook
    Init,
}

#[derive(Deserialize)]
struct Config {
    protect: ProtectConfig,
}

#[derive(Deserialize)]
struct ProtectConfig {
    patterns: Vec<String>,
}

fn load_config() -> Result<Config, String> {
    let content = std::fs::read_to_string(".donttouch.toml")
        .map_err(|e| format!("Could not read .donttouch.toml: {e}"))?;
    let config: Config = toml::from_str(&content)
        .map_err(|e| format!("Invalid .donttouch.toml: {e}"))?;
    if config.protect.patterns.is_empty() {
        return Err("No patterns defined in [protect].patterns".into());
    }
    Ok(config)
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

    let files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    Ok(files)
}

fn get_modified_files() -> Result<Vec<String>, String> {
    let output = Command::new("git")
        .args(["diff", "--name-only", "--diff-filter=ACMRD"])
        .output()
        .map_err(|e| format!("Failed to run git: {e}"))?;

    let mut files: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter(|l| !l.is_empty())
        .map(String::from)
        .collect();

    // Also include staged
    if let Ok(staged) = get_staged_files() {
        for f in staged {
            if !files.contains(&f) {
                files.push(f);
            }
        }
    }

    Ok(files)
}

fn find_violations(files: &[String], patterns: &[Pattern]) -> Vec<String> {
    files
        .iter()
        .filter(|f| patterns.iter().any(|p| p.matches(f)))
        .cloned()
        .collect()
}

fn cmd_check() {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

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

fn cmd_init() {
    let hook_path = ".git/hooks/pre-commit";

    // Check we're in a git repo
    if !std::path::Path::new(".git").exists() {
        eprintln!("donttouch: not a git repository (no .git directory)");
        exit(1);
    }

    // Create hooks dir if needed
    std::fs::create_dir_all(".git/hooks").ok();

    let hook_content = r#"#!/bin/sh
# donttouch pre-commit hook
# Installed by: donttouch init

if command -v donttouch >/dev/null 2>&1; then
    donttouch check
else
    echo "warning: donttouch is not installed, skipping protected file check"
fi
"#;

    // Check for existing hook
    if std::path::Path::new(hook_path).exists() {
        let existing = std::fs::read_to_string(hook_path).unwrap_or_default();
        if existing.contains("donttouch") {
            println!("✅ Pre-commit hook already contains donttouch.");
            return;
        }
        // Append to existing hook
        let appended = format!("{existing}\n{hook_content}");
        std::fs::write(hook_path, appended).unwrap_or_else(|e| {
            eprintln!("donttouch: failed to update hook: {e}");
            exit(1);
        });
        println!("✅ Added donttouch to existing pre-commit hook.");
    } else {
        std::fs::write(hook_path, hook_content).unwrap_or_else(|e| {
            eprintln!("donttouch: failed to write hook: {e}");
            exit(1);
        });
        println!("✅ Installed pre-commit hook.");
    }

    // Make executable
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o755);
        std::fs::set_permissions(hook_path, perms).ok();
    }
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Check => cmd_check(),
        Commands::Status => cmd_status(),
        Commands::Init => cmd_init(),
    }
}
