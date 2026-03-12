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

    // Config commands don't need an API client
    if let Command::Config { action } = cli.command {
        return commands::config::run(action, &mut config, &config_path);
    }

    // All other commands need a client
    let client = client::Client::new(&config, cli.timeout)?;

    match cli.command {
        Command::Config { .. } => unreachable!(),
        Command::Ping => {
            let resp = client.get("/ping").await?;
            let data: types::PongResponse = resp.json().await?;
            if cli.json {
                output::print_json(&data);
            } else {
                println!("{}", data.message);
            }
        }
        Command::Whoami => {
            let resp = client.get("/auth/whoami").await?;
            let data: types::WhoamiUser = resp.json().await?;
            if cli.json {
                output::print_json(&data);
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
            commands::archives::run(action, &client, cli.json).await?;
        }
        Command::Orders { action } => {
            commands::orders::run(action, &client, cli.json).await?;
        }
        Command::Notifications { action } => {
            commands::notifications::run(action, &client, cli.json).await?;
        }
        Command::Feasibility { action } => {
            commands::feasibility::run(action, &client, cli.json).await?;
        }
        Command::Pricing { aoi } => {
            let req = types::PricingRequest { aoi };
            let resp = client.post("/pricing", &req).await?;
            let data: serde_json::Value = resp.json().await?;
            if cli.json {
                println!("{}", serde_json::to_string_pretty(&data).unwrap());
            } else {
                output::print_value(&data, 0);
            }
        }
    }

    Ok(())
}
