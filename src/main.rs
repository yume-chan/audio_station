#![feature(portable_simd)]

use clap::{Parser, Subcommand};
use std::io;

use crate::shared::DeviceType;

mod client;
mod server;
mod shared;

#[derive(Parser)]
#[command(name = "audio_station")]
#[command(about = "Audio station with client and server modes")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Client {
        #[arg(long = "type", value_enum, default_value_t = DeviceType::Output)]
        r#type: DeviceType,
    },
    Server,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Client { r#type } => client::run(r#type),
        Commands::Server => server::run(),
    }
}
