use serde::Deserialize;
use std::fs;
use std::path::Path;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub ignore: Option<Vec<String>>,
}

impl Config {
    pub fn load() -> Self {
        let config_path = Path::new(".agg-files");
        if config_path.exists() {
            if let Ok(contents) = fs::read_to_string(config_path) {
                if let Ok(config) = serde_yaml::from_str(&contents) {
                    return config;
                }
            }
        }
        // Return default config if file doesn't exist or can't be parsed
        Self { ignore: None }
    }

    pub fn should_ignore(&self, path: &str) -> bool {
        if let Some(ignore_patterns) = &self.ignore {
            for pattern in ignore_patterns {
                let regex = glob_to_regex(pattern);
                if regex.is_match(path) {
                    return true;
                }
            }
        }
        false
    }
}

fn glob_to_regex(pattern: &str) -> regex::Regex {
    let cleaned_pattern = pattern
        .replace(".", "\\.")
        .replace("*", ".*")
        .replace("/", "\\/");
    
    let regex_str = cleaned_pattern
        .trim_start_matches("./")
        .to_string();
    
    regex::Regex::new(&format!("^.*{}.*$", regex_str)).unwrap()
}

