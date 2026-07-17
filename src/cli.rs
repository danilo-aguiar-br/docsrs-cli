//! Clap surface for docsrs-cli.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "docsrs-cli",
    version,
    about = "One-shot CLI for crates.io and docs.rs documentation",
    long_about = None,
    disable_help_subcommand = false
)]
pub struct Cli {
    /// Emit JSON envelope on stdout
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format (json is alias of --json)
    #[arg(long, global = true, value_enum)]
    pub format: Option<OutputFormat>,

    /// Wall-clock timeout in seconds
    #[arg(long, global = true, env = "DOCSRS_CLI_TIMEOUT_SECS")]
    pub timeout: Option<u64>,

    /// Connect timeout in seconds
    #[arg(long, global = true)]
    pub connect_timeout: Option<u64>,

    /// Override User-Agent
    #[arg(long, global = true, env = "DOCSRS_CLI_USER_AGENT")]
    pub user_agent: Option<String>,

    /// Force human message locale (en or pt-BR)
    #[arg(long, global = true, env = "DOCSRS_CLI_LANG")]
    pub lang: Option<String>,

    /// Override XDG config directory
    #[arg(long, global = true, env = "DOCSRS_CLI_CONFIG_DIR")]
    pub config_dir: Option<PathBuf>,

    /// Override XDG cache directory for HTTP body cache
    #[arg(long, global = true, env = "DOCSRS_CLI_CACHE_DIR")]
    pub cache_dir: Option<PathBuf>,

    /// Disable disk cache (always hit network)
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Disk cache TTL in seconds (default 86400 = 24h)
    #[arg(long, global = true, env = "DOCSRS_CLI_CACHE_TTL_SECS")]
    pub cache_ttl_secs: Option<u64>,

    /// Soft cap on disk cache size in bytes (default 268435456 = 256 MiB; 0 = unlimited)
    #[arg(long, global = true, env = "DOCSRS_CLI_MAX_CACHE_BYTES")]
    pub max_cache_bytes: Option<u64>,

    /// Plan URLs without opening network sockets
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Cap downloaded body size in bytes
    #[arg(long, global = true)]
    pub max_body_bytes: Option<u64>,

    /// Cap emitted payload size in bytes
    #[arg(long, global = true, env = "DOCSRS_CLI_MAX_OUTPUT_BYTES")]
    pub max_output_bytes: Option<u64>,

    /// Minimum delay between requests to the same host (ms)
    #[arg(long, global = true)]
    pub rate_limit_delay_ms: Option<u64>,

    /// Max HTTP retries for transient errors
    #[arg(long, global = true)]
    pub max_retries: Option<u32>,

    /// Increase stderr verbosity
    #[arg(short = 'v', long, global = true, action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Suppress non-error stderr prose
    #[arg(short = 'q', long, global = true)]
    pub quiet: bool,

    /// Disable ANSI colors on stderr
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    Markdown,
    Json,
    Text,
}

#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Search crates on crates.io
    SearchCrates {
        /// Search keywords (English)
        query: String,
        /// Results per page (max 100)
        #[arg(long, default_value_t = 10)]
        per_page: u32,
        /// Sort order
        #[arg(long, value_enum, default_value_t = SortKind::Relevance)]
        sort: SortKind,
        /// Page number (1-based)
        #[arg(long, default_value_t = 1)]
        page: u32,
    },
    /// Fetch crate overview docblock from docs.rs (not git README)
    Readme {
        crate_name: String,
        #[arg(long)]
        crate_version: Option<String>,
    },
    /// Fetch documentation for a typed item
    GetItem {
        crate_name: String,
        item_type: String,
        item_path: String,
        #[arg(long)]
        crate_version: Option<String>,
    },
    /// Search symbols in crate all.html index
    SearchInCrate {
        crate_name: String,
        /// Substring filter; empty string lists all classified items
        #[arg(default_value = "")]
        query: String,
        #[arg(long)]
        crate_version: Option<String>,
        #[arg(long)]
        item_type: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// Print binary version
    Version,
    /// Validate local TLS/config readiness
    Doctor,
    /// Emit JSON Schema for a command
    Schema {
        #[arg(long)]
        cmd: String,
    },
    /// Generate shell completions
    Completions {
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Inspect or clear the XDG HTTP disk cache
    Cache {
        #[command(subcommand)]
        action: CacheAction,
    },
}

#[derive(Debug, Clone, Subcommand)]
pub enum CacheAction {
    /// Delete all cached HTTP bodies under the cache dir
    Clear,
    /// Report entry count, total bytes, and budget
    Stats,
}

#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SortKind {
    Relevance,
    Downloads,
    #[value(name = "recent-downloads")]
    RecentDownloads,
    #[value(name = "recent-updates")]
    RecentUpdates,
    New,
    Alphabetical,
}

impl SortKind {
    pub fn as_api_str(self) -> &'static str {
        match self {
            Self::Relevance => "relevance",
            Self::Downloads => "downloads",
            Self::RecentDownloads => "recent-downloads",
            Self::RecentUpdates => "recent-updates",
            Self::New => "new",
            Self::Alphabetical => "alphabetical",
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    Elvish,
    /// Canonical CLI value is `power-shell`; `powershell` is accepted as alias.
    #[value(name = "power-shell", alias = "powershell")]
    PowerShell,
}

impl Shell {
    pub fn to_clap_shell(self) -> clap_complete::Shell {
        match self {
            Self::Bash => clap_complete::Shell::Bash,
            Self::Zsh => clap_complete::Shell::Zsh,
            Self::Fish => clap_complete::Shell::Fish,
            Self::Elvish => clap_complete::Shell::Elvish,
            Self::PowerShell => clap_complete::Shell::PowerShell,
        }
    }
}

impl Cli {
    /// Whether JSON envelope is requested.
    pub fn wants_json(&self) -> bool {
        self.json || matches!(self.format, Some(OutputFormat::Json))
    }

    /// Reject incompatible `--json` + `--format text|markdown`.
    pub fn validate_format_conflict(&self) -> Result<(), crate::error::AppError> {
        use crate::error::{AppError, ErrorKind};
        if self.json
            && matches!(
                self.format,
                Some(OutputFormat::Text | OutputFormat::Markdown)
            )
        {
            return Err(AppError::new(
                ErrorKind::Usage,
                "cannot combine --json with --format text or --format markdown",
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn sort_api_strings() {
        assert_eq!(SortKind::Relevance.as_api_str(), "relevance");
        assert_eq!(SortKind::Downloads.as_api_str(), "downloads");
        assert_eq!(SortKind::RecentDownloads.as_api_str(), "recent-downloads");
        assert_eq!(SortKind::RecentUpdates.as_api_str(), "recent-updates");
        assert_eq!(SortKind::New.as_api_str(), "new");
        assert_eq!(SortKind::Alphabetical.as_api_str(), "alphabetical");
    }

    #[test]
    fn wants_json_from_flag_or_format() {
        let a = Cli::try_parse_from(["docsrs-cli", "version", "--json"]).unwrap();
        assert!(a.wants_json());
        let b = Cli::try_parse_from(["docsrs-cli", "version", "--format", "json"]).unwrap();
        assert!(b.wants_json());
        let c = Cli::try_parse_from(["docsrs-cli", "version"]).unwrap();
        assert!(!c.wants_json());
    }

    #[test]
    fn format_conflict_detected() {
        let cli =
            Cli::try_parse_from(["docsrs-cli", "version", "--json", "--format", "text"]).unwrap();
        assert!(cli.validate_format_conflict().is_err());
        let ok = Cli::try_parse_from(["docsrs-cli", "version", "--json"]).unwrap();
        assert!(ok.validate_format_conflict().is_ok());
    }

    #[test]
    fn shell_mapping() {
        assert!(matches!(
            Shell::Bash.to_clap_shell(),
            clap_complete::Shell::Bash
        ));
        assert!(matches!(
            Shell::Zsh.to_clap_shell(),
            clap_complete::Shell::Zsh
        ));
        assert!(matches!(
            Shell::Fish.to_clap_shell(),
            clap_complete::Shell::Fish
        ));
        assert!(matches!(
            Shell::Elvish.to_clap_shell(),
            clap_complete::Shell::Elvish
        ));
        assert!(matches!(
            Shell::PowerShell.to_clap_shell(),
            clap_complete::Shell::PowerShell
        ));
    }

    #[test]
    fn powershell_alias_and_canonical_parse() {
        let a = Cli::try_parse_from(["docsrs-cli", "completions", "powershell"]).unwrap();
        let b = Cli::try_parse_from(["docsrs-cli", "completions", "power-shell"]).unwrap();
        match (a.command, b.command) {
            (Commands::Completions { shell: sa }, Commands::Completions { shell: sb }) => {
                assert!(matches!(sa, Shell::PowerShell));
                assert!(matches!(sb, Shell::PowerShell));
            }
            _ => panic!("expected completions commands"),
        }
    }

    #[test]
    fn parse_search_crates_defaults() {
        let cli = Cli::try_parse_from(["docsrs-cli", "search-crates", "serde"]).unwrap();
        match cli.command {
            Commands::SearchCrates {
                query,
                per_page,
                sort,
                page,
            } => {
                assert_eq!(query, "serde");
                assert_eq!(per_page, 10);
                assert_eq!(sort, SortKind::Relevance);
                assert_eq!(page, 1);
            }
            _ => panic!("wrong command"),
        }
    }
}
