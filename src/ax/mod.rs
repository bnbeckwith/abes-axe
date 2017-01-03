use git2::{SORT_TIME, Commit, Repository, Oid, Tree, DiffDelta, DiffHunk, DiffOptions};

use clap::App;
use errors::*;
use chrono::{Duration, NaiveDateTime};
mod sample;
use self::sample::Sample;

#[derive(Clone)]
struct AxOptions {
    interval: i64,
    repo_path: String,
    cohort_fmt: String,
}

impl AxOptions {
    pub fn new(app_config: App) -> AxOptions {
        let matches = app_config.get_matches();
        let interval = matches.value_of("interval")
            .unwrap()
            .parse::<i64>()
            .ok()
            .unwrap();
        let repo_path = matches.value_of("REPO").unwrap();
        let cohort_fmt = matches.value_of("cohortfmt").unwrap();

        AxOptions {
            interval: interval,
            repo_path: repo_path.to_owned(),
            cohort_fmt: cohort_fmt.to_owned()
        }
    }
}

pub struct Ax {
    repo: Repository,
    options: AxOptions,
    samples: Vec<Sample>
}

impl Ax {
    pub fn new(app_config: App) -> Result<Ax> {
        let options = AxOptions::new(app_config);
        let repo = Repository::open(options.clone().repo_path)
            .chain_err(|| "Couldn't open repository")?;
        Ok(Ax {
            repo: repo, 
            options: options,
            samples: Vec::new()
        })
    }

    fn get_revwalk_ids(&self) -> Result<Vec<Oid>> {
        let mut revwalk = self.repo.revwalk()
            .chain_err(|| "Unable to revwalk")?;
        revwalk.set_sorting(SORT_TIME);
        revwalk.push_head()
            .chain_err(|| "Unable to push HEAD");
        
        Ok(revwalk.map(|i| i.unwrap()).collect())
    }

    fn commit_date_time(&self, commit: &Commit) -> Result<NaiveDateTime> {
        Ok(NaiveDateTime::from_num_seconds_from_unix_epoch(commit.time().seconds(), 0))
    }

    fn find_commit(&self, oid: &Oid) -> Result<Commit> {
        self.repo.find_commit(*oid)
            .chain_err(|| format!("Couldn't find commit for id: {}", oid))
    } 

    fn cohort_name(&self, dt: &NaiveDateTime) -> String {
        dt.format(&self.options.cohort_fmt).to_string()
    }

    fn process_commit(&mut self, id: Oid) -> Result<&Ax> {
        
        Ok(self)
    }
    
    pub fn build_cohorts(&mut self) -> Result<&Ax> {
        let mut ids: Vec<Oid> = self.get_revwalk_ids()
            .chain_err(|| "Unable to obtain revwalk ids")?;
        
        let duration = Duration::seconds(self.options.interval);
        let mut dt = self.commit_date_time(&self.find_commit(&ids[0])?)? + duration;
        let mut current_tree: Option<&Tree> = None;
        let mut samples = self.samples.clone();
        samples.clear();

        for id in ids {
            let commit = self.find_commit(&id)
                .chain_err(|| format!("Couldn't find commit: {}", id))? ;
            let commit_dt = self.commit_date_time(&commit)?;
            if commit_dt >= (dt + duration) {
                dt = commit_dt
            }else {
                continue
            }
            
            let mut sample = if self.samples.is_empty() {
                Sample::new(&commit_dt)
            }else{
                let idx = samples.len()-1;
                samples[idx].clone_and_date(&commit_dt)
            };
            
            let last_tree = current_tree;
            current_tree = Some(&commit.tree().chain_err(|| "Couldn't retreive tree")?);
            
            let mut diff_opts = DiffOptions::new();
            diff_opts.include_unmodified(false)
                .ignore_filemode(true)
                .context_lines(0);

            let diff = self.repo.diff_tree_to_tree(last_tree, current_tree, Some(&mut diff_opts))
                .chain_err(|| "Couldn't diff trees!")?;
                        
            let mut file_cb = |_d: DiffDelta, _n: f32| true;

            let mut hunk_cb = |d: DiffDelta, hunk: DiffHunk| {
                let s = &mut sample;
                s.add_diff_hunk(d,hunk);
                true
            };

            diff.foreach(&mut file_cb, None, Some(&mut hunk_cb), None)
                .chain_err(|| "Couldn't do diff")?;

            samples.push(sample);
        }
        
        Ok(self)
    }
}

