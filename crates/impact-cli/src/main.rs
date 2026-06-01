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

    let mut registry = impact_framework::AdapterRegistry::new();
    registry.register(Box::new(impact_vue::VueAdapter::new()));

    match cli.command {
        Command::Analyze(args) => commands::analyze::run(&args, &registry),
    }
}
