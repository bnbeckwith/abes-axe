// Setup error-chain
// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]
#[macro_use]
extern crate error_chain;

extern crate clap;
use clap::{App, Arg};

extern crate git2;
use git2::{Commit, Repository, Oid, Delta};

extern crate chrono;
use chrono::*;

use std::collections::{HashMap, HashSet};

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}

use errors::*;

mod ax;
use ax::Ax;

fn get_commits(repo: &Repository, interval: i64) -> Result<Vec<Commit>> {
    let mut revwalk = repo.revwalk()
        .chain_err(|| "Unable to revwalk")?;
    revwalk.set_sorting(git2::SORT_TIME);
    revwalk.push_head()
        .chain_err(|| "Unable to push HEAD")?;

    let mut commits = Vec::new();

    for id in revwalk {
        let id = id.unwrap();
        let commit = repo.find_commit(id)
            .chain_err(|| format!("Couldn't find commit: {}", id))?;
        commits.push(commit);
    }

    Ok(commits)
}

fn commit_date_time(repo: &Repository, commit_id: Oid) -> Result<NaiveDateTime> {
    let commit = repo.find_commit(commit_id)
        .chain_err(|| format!("Couldn't find commit from id: {}", commit_id))?;
    Ok(NaiveDateTime::from_num_seconds_from_unix_epoch(commit.time().seconds(), 0))
}

fn get_revwalk_ids(repo: &Repository) -> Result<Vec<Oid>> {
    let mut revwalk = repo.revwalk()
        .chain_err(|| "Unable to revwalk")?;
    revwalk.set_sorting(git2::SORT_TIME);
    revwalk.push_head()
        .chain_err(|| "Unable to push HEAD")?;
    
    Ok(revwalk.map(|i| i.unwrap()).collect())
}


fn get_blob_ids(repo: &Repository, commits: &Vec<Commit>) -> Result<HashSet<Oid>> {
    let mut entries = HashSet::new();

    for commit in commits {
        for entry in commit.tree().chain_err(|| "Tree")?.iter() {
            println!("Tree: {}", entry.name().unwrap_or("None"));
            let object = entry.to_object(repo)
                .chain_err(|| "Couldn't make object")?;
            if let Some(blob) = object.as_blob() {
                // ADD PATTERNS HERE
                entries.insert(blob.id());
            }
        }
    }

    Ok(entries)
}

fn run(app_config: App) -> Result<()> {

    let mut ax = Ax::new(app_config)
        .chain_err(|| "Couldn't build Ax structure")?;
    
    ax.build_cohorts()
        .chain_err(|| "Couldn't build cohorts")?;
    
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
