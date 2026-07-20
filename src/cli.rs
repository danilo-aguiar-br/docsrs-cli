//! Clap surface for docsrs-cli.

use std::path::PathBuf;

use clap::{Parser, Subcommand, ValueEnum};

#[derive(Debug, Clone, Parser)]
#[command(
    name = "docsrs-cli",
    version,
    about = "One-shot CLI for crates.io and docs.rs documentation",
    long_about = "One-shot BORN→EXECUTE→DIE CLI for crates.io and docs.rs.
\
Stdout is the data contract (JSON or markdown). Stderr is diagnostics only.
\
JSON is automatic when stdout is not a TTY; force human with --format markdown|text.
\
First Ctrl-C / SIGINT cancels cooperatively (exit 130). Second Ctrl-C within 5s force-exits 130.
\
SIGTERM and SIGHUP cancel as terminate (exit 143). On Windows, Ctrl+Break and console close also terminate (143).
\
Broken pipe on stdout is exit 141 (pipeline-safe; no stack trace).",
    after_help = "Examples:
  \
docsrs-cli search-crates serde --json
  \
docsrs-cli readme tokio --json
  \
docsrs-cli get-item clap trait clap::Parser --json
  \
docsrs-cli search-in-crate reqwest Client --json
  \
docsrs-cli doctor --json
  \
docsrs-cli config path --json
  \
docsrs-cli config show --json
  \
docsrs-cli config init --json
  \
docsrs-cli commands --json
  \
docsrs-cli completions bash
  \
docsrs-cli --dry-run readme serde
  \
# Agents: JSON is auto when stdout is not a TTY; force markdown with --format markdown",
    disable_help_subcommand = false
)]
/// Top-level clap parser for the `docsrs-cli` binary.
pub struct Cli {
    /// Emit JSON envelope on stdout
    #[arg(long, global = true)]
    pub json: bool,

    /// Output format (json is alias of --json)
    #[arg(long, global = true, value_enum)]
    pub format: Option<OutputFormat>,

    /// Wall-clock timeout in seconds (must be >= 1 when set)
    #[arg(long, global = true)]
    pub timeout: Option<u64>,

    /// Connect timeout in seconds (must be >= 1 when set)
    #[arg(long, global = true)]
    pub connect_timeout: Option<u64>,

    /// Override User-Agent
    #[arg(long, global = true)]
    pub user_agent: Option<String>,

    /// Force human stderr locale: en or pt-BR (fail-closed; JSON stays English)
    #[arg(long, global = true)]
    pub lang: Option<String>,

    /// Override XDG config directory
    #[arg(long, global = true)]
    pub config_dir: Option<PathBuf>,

    /// Override XDG cache directory for HTTP body cache
    #[arg(long, global = true)]
    pub cache_dir: Option<PathBuf>,

    /// Disable disk cache (always hit network)
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Disk cache TTL in seconds (default 86400 = 24h)
    #[arg(long, global = true)]
    pub cache_ttl_secs: Option<u64>,

    /// Soft cap on disk cache size in bytes (default 268435456 = 256 MiB; 0 = unlimited)
    #[arg(long, global = true)]
    pub max_cache_bytes: Option<u64>,

    /// Plan URLs without opening network sockets
    #[arg(long, global = true)]
    pub dry_run: bool,

    /// Cap downloaded body size in bytes (hard max 10485760 = 10 MiB; cannot raise above)
    #[arg(long, global = true)]
    pub max_body_bytes: Option<u64>,

    /// Cap emitted payload size in bytes (hard max 2097152 = 2 MiB; cannot raise above)
    #[arg(long, global = true)]
    pub max_output_bytes: Option<u64>,

    /// Minimum delay between requests to the same host (ms)
    #[arg(long, global = true)]
    pub rate_limit_delay_ms: Option<u64>,

    /// Max concurrent CPU parse workers (`0` = auto from CPUs and free RAM)
    #[arg(long, global = true)]
    pub max_concurrency: Option<u32>,

    /// Max HTTP retries for transient errors (after the first attempt)
    #[arg(long, global = true)]
    pub max_retries: Option<u32>,

    /// Base backoff delay in milliseconds for HTTP retries
    #[arg(long, global = true)]
    pub retry_base_ms: Option<u64>,

    /// Maximum single HTTP retry sleep in milliseconds
    #[arg(long, global = true)]
    pub retry_max_delay_ms: Option<u64>,

    /// Total HTTP retry wall budget in milliseconds (`0` = derive from --timeout)
    #[arg(long, global = true)]
    pub retry_max_elapsed_ms: Option<u64>,

    /// Disable HTTP retries (incident kill switch / debug)
    #[arg(long, global = true)]
    pub disable_retry: bool,

    /// Allow loopback origins (`127.0.0.1` / `localhost`) for offline wiremock
    ///
    /// Also settable via XDG `config.toml` `allow_loopback = true`. Never via env.
    #[arg(long, global = true)]
    pub allow_loopback: bool,

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
    /// Subcommand to execute for this invocation.
    pub command: Commands,
}

/// Human or machine output format selection.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum OutputFormat {
    /// Markdown rendered for humans.
    Markdown,
    /// JSON envelope for agents.
    Json,
    /// Plain-text rendering (alias of markdown for most commands).
    Text,
}

/// Product subcommands exposed on the CLI surface.
#[derive(Debug, Clone, Subcommand)]
pub enum Commands {
    /// Search crates on crates.io
    SearchCrates {
        /// Search keywords (English). Optional when `--page-token` carries a full query string.
        #[arg(default_value = "")]
        query: String,
        /// Results per page (max 100)
        #[arg(long, default_value_t = 10)]
        per_page: u32,
        /// Sort order
        #[arg(long, value_enum, default_value_t = SortKind::Relevance)]
        sort: SortKind,
        /// Page number (1-based). Conflicts with `--page-token`.
        #[arg(long, default_value_t = 1, conflicts_with = "page_token")]
        page: u32,
        /// Opaque pagination token from `meta.next_page` / `meta.prev_page` (full query or seek).
        #[arg(long, conflicts_with = "page")]
        page_token: Option<String>,
    },
    /// Fetch crate overview docblock from docs.rs (not git README)
    Readme {
        /// Crate name on crates.io or a stdlib crate (`name` or `name@1.2.3`).
        crate_name: String,
        /// Optional crate version (`latest` when omitted). Conflicts if it differs from `crate@…`.
        #[arg(long)]
        crate_version: Option<String>,
    },
    /// Fetch documentation for a typed item
    GetItem {
        /// Crate name on crates.io or a stdlib crate (`name` or `name@1.2.3`).
        crate_name: String,
        /// Kind: module|struct|trait|enum|union|fn|function|method|type|const|constant|static|macro|attr|attribute|derive
        item_type: String,
        /// Path with `::` or `/` (e.g. `Runtime::new`, `runtime/Runtime`, `tokio::spawn`)
        item_path: String,
        /// Optional crate version (`latest` when omitted). Conflicts if it differs from `crate@…`.
        #[arg(long)]
        crate_version: Option<String>,
        /// On 404, suggest nearby symbols from all.html (one extra request; exact/prefix/substring/edit-distance).
        #[arg(long)]
        suggest: bool,
    },
    /// Search symbols in crate all.html index
    SearchInCrate {
        /// Crate name on crates.io or a stdlib crate (`name` or `name@1.2.3`).
        crate_name: String,
        /// Text filter; empty string lists all classified items
        #[arg(default_value = "")]
        query: String,
        /// Optional crate version (`latest` when omitted). Conflicts if it differs from `crate@…`.
        #[arg(long)]
        crate_version: Option<String>,
        /// Optional item-kind filter (`struct`, `fn`, `method`, …).
        #[arg(long)]
        item_type: Option<String>,
        /// Maximum hits to emit (clamped to 1000).
        #[arg(long, default_value_t = 100)]
        limit: u32,
        /// Match policy: exact | prefix (default) | substring
        #[arg(long, default_value = "prefix")]
        r#match: String,
    },
    /// Print binary version
    Version,
    /// Validate local TLS/config readiness
    Doctor {
        /// Probe crates.io / docs.rs over the network (opt-in)
        #[arg(long)]
        online: bool,
    },
    /// List the full command tree for agent discovery
    Commands,
    /// Emit JSON Schema for a command
    Schema {
        /// Command name whose schema should be printed.
        #[arg(long)]
        cmd: String,
    },
    /// Generate shell completions
    Completions {
        /// Target shell for completion scripts.
        #[arg(value_enum)]
        shell: Shell,
    },
    /// Inspect or clear the XDG HTTP disk cache
    Cache {
        /// Cache maintenance action.
        #[command(subcommand)]
        action: CacheAction,
    },
    /// Manage XDG config paths and the optional config.toml (no secrets / no .env)
    Config {
        /// Config lifecycle action.
        #[command(subcommand)]
        action: ConfigAction,
    },
}

/// Cache maintenance subcommands.
#[derive(Debug, Clone, Subcommand)]
pub enum CacheAction {
    /// Print resolved cache root and which layer won
    Path,
    /// Delete all cached HTTP bodies under the cache dir
    Clear,
    /// Report entry count, total bytes, and budget
    Stats,
}

/// XDG config lifecycle subcommands (no API keys; product is public HTTP only).
#[derive(Debug, Clone, Subcommand)]
pub enum ConfigAction {
    /// Print resolved config/cache directories and which layer won
    Path,
    /// Print effective runtime configuration (merged defaults + TOML + env + CLI)
    Show,
    /// Create a default config.toml under the resolved config directory
    Init {
        /// Overwrite an existing config.toml
        #[arg(long)]
        force: bool,
    },
}

/// crates.io search sort order.
#[derive(Debug, Clone, Copy, ValueEnum, PartialEq, Eq)]
pub enum SortKind {
    /// Rank by search relevance.
    Relevance,
    /// Rank by total downloads.
    Downloads,
    /// Rank by recent downloads.
    #[value(name = "recent-downloads")]
    RecentDownloads,
    /// Rank by recent crate updates.
    #[value(name = "recent-updates")]
    RecentUpdates,
    /// Rank newest crates first.
    New,
    /// Rank alphabetically by crate name.
    Alphabetical,
}

impl SortKind {
    /// crates.io API `sort=` query value.
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

/// Shells supported by the `completions` subcommand.
#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum Shell {
    /// Bash completions.
    Bash,
    /// Zsh completions.
    Zsh,
    /// Fish completions.
    Fish,
    /// Elvish completions.
    Elvish,
    /// Canonical CLI value is `power-shell`; `powershell` is accepted as alias.
    #[value(name = "power-shell", alias = "powershell")]
    PowerShell,
}

impl Shell {
    /// Stable CLI / JSON name for this shell.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Bash => "bash",
            Self::Zsh => "zsh",
            Self::Fish => "fish",
            Self::Elvish => "elvish",
            Self::PowerShell => "power-shell",
        }
    }

    /// Convert to the `clap_complete` shell enum.
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

impl Commands {
    /// Stable wire command name used in success and error JSON envelopes (DRY).
    ///
    /// Nested cache/config actions use the same names as `schema --cmd` aliases
    /// (`cache-path`, `config-show`, …).
    pub fn wire_name(&self) -> &'static str {
        match self {
            Self::SearchCrates { .. } => "search-crates",
            Self::Readme { .. } => "readme",
            Self::GetItem { .. } => "get-item",
            Self::SearchInCrate { .. } => "search-in-crate",
            Self::Version => "version",
            Self::Doctor { .. } => "doctor",
            Self::Commands => "commands",
            Self::Schema { .. } => "schema",
            Self::Completions { .. } => "completions",
            Self::Cache { action } => match action {
                CacheAction::Path => "cache-path",
                CacheAction::Clear => "cache-clear",
                CacheAction::Stats => "cache-stats",
            },
            Self::Config { action } => match action {
                ConfigAction::Path => "config-path",
                ConfigAction::Show => "config-show",
                ConfigAction::Init { .. } => "config-init",
            },
        }
    }
}

impl Cli {
    /// Wire command name for the active subcommand (error/success envelopes).
    pub fn wire_command(&self) -> &'static str {
        self.command.wire_name()
    }

    /// Whether JSON envelope is requested.
    ///
    /// Agent-first rules (stdin/stdout LLM contract):
    /// - `--json` or `--format json` → JSON
    /// - `--format markdown|text` → human (never auto-JSON)
    /// - otherwise → JSON when stdout is not a TTY (pipe/agent), markdown on TTY
    pub fn wants_json(&self, stdout_is_terminal: bool) -> bool {
        if self.json || matches!(self.format, Some(OutputFormat::Json)) {
            return true;
        }
        if matches!(
            self.format,
            Some(OutputFormat::Markdown | OutputFormat::Text)
        ) {
            return false;
        }
        !stdout_is_terminal
    }

    /// Reject incompatible `--json` + `--format text|markdown`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::error::ErrorKind::Usage`] when `--json` is combined with
    /// `--format text` or `--format markdown`.
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
        assert!(a.wants_json(true));
        assert!(a.wants_json(false));
        let b = Cli::try_parse_from(["docsrs-cli", "version", "--format", "json"]).unwrap();
        assert!(b.wants_json(true));
        let c = Cli::try_parse_from(["docsrs-cli", "version"]).unwrap();
        assert!(!c.wants_json(true), "TTY defaults to human markdown");
        assert!(c.wants_json(false), "non-TTY auto-JSON for agents");
        let d = Cli::try_parse_from(["docsrs-cli", "version", "--format", "markdown"]).unwrap();
        assert!(!d.wants_json(false), "explicit format overrides auto-JSON");
        let e = Cli::try_parse_from(["docsrs-cli", "version", "--format", "text"]).unwrap();
        assert!(!e.wants_json(false));
    }

    #[test]
    fn parse_commands_subcommand() {
        let cli = Cli::try_parse_from(["docsrs-cli", "commands"]).unwrap();
        assert!(matches!(cli.command, Commands::Commands));
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
                page_token,
            } => {
                assert_eq!(query, "serde");
                assert_eq!(per_page, 10);
                assert_eq!(sort, SortKind::Relevance);
                assert_eq!(page, 1);
                assert!(page_token.is_none());
            }
            _ => panic!("wrong command"),
        }
    }
}
