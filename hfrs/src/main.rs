mod cli;
mod commands;
mod output;
mod util;

use anyhow::Result;
use clap::Parser;
use cli::{Cli, Command};
use huggingface_hub::HfApiBuilder;
use output::render;
use util::token::read_active_token;

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let token = cli
        .token
        .clone()
        .or_else(|| std::env::var("HF_TOKEN").ok())
        .or_else(read_active_token);

    let mut builder = HfApiBuilder::new();
    if let Some(t) = token {
        builder = builder.token(t);
    }
    if let Some(endpoint) = cli.endpoint.clone() {
        builder = builder.endpoint(endpoint);
    }
    let api = builder.build()?;

    let result = match cli.command {
        Command::Auth(args) => commands::auth::execute(&api, args).await?,
        Command::Cache(args) => commands::cache::execute(&api, args).await?,
        Command::Collections(args) => commands::collections::execute(&api, args).await?,
        Command::Datasets(args) => commands::datasets::execute(&api, args).await?,
        Command::Discussions(args) => commands::discussions::execute(&api, args).await?,
        Command::Download(args) => commands::download::execute(&api, args).await?,
        Command::Endpoints(args) => commands::endpoints::execute(&api, args).await?,
        Command::Jobs(args) => commands::jobs::execute(&api, args).await?,
        Command::Likes(args) => commands::likes::execute(&api, args).await?,
        Command::Models(args) => commands::models::execute(&api, args).await?,
        Command::Papers(args) => commands::papers::execute(&api, args).await?,
        Command::Repos(args) => commands::repos::execute(&api, args).await?,
        Command::Spaces(args) => commands::spaces::execute(&api, args).await?,
        Command::Upload(args) => commands::upload::execute(&api, args).await?,
        Command::Webhooks(args) => commands::webhooks::execute(&api, args).await?,
        Command::AccessRequests(args) => commands::access_requests::execute(&api, args).await?,
        Command::Env(args) => commands::env::execute(args).await?,
        Command::Version(args) => commands::version::execute(args).await?,
    };

    render(result)?;
    Ok(())
}
