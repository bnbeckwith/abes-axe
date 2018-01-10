use chrono::{NaiveDateTime};
use git2::DiffFile;
use std::sync::Arc;
use std::collections::{HashMap};


use change::{Changeset,Change};

type Cohort = String;
type Lines = Arc<Vec<Cohort>>;
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

    pub fn add_changeset(&mut self, changeset: &Changeset, cohort: &String) -> &mut Sample {
        for change in changeset.changes.iter() {
            match change {
                &Change::Add { ref filename, start, length } =>
                {
                    let mut lines_rc = self.files.entry(filename.to_owned())
                        .or_insert(Arc::new(Vec::new()));
                    let end = Arc::make_mut(&mut lines_rc).split_off(start as usize);
                    Arc::make_mut(&mut lines_rc)
                        .extend_from_slice(&vec![cohort.to_owned(); length as usize]);
                    Arc::make_mut(&mut lines_rc).extend_from_slice(end.as_slice());
                },
                &Change::Delete { ref filename, start, length } =>
                {
                    let mut lines = self.files.entry(filename.to_owned())
                        .or_insert(Arc::new(Vec::new()));
                    let start = start as usize;
                    let end = start + length as usize;
                    Arc::make_mut(lines).drain(start..end);
                },
                &Change::AddFile { ref filename, length } =>
                {
                    self.files.insert(filename.to_owned(),
                                      Arc::new(vec![cohort.to_owned(); length as usize]));
                },
                &Change::DeleteFile { ref filename } =>
                {
                    self.files.remove(filename);
                }
            }
        };
        self
    }
    
    pub fn filename(f: &DiffFile) -> FileName {
        String::from(f.path().map(|e| e.to_str().unwrap()).unwrap())
    }

    pub fn count_cohort_lines(&self, cohort: &String) -> i64 {
        self.files.values()
            .map(|v| v.iter().filter(|v| *v == cohort).count() )
            .fold(0, |acc, v| acc + v) as i64
    }
}
