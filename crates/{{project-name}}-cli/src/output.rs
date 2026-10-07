//! User-facing output: the only module allowed to print. Results go to stdout, errors to
//! stderr; JSON when stdout is not a terminal (agents, pipes), text otherwise.

use std::io::IsTerminal;

use serde_json::json;

use crate::commands::{DoctorReport, Outcome};
use crate::error::CliError;

/// Output format.
#[derive(Clone, Copy, Debug, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// One JSON document.
    Json,
    /// Human-readable lines.
    Text,
}

impl Format {
    /// The flag's value, else `text` on a terminal and `json` otherwise.
    pub fn resolve(flag: Option<Self>) -> Self {
        flag.unwrap_or_else(|| {
            if std::io::stdout().is_terminal() {
                Self::Text
            } else {
                Self::Json
            }
        })
    }
}

/// Prints a command's result.
pub fn outcome(format: Format, outcome: &Outcome) {
    match format {
        Format::Json => sink::stdout(&json!(outcome).to_string()),
        Format::Text => sink::stdout(&text(outcome)),
    }
}

/// Prints an error.
pub fn error(format: Format, err: &CliError) {
    match format {
        Format::Json => sink::stderr(
            &json!({ "error": {
                "code": err.code(),
                "message": err.to_string(),
                "request_id": err.request_id(),
                "exit_code": err.exit_code(),
            }})
            .to_string(),
        ),
        Format::Text => {
            let request = err
                .request_id()
                .map(|id| format!(" (request_id {id})"))
                .unwrap_or_default();
            sink::stderr(&format!("error[{}]: {err}{request}", err.code()));
        }
    }
}

fn text(outcome: &Outcome) -> String {
    match outcome {
        Outcome::Item(item) => format!("{}  {}", item.id, item.name.as_str()),
        Outcome::Page(page) => {
            let mut lines: Vec<String> = page
                .items
                .iter()
                .map(|item| format!("{}  {}", item.id, item.name.as_str()))
                .collect();
            if let Some(next) = page.next_offset {
                lines.push(format!("(more: --offset {next})"));
            }
            if lines.is_empty() {
                "(no items)".to_owned()
            } else {
                lines.join("\n")
            }
        }
        Outcome::Deleted { deleted } => format!("deleted {deleted}"),
        Outcome::Doctor(report) => doctor_text(report),
    }
}

fn doctor_text(report: &DoctorReport) -> String {
    let url = report.server_url.as_deref().unwrap_or("(none)");
    let dir = report.harness_dir.as_deref().unwrap_or("(none)");
    format!(
        "server_url: {url}\nreachable: {}\nready: {}\nharness_dir: {dir}",
        report.reachable, report.ready
    )
}

#[cfg(test)]
mod tests {
    use {{crate_name}}_core::types::{Item, ItemId, ItemName, ItemPage, Timestamp};

    use super::*;

    fn item(bits: u64, name: &str) -> Item {
        Item {
            id: ItemId::from_bits(bits),
            name: ItemName::new(name).expect("valid"),
            created_at_ms: Timestamp::from_millis(0),
        }
    }

    #[test]
    fn text_output_is_one_line_per_item() {
        let page = Outcome::Page(ItemPage {
            items: vec![item(0, "a"), item(1, "b")],
            next_offset: Some(2),
        });
        assert_eq!(
            text(&page),
            "itm_0000000000000000  a\nitm_0000000000000001  b\n(more: --offset 2)",
            "page"
        );
        let empty = Outcome::Page(ItemPage {
            items: Vec::new(),
            next_offset: None,
        });
        assert_eq!(text(&empty), "(no items)", "empty page");
        assert_eq!(
            text(&Outcome::Item(item(5, "pen"))),
            "itm_0000000000000005  pen",
            "item"
        );
        let deleted = Outcome::Deleted {
            deleted: ItemId::from_bits(5),
        };
        assert_eq!(text(&deleted), "deleted itm_0000000000000005", "deleted");
    }

    #[test]
    fn doctor_text_lists_the_essentials() {
        let report = DoctorReport {
            server_url: None,
            url_source: None,
            reachable: false,
            ready: false,
            harness_dir: None,
            app: None,
            stack: None,
        };
        let rendered = doctor_text(&report);
        assert!(
            rendered.contains("server_url: (none)") && rendered.contains("reachable: false"),
            "{rendered}"
        );
        assert!(report.verdict().is_err(), "unreachable doctor fails");
    }
}

mod sink {
    #![expect(
        clippy::print_stdout,
        clippy::print_stderr,
        clippy::disallowed_macros,
        reason = "cli::output is the single place allowed to print"
    )]

    pub(super) fn stdout(text: &str) {
        println!("{text}");
    }

    pub(super) fn stderr(text: &str) {
        eprintln!("{text}");
    }
}
