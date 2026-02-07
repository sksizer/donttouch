use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::exit;

/// Protect files from being modified by AI coding agents and accidental changes.
#[derive(Parser)]
#[command(name = "donttouch", version, about)]
struct Cli {
    /// Ignore git integration (treat directory as a plain directory)
    #[arg(long, global = true)]
    ignoregit: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a default .donttouch.toml in the current directory
    Init,
    /// List protected files and their current state (writable/read-only)
    Status,
    /// Make all protected files read-only
    Lock,
    /// Restore write permissions on protected files
    Unlock,
    /// Check if any protected files are writable (exits non-zero if so)
    Check,
    /// Disable protection (unlock files, skip checks). Push blocked until re-enabled.
    Disable,
    /// Re-enable protection (lock files, resume checks)
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

// --- Config ---

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

fn set_enabled(enabled: bool) -> Result<(), String> {
    let config_path = Path::new(".donttouch.toml");
    let content = std::fs::read_to_string(config_path)
        .map_err(|e| format!("Could not read .donttouch.toml: {e}"))?;

    let new_content = if content.contains("enabled") {
        let mut lines: Vec<String> = content.lines().map(String::from).collect();
        for line in &mut lines {
            let trimmed = line.trim();
            if trimmed.starts_with("enabled") && trimmed.contains('=') {
                *line = format!("enabled = {enabled}");
            }
        }
        lines.join("\n") + "\n"
    } else {
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

    std::fs::write(config_path, new_content)
        .map_err(|e| format!("Failed to write .donttouch.toml: {e}"))?;

    Ok(())
}

// --- Pattern matching ---

fn compile_patterns(raw: &[String]) -> Result<Vec<Pattern>, String> {
    raw.iter()
        .map(|p| Pattern::new(p).map_err(|e| format!("Bad glob pattern '{p}': {e}")))
        .collect()
}

/// Walk the directory and find all files matching protected patterns.
/// Respects .gitignore-style semantics (patterns match relative paths).
fn find_protected_files(patterns: &[Pattern]) -> Vec<PathBuf> {
    let mut results = Vec::new();
    walk_dir(Path::new("."), patterns, &mut results);
    results.sort();
    results
}

fn walk_dir(dir: &Path, patterns: &[Pattern], results: &mut Vec<PathBuf>) {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();

        // Skip .git directory and donttouch's own files
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".git" || name == "target" {
                continue;
            }
        }

        // Get path relative to cwd
        let rel = path.strip_prefix("./").unwrap_or(&path);
        let rel_str = rel.to_string_lossy();

        if path.is_dir() {
            // Check if directory matches a pattern like "secrets/**"
            walk_dir(&path, patterns, results);
        } else if patterns.iter().any(|p| p.matches(&rel_str)) {
            results.push(rel.to_path_buf());
        }
    }
}

// --- File permissions ---

#[cfg(unix)]
fn is_readonly(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(meta) = std::fs::metadata(path) {
        let mode = meta.permissions().mode();
        // Check if owner write bit is unset
        (mode & 0o200) == 0
    } else {
        false
    }
}

#[cfg(not(unix))]
fn is_readonly(path: &Path) -> bool {
    if let Ok(meta) = std::fs::metadata(path) {
        meta.permissions().readonly()
    } else {
        false
    }
}

#[cfg(unix)]
fn set_readonly(path: &Path, readonly: bool) -> Result<(), String> {
    use std::os::unix::fs::PermissionsExt;
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut mode = meta.permissions().mode();
    if readonly {
        mode &= !0o222; // Remove all write bits
    } else {
        mode |= 0o200; // Add owner write bit
    }
    let perms = std::fs::Permissions::from_mode(mode);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Cannot set permissions on {}: {e}", path.display()))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_readonly(path: &Path, readonly: bool) -> Result<(), String> {
    let meta = std::fs::metadata(path)
        .map_err(|e| format!("Cannot read {}: {e}", path.display()))?;
    let mut perms = meta.permissions();
    perms.set_readonly(readonly);
    std::fs::set_permissions(path, perms)
        .map_err(|e| format!("Cannot set permissions on {}: {e}", path.display()))?;
    Ok(())
}

// --- Commands ---

fn cmd_init() {
    if Path::new(".donttouch.toml").exists() {
        println!("⚠️  .donttouch.toml already exists.");
        return;
    }

    let default_config = r#"# donttouch configuration
# Protect files from being modified by AI coding agents and accidental changes.
# Add glob patterns for files that should be protected.

[protect]
enabled = true
patterns = [
    # Examples:
    # ".env",
    # ".env.*",
    # "secrets/**",
    # "docker-compose.prod.yml",
]
"#;

    std::fs::write(".donttouch.toml", default_config).unwrap_or_else(|e| {
        eprintln!("donttouch: failed to create .donttouch.toml: {e}");
        exit(1);
    });

    println!("✅ Created .donttouch.toml");
    println!("   Edit it to add file patterns you want to protect.");
    println!("   Then run: donttouch lock");
}

fn cmd_status() {
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

    // Show enabled state
    if config.protect.enabled {
        println!("🔒 Protection: enabled");
    } else {
        println!("🔓 Protection: disabled");
    }

    println!("\nPatterns:");
    for p in &config.protect.patterns {
        println!("   {p}");
    }

    // Find and show files
    let files = find_protected_files(&patterns);

    if files.is_empty() {
        println!("\nNo files currently match the protected patterns.");
    } else {
        println!("\nProtected files:");
        for f in &files {
            let state = if is_readonly(f) { "🔒 read-only" } else { "🔓 writable" };
            println!("   {state}  {}", f.display());
        }
    }
}

fn cmd_lock() {
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

    let files = find_protected_files(&patterns);

    if files.is_empty() {
        println!("No files match the protected patterns.");
        return;
    }

    let mut locked = 0;
    let mut already = 0;

    for f in &files {
        if is_readonly(f) {
            already += 1;
        } else {
            match set_readonly(f, true) {
                Ok(()) => {
                    println!("   🔒 {}", f.display());
                    locked += 1;
                }
                Err(e) => eprintln!("   ❌ {e}"),
            }
        }
    }

    if locked > 0 {
        println!("\n✅ Locked {locked} file(s).");
    }
    if already > 0 {
        println!("   ({already} already read-only)");
    }
}

fn cmd_unlock() {
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

    let files = find_protected_files(&patterns);

    if files.is_empty() {
        println!("No files match the protected patterns.");
        return;
    }

    let mut unlocked = 0;

    for f in &files {
        if is_readonly(f) {
            match set_readonly(f, false) {
                Ok(()) => {
                    println!("   🔓 {}", f.display());
                    unlocked += 1;
                }
                Err(e) => eprintln!("   ❌ {e}"),
            }
        }
    }

    if unlocked > 0 {
        println!("\n✅ Unlocked {unlocked} file(s).");
    } else {
        println!("All files were already writable.");
    }
}

fn cmd_check() {
    let config = match load_config() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    if !config.protect.enabled {
        println!("⏸️  donttouch: protection is disabled.");
        return;
    }

    let patterns = match compile_patterns(&config.protect.patterns) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    };

    let files = find_protected_files(&patterns);
    let writable: Vec<&PathBuf> = files.iter().filter(|f| !is_readonly(f)).collect();

    if writable.is_empty() {
        println!("✅ All protected files are read-only.");
    } else {
        eprintln!("🚫 donttouch: protected files are writable!\n");
        for f in &writable {
            eprintln!("   • {}", f.display());
        }
        eprintln!("\nRun 'donttouch lock' to make them read-only.");
        exit(1);
    }
}

fn cmd_disable() {
    match set_enabled(false) {
        Ok(()) => {
            // Also unlock files
            let config = load_config().ok();
            if let Some(config) = config {
                if let Ok(patterns) = compile_patterns(&config.protect.patterns) {
                    let files = find_protected_files(&patterns);
                    for f in &files {
                        if is_readonly(f) {
                            let _ = set_readonly(f, false);
                        }
                    }
                }
            }
            println!("🔓 Protection disabled. Files unlocked.");
            println!("   ⚠️  You must run 'donttouch enable' before you can push.");
        }
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    }
}

fn cmd_enable() {
    match set_enabled(true) {
        Ok(()) => {
            // Also lock files
            let config = load_config().ok();
            if let Some(config) = config {
                if let Ok(patterns) = compile_patterns(&config.protect.patterns) {
                    let files = find_protected_files(&patterns);
                    let mut locked = 0;
                    for f in &files {
                        if !is_readonly(f) {
                            if set_readonly(f, true).is_ok() {
                                locked += 1;
                            }
                        }
                    }
                    if locked > 0 {
                        println!("   🔒 Locked {locked} file(s).");
                    }
                }
            }
            println!("✅ Protection enabled.");
        }
        Err(e) => {
            eprintln!("donttouch: {e}");
            exit(1);
        }
    }
}

fn main() {
    let cli = Cli::parse();

    // ignoregit flag is available for future git integration
    // For now, all commands work in plain directories
    let _ = cli.ignoregit;

    match cli.command {
        Commands::Init => cmd_init(),
        Commands::Status => cmd_status(),
        Commands::Lock => cmd_lock(),
        Commands::Unlock => cmd_unlock(),
        Commands::Check => cmd_check(),
        Commands::Disable => cmd_disable(),
        Commands::Enable => cmd_enable(),
    }
}
