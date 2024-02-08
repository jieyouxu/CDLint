use clap::Parser;
use tracing::*;

mod logging;

#[derive(Debug, Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// The path to a Custom Difficulty JSON file.
    input: String,
}

fn main() {
    logging::setup_logging();

    let cli = Args::parse();

    debug!(input = ?cli.input);
}
