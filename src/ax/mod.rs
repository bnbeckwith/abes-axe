use git2::{SORT_TIME, Commit, Repository, Oid, Delta, DiffDelta, DiffFile, DiffHunk, DiffOptions};
use std::collections::{HashMap, HashSet};
use clap::App;
use errors::*;
use chrono::{Duration, NaiveDateTime};
use std::fs::File;
use std::io::Write;

type Lines = Vec<String>;
type FileName = String;
type FileMap = HashMap<FileName, Lines>;

#[derive(Clone)]
pub struct Sample {
    pub date_time: NaiveDateTime,
    files: FileMap
}

impl Sample {
    pub fn new(dt: &NaiveDateTime) -> Sample {
        Sample {
            date_time: *dt,
            files: HashMap::new()
        }
    }

    pub fn clone_and_date(&self, dt: &NaiveDateTime) -> Sample {
        let mut sample = self.clone();
        sample.date_time = *dt;
        sample
    }

    pub fn add_diff_hunk(&mut self, delta: DiffDelta, hunk: DiffHunk, cohort: &String) -> &mut Sample {

        match delta.status() {
            Delta::Added => {
                self.process_added(delta, hunk, cohort)
            },
            Delta::Deleted => {
                self.process_deleted(delta, hunk)
            },
            Delta::Modified => {
                self.process_modified(delta, hunk, cohort)
            },
            Delta::Renamed => {
                self.process_renamed(delta, hunk)
            },
            Delta::Copied => {
                self.process_copied(delta, hunk)
            },
            _ => self 
        }
    }

    fn filename(f: &DiffFile) -> FileName {
        String::from(f.path().map(|e| e.to_str().unwrap()).unwrap())
    }

    fn total_lines(&self) -> i64 {
        self.files.values().map(|v| v.len()).fold(0, |acc,l| acc + l as i64)
    }
    
    fn get_lines(&self, filename: &FileName) -> Lines {
        match self.files.get(filename) {
            Some(v) => v.to_owned(),
            None    => Vec::new()
        }
    }

    fn count_cohort_lines(&self, cohort: &String) -> i64 {
        self.files.values()
            .map(|v| v.iter().filter(|v| *v == cohort).count() )
            .fold(0, |acc, v| acc + v) as i64
    }
    
    fn set_lines(&mut self, filename: &FileName, lines: Lines) -> &mut Sample {
        self.files.insert(filename.to_owned(), lines);
        self
    }

    fn delete_lines(&mut self, filename: &FileName) -> &mut Sample {
        self.files.remove(filename);
        self
    }
    
    fn process_added (&mut self, delta: DiffDelta, hunk: DiffHunk, cohort: &String) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        let mut lines = self.get_lines(&filename);
        let start = hunk.new_start() - 1;
        let end = start + hunk.new_lines(); 
        for _n in start..end {
            lines.insert(start as usize, cohort.to_owned());
        }
        self.set_lines(&filename,lines)
    }
    
    fn process_deleted (&mut self, delta: DiffDelta, _hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.old_file());
        self.delete_lines(&filename)
    }
    
    fn process_modified (&mut self, delta: DiffDelta, hunk: DiffHunk, cohort: &String) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        let mut lines = self.get_lines(&filename);
        let start = if hunk.new_start() > 0 {
            hunk.new_start() - 1
        }else { 0 };
        let old_end = start + hunk.old_lines();
        let new_end = start + hunk.new_lines();
        for _n in start..old_end {
            lines.remove(start as usize);
        }
        for _n in start..new_end {
            lines.insert(start as usize, cohort.to_owned());
        }
        self.set_lines(&filename, lines)
    }
    
    fn process_renamed (&mut self, delta: DiffDelta, _hunk: DiffHunk) -> &mut Sample {
        println!("Renamed: {} to {}",
                 Sample::filename(&delta.old_file()),
                 Sample::filename(&delta.new_file()));
        self
    }
    
    fn process_copied (&mut self, delta: DiffDelta, _hunk: DiffHunk) -> &mut Sample {
        println!("Copied: {} to {}",
                 Sample::filename(&delta.old_file()),
                 Sample::filename(&delta.new_file()));
        self
    }
}

#[derive(Clone)]
struct Options {
    interval: i64,
    repo_path: String,
    cohort_fmt: String,
}

impl Options {
    pub fn new(app_config: App) -> Options {
        let matches = app_config.get_matches();
        let interval = matches.value_of("interval")
            .unwrap()
            .parse::<i64>()
            .ok()
            .unwrap();
        let repo_path = matches.value_of("REPO").unwrap();
        let cohort_fmt = matches.value_of("cohortfmt").unwrap();

        Options {
            interval: interval,
            repo_path: repo_path.to_owned(),
            cohort_fmt: cohort_fmt.to_owned()
        }
    }
}

pub struct Ax {
    repo: Repository,
    options: Options,
}

impl Ax {
    pub fn new(app_config: App) -> Result<Ax> {
        let options = Options::new(app_config);
        let repo = Repository::open(options.clone().repo_path)
            .chain_err(|| "Couldn't open repository")?;
        Ok(Ax {
            repo: repo, 
            options: options
        })
    }

    fn get_revwalk_ids(&self) -> Result<Vec<Oid>> {
        let mut revwalk = self.repo.revwalk()
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

    fn find_commit(&self, oid: &Oid) -> Result<Commit> {
        self.repo.find_commit(*oid)
            .chain_err(|| format!("Couldn't find commit for id: {}", oid))
    }

    pub fn cohort_name(&self, dt: &NaiveDateTime) -> String {
        dt.format(&self.options.cohort_fmt).to_string()
    }

    pub fn make_csv(&self) -> Result<()> {
        let samples = self.collect_samples()
            .chain_err(|| "Couldn't collect samples")?;

        println!("Done collecting {} samples!", samples.len());
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
    
    pub fn collect_samples(&self) -> Result<Vec<Sample>> {
        let ids: Vec<Oid> = self.get_revwalk_ids()
            .chain_err(|| "Unable to obtain revwalk ids")?;

        let duration = Duration::seconds(self.options.interval);
        let dt = self.commit_date_time(&self.find_commit(&ids[0])?)? + duration;

        struct Accumulator {
            samples: Vec<Sample>,
            last_tree_id: Option<Oid>,
            date_time: NaiveDateTime
        };

        let start = Accumulator {
            samples: Vec::new(),
            last_tree_id: None,
            date_time: dt
        };

        let accumulator = ids.iter().fold(start, |acc, &id| {
            let commit = self.find_commit(&id).unwrap();
            let commit_dt = self.commit_date_time(&commit).unwrap();
            let cohort_name = self.cohort_name(&commit_dt); 

            if commit_dt < (acc.date_time + duration) {
                return acc
            }
            
            let mut sample = acc.samples.last()
                .map(|ref x| x.clone_and_date(&commit_dt))
                .unwrap_or(Sample::new(&commit_dt));

            let mut diff_opts = DiffOptions::new();
            diff_opts.include_unmodified(false)
                .ignore_filemode(true)
                .context_lines(0);
            
            let diff = match acc.last_tree_id {
                Some(id) => {
                    self.repo
                        .diff_tree_to_tree(Some(self.repo.find_tree(id).as_ref().unwrap()),
                                           Some(&commit.tree().unwrap()),
                                           Some(&mut diff_opts))
                        .unwrap()
                },
                None => {
                    self.repo
                        .diff_tree_to_tree(None,
                                           Some(&commit.tree().unwrap()),
                                           Some(&mut diff_opts))
                        .unwrap()                    
                }
            };

            {
                let mut file_cb = |_d: DiffDelta, _n: f32| true;

                let mut hunk_cb = |d: DiffDelta, hunk: DiffHunk| {
                    sample.add_diff_hunk(d,hunk, &cohort_name);
                    true
                };

                diff.foreach(&mut file_cb, None, Some(&mut hunk_cb), None).unwrap();
            }
            
            let mut samples = acc.samples.clone();
            samples.push(sample);
            
            Accumulator {
                samples: samples,
                last_tree_id: Some(commit.tree().unwrap().id()),
                date_time: commit_dt
            }
        });
        
        Ok(accumulator.samples)
    }
}
