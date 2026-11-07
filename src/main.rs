use std::path::PathBuf;
use crate::app::App;
use clap::Parser;

pub mod snapshots;
pub mod scanner;
pub mod app;

#[derive(Parser, Debug)]
#[command(version, about)]
pub struct Args {
    #[arg(short, long, default_value=".")]
    path: PathBuf,
}

fn main() -> Result<(), std::io::Error> {
    println!("Hello, world!");

    let args = Args::parse();
    let mut app = App::new(&args);
    let mut term = ratatui::init();

    term.clear()?;
    app.run(&mut term)?;

    Ok(())
}