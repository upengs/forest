// Copyright 2019-2023 ChainSafe Systems
// SPDX-License-Identifier: Apache-2.0, MIT

use tracing_chrome::{ChromeLayerBuilder, FlushGuard, TraceStyle};
use tracing_subscriber::{
    filter::{self, EnvFilter, LevelFilter},
    prelude::*,
};

use crate::cli_shared::cli::{CliOpts, LogConfig};

pub fn setup_logger(
    log_config: &LogConfig,
    opts: &CliOpts,
) -> (Option<tracing_loki::BackgroundTask>, Option<FlushGuard>) {
    let mut loki_task = None;
    let tracing_tokio_console = if opts.tokio_console {
        Some(
            console_subscriber::ConsoleLayer::builder()
                .with_default_env()
                .spawn(),
        )
    } else {
        None
    };
    let tracing_loki = if opts.loki {
        let (layer, task) = tracing_loki::layer(
            tracing_loki::url::Url::parse(&opts.loki_endpoint)
                .map_err(|e| format!("Unable to parse loki endpoint {}: {e}", &opts.loki_endpoint))
                .unwrap(),
            vec![(
                "host".into(),
                gethostname::gethostname()
                    .to_str()
                    .unwrap_or_default()
                    .into(),
            )]
            .into_iter()
            .collect(),
            Default::default(),
        )
        .map_err(|e| format!("Unable to create loki layer: {e}"))
        .unwrap();
        loki_task = Some(task);
        Some(layer.with_filter(LevelFilter::DEBUG))
    } else {
        None
    };
    let tracing_rolling_file = if let Some(log_dir) = &opts.log_dir {
        let file_appender = tracing_appender::rolling::hourly(log_dir, "forest.log");
        Some(
            tracing_subscriber::fmt::Layer::new()
                .with_ansi(false)
                .with_writer(file_appender)
                .with_filter(build_env_filter(log_config)),
        )
    } else {
        None
    };

    // Go to <https://ui.perfetto.dev> to browse trace files.
    // You may want to call ChromeLayerBuilder::trace_style as appropriate
    let (chrome_layer, flush_guard) =
        match std::env::var_os("CHROME_TRACE_FILE").map(|path| match path.is_empty() {
            true => ChromeLayerBuilder::new().build(),
            false => ChromeLayerBuilder::new().file(path).build(),
        }) {
            Some((a, b)) => (Some(a), Some(b)),
            None => (None, None),
        };

    tracing_subscriber::registry()
        .with(tracing_tokio_console)
        .with(tracing_loki)
        .with(tracing_rolling_file)
        .with(chrome_layer)
        .with(
            tracing_subscriber::fmt::Layer::new()
                .with_ansi(opts.color.coloring_enabled())
                .with_filter(build_env_filter(log_config)),
        )
        .init();
    (loki_task, flush_guard)
}

fn build_env_filter(log_config: &LogConfig) -> EnvFilter {
    EnvFilter::builder().parse_lossy(
        [
            "info".into(),
            log_config.to_filter_string(),
            std::env::var(EnvFilter::DEFAULT_ENV).unwrap_or_default(),
        ]
        .join(","),
    )
}