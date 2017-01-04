// Setup error-chain
// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]
#[macro_use] extern crate error_chain;
extern crate git2;
extern crate chrono;
extern crate clap;
extern crate regex;
use clap::{App, Arg};

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}
use errors::*;

mod axe;
use axe::Axe;

fn run(app_config: App) -> Result<()> {
    let axe = Axe::new(app_config)
        .chain_err(|| "Couldn't build Ax structure")?;
    
    axe.make_csv()
        .chain_err(|| "CSV creation failed")?;
    
    Ok(())
}

fn main() {
    let app_config = App::new("git-of-thesus")
        .version("0.1")
        .author("Benjamin Beckwith")
        .arg(Arg::with_name("cohortfmt")
            .long("cohortfmt")
            .short("f")
            .value_name("FMT")
            .help("A datetime format string such at \"%Y\" for creating cohorts")
            .default_value("%Y")
            .takes_value(true))
        .arg(Arg::with_name("interval")
            .long("interval")
            .short("i")
            .value_name("INT")
            .default_value("604800")
            .help("Min difference between commits to analyze (in seconds)")
            .takes_value(true))
        .arg(Arg::with_name("ignore")
            .long("ignore")
            .short("I")
            .multiple(true)
            .takes_value(true)
            .help("File patterns that should be ignored (can provide multiple)"))
        .arg(Arg::with_name("only")
            .long("only")
            .short("O")
            .multiple(true)
            .takes_value(true)
            .help("File patterns that have to match (can provide multiple)"))
        .arg(Arg::with_name("outdir")
            .long("outdir")
            .short("o")
            .takes_value(true)
            .required(true)
            .default_value(".")
            .help("Output directory to store results"))
        .arg(Arg::with_name("branch")
            .long("branch")
            .short("b")
            .takes_value(true)
            .default_value("master")
            .help("Branch to track"))
        .arg(Arg::with_name("REPO")
            .index(1)
            .required(true));

    if let Err(ref e) = run(app_config) {
        println!("Error: {}", e);

        for e in e.iter().skip(1) {
            println!("Caused by: {}", e);
        }

        if let Some(backtrace) = e.backtrace() {
            println!("Backtrace: {:?}", backtrace);
        }

        ::std::process::exit(1);
    }
}
