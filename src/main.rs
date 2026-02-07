use clap::{Parser, Subcommand};
use glob::Pattern;
use serde::Deserialize;
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
    /// Create a default .donttouch.toml in the current directory
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

/// The resolved state of donttouch in a directory.
/// Derived from filesystem inspection — this is the single source of truth.
enum State {
    /// No .donttouch.toml found — not initialized
    Uninitialized,

    /// Config exists, protection is enabled
    Enabled {
        config: ConfigFile,
        files: Vec<ProtectedFile>,
        root: PathBuf,
    },

    /// Config exists, protection is disabled
    Disabled {
        config: ConfigFile,
        files: Vec<ProtectedFile>,
        root: PathBuf,
    },
}

/// A file matched by a protection pattern, with its current permission state.
struct ProtectedFile {
    path: PathBuf,
    readonly: bool,
}

/// Result of a state transition
enum Transition {
    /// State changed successfully
    Ok(String),
    /// Action is not valid in this state
    InvalidAction(String),
    /// Action failed
    Error(String),
}

impl State {
    /// Inspect a directory and derive the state.
    fn resolve(dir: &Path) -> Self {
        let config_path = dir.join(".donttouch.toml");
        let content = match std::fs::read_to_string(&config_path) {
            Ok(c) => c,
            Err(_) => return State::Uninitialized,
        };

        let config: ConfigFile = match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("donttouch: invalid {}: {e}", config_path.display());
                exit(1);
            }
        };

        let patterns: Vec<Pattern> = config
            .protect
            .patterns
            .iter()
            .filter_map(|p| match Pattern::new(p) {
                Ok(pat) => Some(pat),
                Err(e) => {
                    eprintln!("donttouch: bad glob pattern '{p}': {e}");
                    None
                }
            })
            .collect();

        let files = discover_files(dir, &patterns);
        let root = dir.to_path_buf();

        if config.protect.enabled {
            State::Enabled { config, files, root }
        } else {
            State::Disabled { config, files, root }
        }
    }

    /// Execute a command against the current state, returning a transition.
    fn execute(self, cmd: &Command) -> Transition {
        match (cmd, &self) {
            // --- Init ---
            (Command::Init, State::Uninitialized) => do_init(),
            (Command::Init, _) => Transition::InvalidAction(
                "⚠️  .donttouch.toml already exists. Nothing to do.".into(),
            ),

            // --- Status (valid in any initialized state) ---
            (Command::Status, State::Uninitialized) => no_config(),
            (Command::Status, State::Enabled { config, files, .. }) => do_status(config, files, true),
            (Command::Status, State::Disabled { config, files, .. }) => do_status(config, files, false),

            // --- Lock ---
            (Command::Lock, State::Uninitialized) => no_config(),
            (Command::Lock, State::Enabled { files, .. }) => do_lock(files),
            (Command::Lock, State::Disabled { .. }) => Transition::InvalidAction(
                "⏸️  Protection is disabled. Run 'donttouch enable' first.".into(),
            ),

            // --- Unlock (requires outside check — handled before execute) ---
            (Command::Unlock { .. }, State::Uninitialized) => no_config(),
            (Command::Unlock { .. }, State::Enabled { files, .. }) => do_unlock(files),
            (Command::Unlock { .. }, State::Disabled { files, .. }) => do_unlock(files),

            // --- Check ---
            (Command::Check, State::Uninitialized) => no_config(),
            (Command::Check, State::Enabled { files, .. }) => do_check(files),
            (Command::Check, State::Disabled { .. }) => Transition::Ok(
                "⏸️  Protection is disabled. Skipping check.".into(),
            ),

            // --- Enable ---
            (Command::Enable, State::Uninitialized) => no_config(),
            (Command::Enable, State::Enabled { .. }) => Transition::Ok(
                "✅ Protection is already enabled.".into(),
            ),
            (Command::Enable, State::Disabled { files, root, .. }) => do_enable(files, root),

            // --- Disable (requires outside check — handled before execute) ---
            (Command::Disable { .. }, State::Uninitialized) => no_config(),
            (Command::Disable { .. }, State::Disabled { .. }) => Transition::Ok(
                "⏸️  Protection is already disabled.".into(),
            ),
            (Command::Disable { .. }, State::Enabled { files, root, .. }) => do_disable(files, root),
        }
    }
}

fn no_config() -> Transition {
    Transition::InvalidAction("No .donttouch.toml found. Run 'donttouch init' first.".into())
}

// =============================================================================
// Outside-Directory Check
// =============================================================================

/// Verify that the current working directory is strictly outside the target
/// directory. Uses canonical (real) paths to prevent symlink and ../.. tricks.
fn assert_outside(target: &str) -> Result<PathBuf, String> {
    // Resolve the target to its canonical (real, absolute, symlink-free) path
    let canonical_target = std::fs::canonicalize(target)
        .map_err(|e| format!("Cannot resolve target path '{target}': {e}"))?;

    // Verify the target contains a .donttouch.toml
    if !canonical_target.join(".donttouch.toml").exists() {
        return Err(format!(
            "No .donttouch.toml found in '{}'. Is this the right directory?",
            canonical_target.display()
        ));
    }

    // Get canonical cwd
    let canonical_cwd = std::env::current_dir()
        .and_then(|p| std::fs::canonicalize(p))
        .map_err(|e| format!("Cannot resolve current directory: {e}"))?;

    // Check: cwd must NOT be inside (or equal to) the target
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
// Transition Implementations
// =============================================================================

fn do_init() -> Transition {
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

    match std::fs::write(".donttouch.toml", default_config) {
        Ok(()) => Transition::Ok(
            "✅ Created .donttouch.toml\n   Edit it to add file patterns you want to protect.\n   Then run: donttouch lock".into(),
        ),
        Err(e) => Transition::Error(format!("Failed to create .donttouch.toml: {e}")),
    }
}

fn do_status(config: &ConfigFile, files: &[ProtectedFile], enabled: bool) -> Transition {
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

    Transition::Ok(out)
}

fn do_lock(files: &[ProtectedFile]) -> Transition {
    if files.is_empty() {
        return Transition::Ok("No files match the protected patterns.".into());
    }

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

    if locked > 0 {
        out.push_str(&format!("\n✅ Locked {locked} file(s)."));
    }
    if already > 0 {
        out.push_str(&format!("\n   ({already} already read-only)"));
    }
    if locked == 0 && already > 0 {
        out.push_str("\n✅ All protected files are already read-only.");
    }

    Transition::Ok(out)
}

fn do_unlock(files: &[ProtectedFile]) -> Transition {
    if files.is_empty() {
        return Transition::Ok("No files match the protected patterns.".into());
    }

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

    if unlocked > 0 {
        out.push_str(&format!("\n✅ Unlocked {unlocked} file(s)."));
    } else {
        out.push_str("All files were already writable.");
    }

    Transition::Ok(out)
}

fn do_check(files: &[ProtectedFile]) -> Transition {
    let writable: Vec<&ProtectedFile> = files.iter().filter(|f| !f.readonly).collect();

    if writable.is_empty() {
        Transition::Ok("✅ All protected files are read-only.".into())
    } else {
        let mut out = String::from("🚫 Protected files are writable!\n\n");
        for f in &writable {
            out.push_str(&format!("   • {}\n", f.path.display()));
        }
        out.push_str("\nRun 'donttouch lock' to make them read-only.");
        Transition::Error(out)
    }
}

fn do_enable(files: &[ProtectedFile], root: &Path) -> Transition {
    if let Err(e) = write_enabled(root, true) {
        return Transition::Error(e);
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

    if locked > 0 {
        out.push_str(&format!("   🔒 Locked {locked} file(s).\n"));
    }
    out.push_str("✅ Protection enabled.");

    Transition::Ok(out)
}

fn do_disable(files: &[ProtectedFile], root: &Path) -> Transition {
    if let Err(e) = write_enabled(root, false) {
        return Transition::Error(e);
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

    Transition::Ok(out)
}

// =============================================================================
// Filesystem Helpers
// =============================================================================

/// Walk the directory tree and find files matching any of the patterns.
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

        // Skip internal directories
        if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
            if name == ".git" || name == "target" || name == "node_modules" {
                continue;
            }
        }

        // Path relative to the project root
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

/// Write the `enabled` flag to .donttouch.toml in the given root directory
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

    // Commands that require being outside the target directory
    match &cli.command {
        Command::Disable { target } | Command::Unlock { target } => {
            let canonical_target = match assert_outside(target) {
                Ok(p) => p,
                Err(e) => {
                    eprintln!("{e}");
                    exit(1);
                }
            };

            let state = State::resolve(&canonical_target);
            match state.execute(&cli.command) {
                Transition::Ok(msg) => println!("{msg}"),
                Transition::InvalidAction(msg) | Transition::Error(msg) => {
                    eprintln!("{msg}");
                    exit(1);
                }
            }
        }

        // All other commands operate on cwd
        _ => {
            let state = State::resolve(Path::new("."));
            match state.execute(&cli.command) {
                Transition::Ok(msg) => println!("{msg}"),
                Transition::InvalidAction(msg) | Transition::Error(msg) => {
                    eprintln!("{msg}");
                    exit(1);
                }
            }
        }
    }
}
