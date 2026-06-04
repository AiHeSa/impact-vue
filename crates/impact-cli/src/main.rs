use std::collections::HashMap;
use clap::{Parser, Subcommand};

mod commands;

#[derive(Parser)]
#[command(name = "impact", about = "Vue static impact chain analyzer")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    Analyze(commands::analyze::AnalyzeArgs),
}

fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    match cli.command {
        Command::Analyze(args) => {
            // 解析 alias 参数
            let mut aliases = HashMap::new();
            for alias_str in &args.alias {
                if let Some((key, value)) = alias_str.split_once('=') {
                    aliases.insert(key.to_string(), value.to_string());
                }
            }

            let mut registry = impact_framework::AdapterRegistry::new();
            registry.register(Box::new(impact_vue::VueAdapter::with_aliases(aliases)));

            commands::analyze::run(&args, &registry)
        }
    }
}
