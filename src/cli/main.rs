// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use std::ffi::OsString;
use std::sync::Arc;

use crate::cli_shared::logger;
use crate::daemon::get_actual_chain_name;
use crate::networks::ChainConfig;
use crate::shim::address::{CurrentNetwork, Network};
use crate::utils::{bail_moved_cmd, io::ProgressBar};
use crate::{
    cli::subcommands::{cli_error_and_die, Cli},
    rpc_client::state_network_name,
};
use clap::Parser;

use super::subcommands::Subcommand;

pub fn main<ArgT>(args: impl IntoIterator<Item = ArgT>) -> anyhow::Result<()>
where
    ArgT: Into<OsString> + Clone,
{
    // Capture Cli inputs
    let Cli { opts, cmd } = Cli::parse_from(args);

    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .unwrap()
        .block_on(async {
            match opts.to_config() {
                Ok((mut config, _)) => {
                    logger::setup_logger(&opts);
                    ProgressBar::set_progress_bars_visibility(config.client.show_progress_bars);
                    if opts.dry_run {
                        return Ok(());
                    }
                    let opts = &opts;
                    if opts.chain.is_none() {
                        if let Ok(name) = state_network_name((), &config.client.rpc_token).await {
                            let name = get_actual_chain_name(&name);
                            if name == "calibnet" {
                                config.chain = Arc::new(ChainConfig::calibnet());
                            } else if name == "devnet" {
                                config.chain = Arc::new(ChainConfig::devnet());
                            }
                        }
                    }
                    if config.chain.is_testnet() {
                        CurrentNetwork::set_global(Network::Testnet);
                    }
                    // Run command
                    match cmd {
                        Subcommand::Fetch(_cmd) => {
                            bail_moved_cmd("fetch-params", "forest-tool fetch-params")
                        }
                        Subcommand::Chain(cmd) => cmd.run(config).await,
                        Subcommand::Auth(cmd) => cmd.run(config).await,
                        Subcommand::Net(cmd) => cmd.run(config).await,
                        Subcommand::Wallet(..) => bail_moved_cmd("wallet", "forest-wallet"),
                        Subcommand::Sync(cmd) => cmd.run(config).await,
                        Subcommand::Mpool(cmd) => cmd.run(config).await,
                        Subcommand::State(cmd) => cmd.run(config).await,
                        Subcommand::Config(cmd) => cmd.run(&config, &mut std::io::stdout()),
                        Subcommand::Send(cmd) => cmd.run(config).await,
                        Subcommand::Info(cmd) => cmd.run(config, opts).await,
                        Subcommand::DB(cmd) => cmd.run(&config).await,
                        Subcommand::Snapshot(cmd) => cmd.run(config).await,
                        Subcommand::Archive(cmd) => cmd.run().await,
                        Subcommand::Attach(cmd) => cmd.run(config),
                        Subcommand::Shutdown(cmd) => cmd.run(config).await,
                        Subcommand::Car(..) => bail_moved_cmd("car", "forest-tool"),
                    }
                }
                Err(e) => {
                    cli_error_and_die(format!("Error parsing config: {e}"), 1);
                }
            }
        })
}
