use std::fs;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

use crate::cli::CliArgs;
use crate::gitignore_helper::GitignoreHelper;
use crate::pattern_matcher::PatternMatcher;
use crate::config::Config;

pub struct FileProcessor {
    args: CliArgs,
    gitignore: Option<ignore::gitignore::Gitignore>,
    pattern_matcher: PatternMatcher,
    working_dir: PathBuf,
    config: Config,
}

impl FileProcessor {
    pub fn new(args: CliArgs, working_dir: PathBuf) -> Self {
        let gitignore = if !args.ignore_gitignore {
            GitignoreHelper::build()
        } else {
            None
        };

        Self {
            args,
            gitignore,
            pattern_matcher: PatternMatcher::new(),
            working_dir,
            config: Config::load(),
        }
    }

    pub fn process(&self) {
        if self.args.sort_by_size {
            self.process_with_size_sorting();
        } else {
            // existing processing logic
            for pattern in &self.args.patterns {
                let path = Path::new(pattern);
                if path.exists() {
                    if path.is_dir() {
                        self.process_directory(path);
                    } else {
                        self.process_single_file(path);
                    }
                } else {
                    self.process_glob_pattern(pattern);
                }
            }
        }
    }

    fn process_with_size_sorting(&self) {
        let mut file_sizes: Vec<(PathBuf, usize)> = Vec::new();

        // Collect all matching files and their sizes
        for pattern in &self.args.patterns {
            let path = Path::new(pattern);
            if path.exists() {
                if path.is_dir() {
                    self.collect_files_with_sizes(path, &mut file_sizes);
                } else {
                    if let Ok(size) = fs::metadata(path).map(|m| m.len() as usize) {
                        file_sizes.push((path.to_path_buf(), size));
                    }
                }
            } else {
                self.collect_files_from_glob(pattern, &mut file_sizes);
            }
        }

        // Sort files by size (largest first)
        file_sizes.sort_by(|a, b| b.1.cmp(&a.1));

        // Print files and their sizes
        for (path, size) in file_sizes {
            if let Ok(relative_path) = path.strip_prefix(&self.working_dir) {
                println!("# File: ./{} ({} bytes)", relative_path.display(), size);
            } else {
                println!("# File: {} ({} bytes)", path.display(), size);
            }
        }
    }

    fn collect_files_from_glob(&self, pattern: &str, files: &mut Vec<(PathBuf, usize)>) {
        let regex = self.pattern_matcher.glob_to_regex(pattern);
        let walker = self.create_walker();
        
        for entry in walker.into_iter().filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && regex.is_match(path.to_str().unwrap_or("")) {
                    if let Ok(size) = fs::metadata(path).map(|m| m.len() as usize) {
                        files.push((path.to_path_buf(), size));
                    }
                }
            }
        }
    }

    fn collect_files_with_sizes(&self, dir: &Path, files: &mut Vec<(PathBuf, usize)>) {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker.filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    if let Ok(size) = fs::metadata(path).map(|m| m.len() as usize) {
                        files.push((path.to_path_buf(), size));
                    }
                }
            }
        }
    }

    fn process_glob_pattern(&self, pattern: &str) {
        let regex = self.pattern_matcher.glob_to_regex(pattern);
        let walker = self.create_walker();
        
        for entry in walker.into_iter().filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() && regex.is_match(path.to_str().unwrap_or("")) {
                    self.process_single_file(path);
                }
            }
        }
    }

    fn process_directory(&self, dir: &Path) {
        let walker = WalkDir::new(dir).into_iter();
        for entry in walker.filter_entry(|e| self.should_process_entry(e.path())) {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_file() {
                    self.process_single_file(path);
                }
            }
        }
    }

    fn create_walker(&self) -> WalkDir {
        if self.args.recursive {
            WalkDir::new(&self.working_dir)
        } else {
            WalkDir::new(&self.working_dir).max_depth(1)
        }
    }

    fn should_process_entry(&self, path: &Path) -> bool {
        // Convert path to string for config checking
        let path_str = path.to_str().unwrap_or("");

        // Check config ignore patterns
        if self.config.should_ignore(path_str) {
            return false;
        }

        // First check if it's a .git directory or within one
        if path.components().any(|c| c.as_os_str() == ".git") {
            return false;
        }

        // Then check gitignore if enabled
        if let Some(gi) = &self.gitignore {
            !gi.matched(path, path.is_dir()).is_ignore()
        } else {
            true
        }
    }

    fn process_single_file(&self, path: &Path) {
        println!("# File: {}", path.display());
        if !self.args.files_only {
            match fs::read_to_string(path) {
                Ok(contents) => {
                    println!("{}", contents);
                    println!("\n=====================\n");
                }
                Err(_) => println!("Error reading file: {}", path.display()),
            }
        }
    }
}
