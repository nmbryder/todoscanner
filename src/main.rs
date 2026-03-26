mod config;
mod matcher;
mod output;
mod scanner;

use clap::Parser;

fn main() -> anyhow::Result<()> {
    let args = config::Args::parse();
    let cfg = config::Config::from_args(args)?;

    if !cfg.color {
        colored::control::set_override(false);
    }

    let mut printer = output::Printer::new(&cfg);
    scanner::scan(&cfg, |m| printer.print(m))?;
    printer.finish();

    Ok(())
}
