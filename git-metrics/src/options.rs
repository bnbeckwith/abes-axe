use clap::App;
use itertools::Itertools;
use regex::Regex;

#[derive(Clone)]
pub struct Options {
    interval: i64,
    repo_path: String,
    cohort_fmt: String,
    ignore: Option<Regex>,
    only: Option<Regex>
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
        let ignore_patterns = matches.values_of("ignore").map(|iter| {
            let re: String = iter.map(|s| String::from(s))
                .intersperse(String::from("|"))
                .collect();
            Regex::new(re.as_str()).unwrap()
        });
        let only_patterns = matches.values_of("only").map(|iter| {
            let re: String = iter.map(|s| String::from(s))
                .intersperse(String::from("|"))
                .collect();
            Regex::new(re.as_str()).unwrap()
        });
        
        Options {
            interval: interval,
            repo_path: repo_path.to_owned(),
            cohort_fmt: cohort_fmt.to_owned(),
            ignore: ignore_patterns,
            only: only_patterns
        }
    }

    pub fn path(&self) -> String {
        self.repo_path.clone()
    }

    pub fn format(&self) -> String {
        self.cohort_fmt.clone()
    }

    pub fn should_ignore(&self, filename: &str) -> bool {
        match self.ignore {
            None => false,
            Some(ref re) => re.is_match(filename)
        }
    }

    pub fn should_keep(&self, filename: &str) -> bool {
        match self.ignore {
            None => true,
            Some(ref re) => re.is_match(filename)
        }
    }

    pub fn interval(&self) -> i64 {
        self.interval
    }
}
