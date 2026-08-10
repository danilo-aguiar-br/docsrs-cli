//! Unit tests for argument parsing and CLI-level derivations.

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
    let cli = Cli::try_parse_from(["docsrs-cli", "version", "--json", "--format", "text"]).unwrap();
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
