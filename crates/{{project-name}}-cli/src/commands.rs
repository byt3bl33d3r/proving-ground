//! Command implementations. Each returns an [`Outcome`] for `output` to render.

use {{crate_name}}_core::types::{Item, ItemId, ItemPage};
use {{crate_name}}_runtime::config::env_value;
use serde::Serialize;
use serde_json::Value;

use crate::args::{Cli, Command, ItemsCommand};
use crate::client::Client;
use crate::error::CliError;
use crate::harness;

/// What a successful command produced.
#[derive(Debug, Serialize)]
#[serde(untagged)]
pub enum Outcome {
    /// One item.
    Item(Item),
    /// A page of items.
    Page(ItemPage),
    /// A deleted item's id.
    Deleted {
        /// The deleted id.
        deleted: ItemId,
    },
    /// The `doctor` report.
    Doctor(DoctorReport),
}

/// Where the server URL came from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum UrlSource {
    /// `--server-url`.
    Flag,
    /// `.harness/app.json`.
    Harness,
    /// `$APP_URL`.
    Env,
}

/// Output of `doctor`.
#[derive(Debug, Serialize)]
pub struct DoctorReport {
    /// Resolved server URL, if any.
    pub server_url: Option<String>,
    /// Where it came from.
    pub url_source: Option<UrlSource>,
    /// Whether the server answered.
    pub reachable: bool,
    /// Whether `/readyz` returned 200.
    pub ready: bool,
    /// Harness directory in use.
    pub harness_dir: Option<String>,
    /// Contents of `.harness/app.json`.
    pub app: Option<harness::AppInfo>,
    /// Contents of `.harness/stack.json`.
    pub stack: Option<Value>,
}

/// Resolves the server URL: `--server-url`, then `.harness/app.json`, then `$APP_URL`.
pub fn server_url(flag: Option<&str>) -> Option<(String, UrlSource)> {
    if let Some(url) = flag {
        return Some((url.to_owned(), UrlSource::Flag));
    }
    if let Some(app) = harness::dir().as_deref().and_then(harness::app) {
        return Some((app.url, UrlSource::Harness));
    }
    env_value("APP_URL")
        .ok()
        .flatten()
        .map(|url| (url, UrlSource::Env))
}

/// Runs one command.
pub async fn run(cli: &Cli) -> Result<Outcome, CliError> {
    match &cli.command {
        Command::Doctor => Ok(Outcome::Doctor(doctor(cli.server_url.as_deref()).await)),
        Command::Items(command) => items(cli.server_url.as_deref(), command).await,
    }
}

fn connect(flag: Option<&str>) -> Result<Client, CliError> {
    let (url, _) = server_url(flag).ok_or_else(|| CliError::Unreachable {
        message: "no server found: .harness/app.json is missing and APP_URL is unset. Run: just up"
            .to_owned(),
    })?;
    Ok(Client::new(&url)?)
}

fn parse_id(raw: &str) -> Result<ItemId, CliError> {
    ItemId::parse(raw)
        .map_err(|e| CliError::usage("validation_error", format!("invalid input: {e}")))
}

/// Validates arguments first (usage errors never need a server), then calls the API.
async fn items(flag: Option<&str>, command: &ItemsCommand) -> Result<Outcome, CliError> {
    Ok(match command {
        ItemsCommand::Create { name } => Outcome::Item(connect(flag)?.create_item(name).await?),
        ItemsCommand::Get { id } => {
            let id = parse_id(id)?;
            Outcome::Item(connect(flag)?.get_item(id).await?)
        }
        ItemsCommand::List { offset, limit } => {
            Outcome::Page(connect(flag)?.list_items(*offset, *limit).await?)
        }
        ItemsCommand::Delete { id } => {
            let deleted = parse_id(id)?;
            connect(flag)?.delete_item(deleted).await?;
            Outcome::Deleted { deleted }
        }
    })
}

async fn doctor(flag: Option<&str>) -> DoctorReport {
    let dir = harness::dir();
    let resolved = server_url(flag);
    let ready = match &resolved {
        Some((url, _)) => match Client::new(url) {
            Ok(client) => client.ready().await.ok(),
            Err(_unusable) => None,
        },
        None => None,
    };
    DoctorReport {
        server_url: resolved.as_ref().map(|(url, _)| url.clone()),
        url_source: resolved.map(|(_, source)| source),
        reachable: ready.is_some(),
        ready: ready.unwrap_or(false),
        harness_dir: dir.as_ref().map(|path| path.display().to_string()),
        app: dir.as_deref().and_then(harness::app),
        stack: dir.as_deref().and_then(harness::stack),
    }
}

impl DoctorReport {
    /// `Err(Unreachable)` when no server answered, so `doctor` exits 4.
    pub fn verdict(&self) -> Result<(), CliError> {
        if self.reachable {
            Ok(())
        } else {
            Err(CliError::Unreachable {
                message: "server not reachable. Run: just up (or pass --server-url)".to_owned(),
            })
        }
    }
}
