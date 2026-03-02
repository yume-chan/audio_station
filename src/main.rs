#![feature(portable_simd)]

use clap::{Parser, Subcommand};
use std::io;

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
    Client,
    Server,
}

fn main() -> io::Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Client => client::run(),
        Commands::Server => server::run(),
    }
}
