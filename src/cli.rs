//! Command line argument parsing.
//!
//! Accepts both the WinMerge slash syntax (`/ignorews`, `/dl Mine`) and the
//! GNU-ish long/short forms this app shipped with (`--ignore-whitespace`, `-w`).
//! Option names are matched case-insensitively, like WinMerge.

use crate::diff::folder::CompareMethod;

pub const USAGE: &str = "\
WinXMerge — cross-platform file diff and merge tool

Usage:
  winxmerge [options] <left> <right>          2-way compare
  winxmerge [options] <base> <left> <right>   3-way merge

Compare options:
  /ignorews[:N]          Ignore whitespace differences
                         (:0 disables, :1 ignores whitespace changes, :2 ignores all whitespace)
  /ignorecase[:N]        Ignore letter case differences
  /ignoreblanklines[:N]  Ignore blank line differences
  /ignoreeol[:N]         Ignore line ending differences

Folder compare options:
  /m <method>            Full|Quick|Binary|Date|SizeDate|Size|Existence
                         (also accepted as /m:<method>)

Window options:
  /dl <desc>             Description shown for the left pane
  /dm <desc>             Description shown for the middle pane (3-way)
  /dr <desc>             Description shown for the right pane
  /l <n>                 Jump to line <n> after the initial compare
  /e                     Close the window with the Esc key

Automation:
  /x, /xq                Close automatically when the files are identical
  /enableexitcode        Exit with 0 (identical), 1 (different) or 2 (error)

Other:
  /?, --help, -h         Show this help
  --clear-history        Clear the session and recent file list, then exit
  --server               Start as the IPC server (Unix only)

Long forms --ignore-whitespace|-w, --ignore-case|-i and --ignore-blank-lines|-B
are kept as aliases of the matching /ignore* options.
";

#[derive(Debug, Default, PartialEq)]
pub struct CliArgs {
    pub paths: Vec<String>,
    pub help: bool,
    pub server: bool,
    pub clear_history: bool,
    // None = not specified on the command line (keep the saved setting).
    pub ignore_whitespace: Option<bool>,
    pub ignore_whitespace_all: Option<bool>,
    pub ignore_case: Option<bool>,
    pub ignore_blank_lines: Option<bool>,
    pub ignore_eol: Option<bool>,
    pub left_title: Option<String>,
    pub base_title: Option<String>,
    pub right_title: Option<String>,
    pub goto_line: Option<i32>,
    pub esc_closes: bool,
    pub auto_close_identical: bool,
    pub enable_exit_code: bool,
    pub folder_compare_method: Option<CompareMethod>,
}

/// Splits an option token into its lowercased name and optional `:value` part.
/// Returns `None` when the token is not an option (no `/` or `-` prefix, or an
/// existing path — Unix absolute paths start with `/` and files may start with `-`).
fn split_option(token: &str) -> Option<(String, Option<&str>)> {
    if !(token.starts_with('/') || token.starts_with('-')) {
        return None;
    }
    if std::path::Path::new(token).exists() {
        return None;
    }
    let body = token
        .strip_prefix("--")
        .or_else(|| token.strip_prefix('-'))
        .or_else(|| token.strip_prefix('/'))
        .unwrap_or(token);
    match body.split_once(':') {
        Some((name, value)) => Some((name.to_ascii_lowercase(), Some(value))),
        None => Some((body.to_ascii_lowercase(), None)),
    }
}

/// WinMerge treats `:0` as "off" and any other value (`:1`, `:2`) as "on".
/// `/ignorews` is the exception — see its match arm below, which distinguishes
/// `:1` (ignore whitespace change) from `:2` (ignore all whitespace).
fn flag_value(value: Option<&str>) -> bool {
    value != Some("0")
}

pub fn parse(args: &[String]) -> CliArgs {
    let mut cli = CliArgs::default();
    let mut i = 1; // skip argv[0]
    while i < args.len() {
        let arg = &args[i];
        i += 1;
        let (name, value) = match split_option(arg) {
            Some(parts) => parts,
            None => {
                cli.paths.push(arg.clone());
                continue;
            }
        };
        // Options whose value is the next argument, WinMerge style.
        let mut take_next = || {
            let v = args.get(i).cloned();
            if v.is_some() {
                i += 1;
            }
            v
        };
        match name.as_str() {
            "?" | "help" | "h" => cli.help = true,
            "server" => cli.server = true,
            "clear-history" => cli.clear_history = true,
            "ignorews" | "ignore-whitespace" | "w" => {
                // WinMerge: :0 = compare, :1 (and bare) = ignore change, :2 = ignore all.
                let on = value != Some("0");
                cli.ignore_whitespace = Some(on);
                cli.ignore_whitespace_all = Some(on && value == Some("2"));
            }
            "ignorecase" | "ignore-case" | "i" => cli.ignore_case = Some(flag_value(value)),
            "ignoreblanklines" | "ignore-blank-lines" | "b" => {
                cli.ignore_blank_lines = Some(flag_value(value))
            }
            "ignoreeol" | "ignore-eol" => cli.ignore_eol = Some(flag_value(value)),
            "dl" => cli.left_title = take_next(),
            "dm" => cli.base_title = take_next(),
            "dr" => cli.right_title = take_next(),
            "l" => cli.goto_line = take_next().and_then(|v| v.parse().ok()),
            "m" => {
                if let Some(v) = value.map(|s| s.to_string()).or_else(take_next) {
                    match CompareMethod::from_name(&v) {
                        Some(m) => cli.folder_compare_method = Some(m),
                        None => eprintln!("[winxmerge] unknown /m value ignored: {}", v),
                    }
                }
            }
            "e" => cli.esc_closes = true,
            "x" | "xq" => cli.auto_close_identical = true,
            "enableexitcode" => cli.enable_exit_code = true,
            _ => eprintln!("[winxmerge] unknown option ignored: {}", arg),
        }
    }
    cli
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_args(argv: &[&str]) -> CliArgs {
        let owned: Vec<String> = std::iter::once("winxmerge")
            .chain(argv.iter().copied())
            .map(String::from)
            .collect();
        parse(&owned)
    }

    #[test]
    fn collects_positional_paths() {
        let cli = parse_args(&["base.txt", "left.txt", "right.txt"]);
        assert_eq!(cli.paths, vec!["base.txt", "left.txt", "right.txt"]);
    }

    #[test]
    fn slash_options_are_not_paths() {
        let cli = parse_args(&["/ignorews", "/ignorecase", "a.txt", "b.txt"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_case, Some(true));
        assert_eq!(cli.paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn colon_zero_disables() {
        let cli = parse_args(&["/ignorews:0", "/ignoreeol:2"]);
        assert_eq!(cli.ignore_whitespace, Some(false));
        assert_eq!(cli.ignore_eol, Some(true));
    }

    #[test]
    fn ignorews_levels() {
        // WinMerge: :0 = off, :1 (and bare) = ignore whitespace change, :2 = ignore all.
        let cli = parse_args(&["/ignorews"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_whitespace_all, Some(false));

        let cli = parse_args(&["/ignorews:1"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_whitespace_all, Some(false));

        let cli = parse_args(&["/ignorews:2"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_whitespace_all, Some(true));

        let cli = parse_args(&["/ignorews:0"]);
        assert_eq!(cli.ignore_whitespace, Some(false));
        assert_eq!(cli.ignore_whitespace_all, Some(false));
    }

    #[test]
    fn unspecified_options_stay_none() {
        let cli = parse_args(&["a.txt", "b.txt"]);
        assert_eq!(cli.ignore_whitespace, None);
        assert_eq!(cli.ignore_eol, None);
    }

    #[test]
    fn value_options_consume_the_next_argument() {
        let cli = parse_args(&["/dl", "Mine", "/dr", "Theirs", "/l", "42", "a.txt", "b.txt"]);
        assert_eq!(cli.left_title.as_deref(), Some("Mine"));
        assert_eq!(cli.right_title.as_deref(), Some("Theirs"));
        assert_eq!(cli.goto_line, Some(42));
        assert_eq!(cli.paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn dangling_value_option_is_ignored() {
        let cli = parse_args(&["a.txt", "b.txt", "/dl"]);
        assert_eq!(cli.left_title, None);
        assert_eq!(cli.paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn option_names_are_case_insensitive() {
        let cli = parse_args(&["/IgnoreWS", "/EnableExitCode", "-B"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_blank_lines, Some(true));
        assert!(cli.enable_exit_code);
    }

    #[test]
    fn legacy_long_forms_still_work() {
        let cli = parse_args(&["--ignore-whitespace", "-i", "--server", "--clear-history"]);
        assert_eq!(cli.ignore_whitespace, Some(true));
        assert_eq!(cli.ignore_case, Some(true));
        assert!(cli.server);
        assert!(cli.clear_history);
    }

    #[test]
    fn unknown_options_are_dropped_not_treated_as_paths() {
        let cli = parse_args(&["/nosuchoption", "a.txt"]);
        assert_eq!(cli.paths, vec!["a.txt"]);
    }

    #[test]
    fn existing_paths_win_over_option_syntax() {
        // Unix absolute paths start with '/', so an existing path must never be
        // mistaken for an option.  A freshly created temp file rather than a
        // well-known one: /etc/hosts does not exist on Windows, where the parser
        // then reads "/etc/hosts" as an option and is right to.
        let path = std::env::temp_dir().join("winxmerge_cli_existing_path.txt");
        std::fs::write(&path, b"").unwrap();
        let arg = path.to_string_lossy().into_owned();
        let cli = parse_args(&[&arg]);
        let _ = std::fs::remove_file(&path);
        assert_eq!(cli.paths, vec![arg]);
    }

    #[test]
    fn help_is_recognised_in_every_form() {
        for form in ["/?", "--help", "-h"] {
            assert!(parse_args(&[form]).help, "{} not recognised", form);
        }
    }

    #[test]
    fn m_option_takes_next_argument() {
        let cli = parse_args(&["/m", "Quick", "a.txt", "b.txt"]);
        assert_eq!(cli.folder_compare_method, Some(CompareMethod::Quick));
        assert_eq!(cli.paths, vec!["a.txt", "b.txt"]);
    }

    #[test]
    fn m_option_accepts_colon_value_case_insensitively() {
        let cli = parse_args(&["/m:size"]);
        assert_eq!(cli.folder_compare_method, Some(CompareMethod::Size));
    }

    #[test]
    fn m_option_with_unknown_value_stays_none_and_is_not_a_path() {
        let cli = parse_args(&["/m", "Bogus", "a.txt"]);
        assert_eq!(cli.folder_compare_method, None);
        assert_eq!(cli.paths, vec!["a.txt"]);
    }
}
