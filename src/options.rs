use regex::Regex;
use clap::App;
use itertools::Itertools;

#[derive(Clone)]
pub struct Options {
    pub interval: i64,
    pub repo_path: String,
    pub cohort_fmt: String,
    pub ignore: Option<Regex>,
    pub only: Option<Regex>
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
}
