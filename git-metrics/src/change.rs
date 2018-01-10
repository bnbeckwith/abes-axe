use chrono::{NaiveDateTime};
use git2::{Delta,DiffDelta,DiffHunk};

use sample::Sample;

pub enum Change {
    Add {filename: String, start: u32, length: u32},
    Delete {filename: String, start: u32, length: u32},
    DeleteFile {filename: String},
    AddFile {filename: String, length: u32}
}

pub struct Changeset {
    pub date_time: NaiveDateTime,
    pub changes: Vec<Change>  
}

impl Changeset { 
    pub fn new(dt: NaiveDateTime) -> Changeset {
        Changeset{
            date_time: dt,
            changes: Vec::new()
        }
    }

    pub fn process_added(&mut self, d: DiffDelta, h: DiffHunk) -> bool {
        let filename = Sample::filename(&d.new_file());
        self.changes.push(Change::AddFile{filename: filename, length: h.new_lines()});
        true
    }

    pub fn process_deleted(&mut self, d: DiffDelta, _h: DiffHunk) -> bool {
        let filename = Sample::filename(&d.old_file());
        self.changes.push(Change::DeleteFile{filename: filename});
        true
    }
    
    pub fn process_modified(&mut self, d: DiffDelta, h: DiffHunk) -> bool {
        let filename = Sample::filename(&d.new_file());
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
