mod cli;
mod client;
mod commands;
mod config;
mod error;
mod output;
mod types;

use clap::Parser;
use cli::{Cli, Command};
use config::Config;
use error::CliError;
use std::process;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if let Err(e) = run(cli).await {
        eprintln!("error: {e}");
        process::exit(1);
    }
}

async fn run(cli: Cli) -> Result<(), CliError> {
    let config_path = cli.config.unwrap_or_else(Config::path);
    let mut config = Config::load(&config_path)?;

    match cli.command {
        Command::Config { action } => commands::config::run(action, &mut config, &config_path),
        command => {
            let client = client::Client::new(&config, cli.timeout)?;
            run_api_command(command, &client, cli.json).await
        }
    }
}

async fn run_api_command(
    command: Command,
    client: &client::Client,
    json: bool,
) -> Result<(), CliError> {
    match command {
        Command::Ping => {
            let resp = client.get("/ping").await?;
            let data: types::PongResponse = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("{}", data.message);
            }
        }
        Command::Whoami => {
            let resp = client.get("/auth/whoami").await?;
            let data: types::WhoamiUser = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                println!("{} {} <{}>", data.first_name, data.last_name, data.email);
                println!("ID:       {}", data.id);
                if let Some(org) = &data.organization_id {
                    println!("Org:      {org}");
                }
                println!(
                    "Budget:   ${:.2} used of ${:.2}",
                    data.current_budget_usage as f64 / 100.0,
                    data.budget_amount as f64 / 100.0
                );
            }
        }
        Command::Archives { action } => {
            commands::archives::run(action, client, json).await?;
        }
        Command::Orders { action } => {
            commands::orders::run(action, client, json).await?;
        }
        Command::Notifications { action } => {
            commands::notifications::run(action, client, json).await?;
        }
        Command::Feasibility { action } => {
            commands::feasibility::run(action, client, json).await?;
        }
        Command::Pricing { aoi } => {
            let req = types::PricingRequest { aoi };
            let resp = client.post("/pricing", &req).await?;
            let data: serde_json::Value = resp.json().await?;
            if json {
                output::print_json(&data)?;
            } else {
                output::print_value(&data, 0);
            }
        }
        Command::Config { .. } => {
            return Err(CliError::General(
                "config commands must be handled before creating the API client".into(),
            ));
        }
    }

    Ok(())
}
