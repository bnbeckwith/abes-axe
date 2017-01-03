use std::collections::{HashMap};

use git2::{DiffHunk, DiffDelta, DiffFile, Delta, Oid};
use chrono::NaiveDateTime;

type Lines = Vec<Oid>;
type FileName = String;
type FileMap = HashMap<FileName, Lines>;

#[derive(Clone)]
pub struct Sample {
    date_time: NaiveDateTime,
    files: FileMap
}

impl Sample {
    pub fn new(dt: &NaiveDateTime) -> Sample {
        Sample {
            date_time: *dt,
            files: HashMap::new()
        }
    }

    pub fn clone_and_date(&mut self, dt: &NaiveDateTime) -> Sample {
        let mut sample = self.clone();
        sample.date_time = *dt;
        sample
    }

    pub fn add_diff_hunk(&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {

        match delta.status() {
            Delta::Added => {
                self.process_added(delta, hunk)
            },
            Delta::Deleted => {
                self.process_deleted(delta, hunk)
            },
            Delta::Modified => {
                self.process_modified(delta, hunk)
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

    fn filename(f: &DiffFile) -> String {
        String::from(f.path().map(|e| e.to_str().unwrap()).unwrap())
    }

    fn get_lines(&self, filename: FileName) -> Lines {
        match self.files.get(&filename) {
            Some(v) => v.to_owned(),
            None    => Vec::new()
        }
    }
    
    fn process_added (&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        let oid = delta.new_file().id();
        let mut lines = self.get_lines(filename);
        let start = hunk.new_start() - 1;
        let end = start + hunk.new_lines(); 
        for n in start..end {
            lines.insert(n as usize, oid);
        }
        self
    }
    
    fn process_deleted (&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.old_file());
        let mut lines = self.get_lines(filename);

        let start = hunk.new_start() - 1;
        let end = start + hunk.old_lines();
        for n in start..end {
            lines.remove(n as usize);
        }
        self
    }
    
    fn process_modified (&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        let oid = delta.new_file().id();
        let mut lines = self.get_lines(filename);
        let start = hunk.new_start() - 1;
        let end = start + hunk.new_lines(); 
        for n in start..end {
            lines[n as usize] = oid;
        }
        self
    }
    
    fn process_renamed (&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        self
    }
    
    fn process_copied (&mut self, delta: DiffDelta, hunk: DiffHunk) -> &mut Sample {
        let filename = Sample::filename(&delta.new_file());
        self
    }
}
