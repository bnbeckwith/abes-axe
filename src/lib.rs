// Setup error-chain
// `error_chain!` can recurse deeply
#![recursion_limit = "1024"]

extern crate chrono;
extern crate clap;
#[macro_use]
extern crate error_chain;
extern crate git2;
extern crate itertools;
extern crate regex;
extern crate futures;
extern crate parallel_event_emitter;

pub mod options;
use options::Options;

mod sample;
use sample::{Sample, Changeset};

// We'll put our errors in an `errors` module, and other modules in
// this crate will `use errors::*;` to get access to everything
// `error_chain!` creates.
mod errors {
    // Create the Error, ErrorKind, ResultExt, and Result types
    error_chain!{}
}
use errors::*;
use git2::{SORT_TIME, Commit, Repository, Oid, DiffDelta, DiffHunk, DiffOptions, Tree};
use std::collections::{HashSet};
use std::fs::File;
use std::io::Write;
use std::thread;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use chrono::{Duration, NaiveDateTime};

pub struct Axe {
    options: Options,
}

impl Axe {
    pub fn new(options: Option<Options>) -> Result<Axe> {
        match options {
            Some(opts) => Ok(Axe{options: opts}),
            None       => Ok(Axe{options: Options::default()})
        }
    }

    fn open_repo(&self) -> Result<Repository> {
        Repository::open(self.options.repo_path.clone())
            .chain_err(|| "Couldn't open repository")
    }

    fn get_revwalk_ids(&self) -> Result<Vec<Oid>> {
        let repo = self.open_repo()?;
        let mut revwalk = repo.revwalk()
            .chain_err(|| "Unable to revwalk")?;
        revwalk.set_sorting(SORT_TIME);
        revwalk.push_head()
            .chain_err(|| "Unable to push HEAD")?;

        let mut ids: Vec<Oid> = revwalk.map(|i| i.unwrap()).collect();
        ids.reverse();
        Ok(ids)
    }

    fn commit_date_time(&self, commit: &Commit) -> Result<NaiveDateTime> {
        Ok(NaiveDateTime::from_num_seconds_from_unix_epoch(commit.time().seconds(), 0))
    }

    pub fn cohort_name(&self, dt: &NaiveDateTime) -> String {
        dt.format(&self.options.cohort_fmt).to_string()
    }

    pub fn make_csv(&self) -> Result<()> {
        let samples = self.collect_samples()
            .chain_err(|| "Couldn't collect samples")?;

        let mut cohorts : Vec<String> = samples
            .iter()
            .map(|s| self.cohort_name(&s.date_time))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect();

        cohorts.sort();

        let mut f = File::create("axoutput.csv")
            .chain_err(|| "Couldn't create output file, axoutput.csv")?;

        write!(&mut f, "DateTime{}\n", cohorts.iter().map(|k| format!(",{}", k)).collect::<String>())
            .chain_err(|| "Couldn't write header")?;

        for sample in samples {
            let data = cohorts.iter()
                .map(|c| format!(",{}", sample.count_cohort_lines(&c)))
                .collect::<String>();
            write!(&mut f,"{}{}\n", sample.date_time, data)
                .chain_err(|| format!("Couldn't write line: {}", data))?;
        }

        Ok(())
    }

    fn skip_file(&self, filename: &str) -> bool {
        let ignore = match self.options.ignore {
            None => false,
            Some(ref re) => re.is_match(filename)
        };
        let keep = match self.options.only {
            None => true,
            Some(ref re) => re.is_match(filename)
        };
        ignore || !keep
    }

    pub fn collect_samples(&self) -> Result<Vec<Sample>> {
        let ids = self.get_revwalk_ids()
            .chain_err(|| "Unable to obtain revwalk ids")?;

        let repo = self.open_repo()?;
        let duration = Duration::seconds(self.options.interval);
        let first_commit = repo.find_commit(ids[0])
            .chain_err(|| format!("Couldn't find commit for id: {}", ids[0]))?;
        let dt = self.commit_date_time(&first_commit)?;

        let mut diff_opts = DiffOptions::new();
        diff_opts.include_unmodified(false)
            .ignore_filemode(true)
            .context_lines(0);

        let commits: Vec<Commit> = ids.iter().fold(vec![first_commit], |mut acc, &id| {
            let commit = repo.find_commit(id).unwrap();
            let commit_dt = self.commit_date_time(&commit).unwrap();
            let last_dt = self.commit_date_time(&acc.last().unwrap()).unwrap();

            if commit_dt < (last_dt + duration) {
                return acc
            }

            acc.push(commit);
            acc
        });

        struct TreeData<'a> {
            tree: Option<Tree<'a>>,
            date_time: NaiveDateTime
        }

        let mut trees: Vec<TreeData> = vec![TreeData{ tree: None, date_time: dt}];
        trees.extend(commits.iter().map(|ref c|
            TreeData{
                tree: Some(c.tree().unwrap()),
                date_time: self.commit_date_time(&c).unwrap()
            }
        ));

        let mut file_cb = |_d: DiffDelta, _n: f32| true;

        // struct Options {
        //     ...,
        //     eventBus: EventBus,
        //     collectingChangesetCallbacks: Vec<Box<Fn<_>>>,
        //     ...
        // }
        // updateTerm = |progress| println!("Progress: {}", progress);
        // options.collectingChangesetCallbacks.push(updateTerm);
        // options.nextStepCallbacks.push(updateTerm);

        // options.progressCallbacks.push(|progress| updateGUI!("Progress: {}", progress));
        // options.eventBus.subscribe(Status.ChangesetCollection, updateTerm);

        // foreach(callback in options.collectingChangesetCallbacks) {
        //     callback(trees.len());
        // }
        // options.eventBus.publish(Status.ChangesetCollection, 47);
        // let mut mb = MultiBar::new();
        // let mut pb = mb.create_bar(trees.len() as u64);
        // let mut pb2 = mb.create_bar(trees.len() as u64);

        // thread::spawn(move || mb.listen());

        // pb.message("Collecting changesets: ");
        // pb.format("╢▌▌░╟");
        // pb2.message("Processing changesets: ");
        // pb2.format("╢▌▌░╟");

        let (tx, rx) : (Sender<Changeset>, Receiver<Changeset>) = mpsc::channel();
        let (samples_tx, samples_rx) : (Sender<Vec<Sample>>, Receiver<Vec<Sample>>)
            = mpsc::channel();

        let cohort_fmt = self.options.cohort_fmt.clone();
        thread::spawn(move || {
            let mut acc: Vec<Sample> = Vec::new();
            for changeset in rx.iter() {
                let mut sample: Sample = if acc.is_empty() {
                    Sample::new(&changeset.date_time)
                } else {
                    acc.last().unwrap()
                        .clone_and_date(&changeset.date_time)
                };

                let cohort = changeset.date_time.format(&cohort_fmt).to_string();

                acc.push(sample.add_changeset(&changeset, &cohort).to_owned())
            }

            samples_tx.send(acc).unwrap();
        });

        let repo = self.open_repo()?;
        for pair in trees.windows(2) {

            let mut changeset = Changeset::new(pair[1].date_time);

            {
                let mut hunk_cb = |d: DiffDelta, hunk: DiffHunk| {
                    let filename = match (d.new_file().path(), d.old_file().path()) {
                        (_, Some(p)) => p.to_str().unwrap(),
                        (Some(p), _) => p.to_str().unwrap(),
                        _ => "" // TODO Consider error here
                    };
                    if self.skip_file(filename) {
                        return true
                    }
                    changeset.add_diff_hunk(d,hunk)
                };

                let diff = if pair[0].tree.is_none() {
                    let rh_tree = pair[1].tree.as_ref().unwrap();

                    repo.diff_tree_to_tree(None,
                                                Some(&rh_tree),
                                                Some(&mut diff_opts))
                }else {
                    let lh_tree = pair[0].tree.as_ref().unwrap();
                    let rh_tree = pair[1].tree.as_ref().unwrap();

                    repo.diff_tree_to_tree(Some(&lh_tree),
                                                Some(&rh_tree),
                                                Some(&mut diff_opts))
                };

                diff.unwrap().foreach(&mut file_cb, None, Some(&mut hunk_cb), None)
                    .unwrap();
            }
            tx.send(changeset).unwrap();
        };

        drop(tx);

        let samples = samples_rx.recv().unwrap();

        Ok(samples)
    }
}
