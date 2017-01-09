use git2::{SORT_TIME, Commit, Repository, Oid, Delta, DiffDelta, DiffFile, DiffHunk, DiffOptions, Tree};
use std::collections::{HashMap, HashSet};
use clap::App;
use errors::*;
use chrono::{Duration, NaiveDateTime};
use std::fs::File;
use std::io::Write;
use regex::Regex;
use pbr::MultiBar;
use std::thread;

type Cohort = String;
type Lines = Vec<Cohort>;
type FileName = String;
type FileMap = HashMap<FileName, Lines>;
type CohortMap = HashMap<Cohort, u64>;
type FileCohorts = HashMap<FileName, CohortMap>;
    
enum Change {
    Add {filename: String, start: u32, length: u32},
    Delete {filename: String, start: u32, length: u32},
    DeleteFile {filename: String},
    AddFile {filename: String, length: u32}
}

pub struct Changeset {
    date_time: NaiveDateTime,
    changes: Vec<Change>  
}

impl Changeset { 
    pub fn new(dt: NaiveDateTime) -> Changeset {
        Changeset{
            date_time: dt,
            changes: Vec::new()
        }
    }

    pub fn process_added(&mut self, d: DiffDelta, h: DiffHunk) -> bool {
        let filename = Accumulator::filename(&d.new_file());
        self.changes.push(Change::AddFile{filename: filename, length: h.new_lines()});
        true
    }

    pub fn process_deleted(&mut self, d: DiffDelta, _h: DiffHunk) -> bool {
        let filename = Accumulator::filename(&d.old_file());
        self.changes.push(Change::DeleteFile{filename: filename});
        true
    }
    
    pub fn process_modified(&mut self, d: DiffDelta, h: DiffHunk) -> bool {
        let filename = Accumulator::filename(&d.new_file());
        let start = if h.new_start() > 0 {
            h.new_start() -1
        } else {0};
        let old_end = start + h.old_lines();
        let new_end = start + h.new_lines();
        if h.old_lines() > 0 {
            self.changes.push(Change::Delete{filename: filename.clone(),
                                             start: start,
                                             length: old_end-start});
        }
        if h.new_lines() > 0 {
            self.changes.push(Change::Add{filename: filename.clone(),
                                          start: start,
                                          length: new_end-start});
        }
        true
    }

    pub fn add_diff_hunk(&mut self, d: DiffDelta, h: DiffHunk) -> bool {
        match d.status() {
            Delta::Added =>    self.process_added(d,h),
            Delta::Deleted =>  self.process_deleted(d,h),
            Delta::Modified => self.process_modified(d,h),
            _ => {
                println!("Unsupported status!");
                false
            }
            
        }
    }
}

#[derive(Clone)]
struct Accumulator {
    files: FileMap
}

impl Accumulator {
    pub fn new() -> Accumulator {
        Accumulator { files: HashMap::new() }
    }

    pub fn add_changeset(&mut self, changeset: &Changeset, cohort: &String) -> &mut Accumulator {
        for change in changeset.changes.iter() {
            match change {
                &Change::Add { ref filename, start, length } =>
                {
                    // TODO: split, create, rejoin 
                    let mut lines = self.get_lines(&filename);
                    for _n in start..(start+length){
                        lines.insert(start as usize, cohort.to_owned())
                    }
                    self.set_lines(&filename, lines);
                },
                &Change::Delete { ref filename, start, length } =>
                {
                    // TODO: can I use DRAIN
                    let mut lines = self.get_lines(&filename);
                    for _n in start..(start+length){
                        lines.remove(start as usize);
                    }
                    self.set_lines(&filename, lines);
                },
                &Change::AddFile { ref filename, length } =>
                {
                    self.set_lines(&filename,vec![cohort.to_owned(); length as usize]);
                },
                &Change::DeleteFile { ref filename } =>
                {
                    self.delete_lines(&filename);
                }
            }
        };
        self
    }
    fn filename(f: &DiffFile) -> FileName {
        String::from(f.path().map(|e| e.to_str().unwrap()).unwrap())
    }

    fn get_lines(&self, filename: &FileName) -> Lines {
        match self.files.get(filename) {
            Some(v) => v.to_owned(),
            None    => Vec::new()
        }
    }

    fn set_lines(&mut self, filename: &FileName, lines: Lines) -> &mut Accumulator {
        self.files.insert(filename.to_owned(), lines);
        self
    }

    fn delete_lines(&mut self, filename: &FileName) -> &mut Accumulator {
        self.files.remove(filename);
        self
    }    
}

#[derive(Clone)]
pub struct Sample {
    pub date_time: NaiveDateTime,
    files: FileCohorts
}

impl Sample {

    pub fn from_changesets(changes: Vec<Changeset>, cohort_fmt: &String, iter_cb: &mut FnMut() -> bool) -> Vec<Sample> {
        let mut result: Vec<Sample> = Vec::new();
        changes.iter().fold(Accumulator::new(), |mut acc, ref set| {
            
            let cohort = set.date_time.format(cohort_fmt).to_string();

            iter_cb();
 
            result.push(
                Sample::from_acc(set.date_time, acc.add_changeset(&set, &cohort)));
            acc
        });
        result
    }

    fn from_acc(dt: NaiveDateTime, accumulator: &Accumulator) -> Sample {
        let mut fcs: FileCohorts = HashMap::new();
        // Key, value, summarize the lines on the values. Map over into a Hash of counts.
        for key in accumulator.files.keys() {
            let fh: HashMap<String, u64> =
                accumulator.files.get(key).unwrap()
                .iter()
                .fold(HashMap::new(), |mut acc, value| {
                    {
                        let count = acc.entry(value.to_owned()).or_insert(0);
                        *count += 1;
                    }
                    acc
                });
            
            fcs.insert(key.to_owned(), fh);
        }
        
        Sample{ date_time: dt, files: fcs}
    }
 
    pub fn count_cohort_lines(&self, cohort: &String) -> u64 {
        self.files.values()
            .map(|v: &CohortMap| v.get(cohort).unwrap().to_owned() )
            .sum()
    }
}

#[derive(Clone)]
struct Options {
    interval: i64,
    repo_path: String,
    cohort_fmt: String,
    ignore: Option<Regex>
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
        let ignore_patterns = matches.value_of("ignore").map(|i| Regex::new(i).unwrap());

        Options {
            interval: interval,
            repo_path: repo_path.to_owned(),
            cohort_fmt: cohort_fmt.to_owned(),
            ignore: ignore_patterns
        }
    }
}

pub struct Axe {
    repo: Repository,
    options: Options,
}

impl Axe {
    pub fn new(app_config: App) -> Result<Axe> {
        let options = Options::new(app_config);
        let repo = Repository::open(options.clone().repo_path)
            .chain_err(|| "Couldn't open repository")?;
        Ok(Axe {
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
        match self.options.ignore {
            None => false,
            Some(ref re) => re.is_match(filename)
        }
    }
    
    pub fn collect_samples(&self) -> Result<Vec<Sample>> {
        let ids = self.get_revwalk_ids()
            .chain_err(|| "Unable to obtain revwalk ids")?;

        let duration = Duration::seconds(self.options.interval);
        let first_commit = self.find_commit(&ids[0])?;
        let dt = self.commit_date_time(&first_commit)?;
        
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_unmodified(false)
            .ignore_filemode(true)
            .context_lines(0);

        let commits: Vec<Commit> = ids.iter().fold(vec![first_commit], |mut acc, &id| {
            let commit = self.find_commit(&id).unwrap();
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

        
        let mut mb = MultiBar::new();
        let mut pb = mb.create_bar(trees.len() as u64);
        let mut pb2 = mb.create_bar(trees.len() as u64);

        thread::spawn(move || mb.listen());

        pb.message("Collecting changesets: ");
        pb.format("╢▌▌░╟");
        pb2.message("Processing changesets: ");
        pb2.format("╢▌▌░╟");
        
        let changesets = trees.windows(2).map(|pair|{
                    
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
                
                // TODO Helper function taking my references.... see Alex.
                //   conversion traits
                let diff = if pair[0].tree.is_none() {
                    let rh_tree = pair[1].tree.as_ref().unwrap();
                    
                    self.repo.diff_tree_to_tree(None,
                                                Some(&rh_tree),
                                                Some(&mut diff_opts))
                }else {
                    let lh_tree = pair[0].tree.as_ref().unwrap();
                    let rh_tree = pair[1].tree.as_ref().unwrap();
                    
                    self.repo.diff_tree_to_tree(Some(&lh_tree),
                                                Some(&rh_tree),
                                                Some(&mut diff_opts))
                };
                
                diff.unwrap().foreach(&mut file_cb, None, Some(&mut hunk_cb), None)
                    .unwrap();
            }
            pb.inc();
            changeset
        }).collect::<Vec<Changeset>>();

        pb.finish();

        let samples = {
            let mut inc_pb = || {
                pb2.inc();
                true
            };
            Sample::from_changesets(changesets, &self.options.cohort_fmt, &mut inc_pb)
        };
       
        pb2.finish();
        
        Ok(samples)
    }
}
