use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DesktopEntry {
    pub name: String,
    pub exec: String,
    pub icon: Option<String>,
    pub categories: Vec<String>,
}

pub struct AppLauncher {
    entries: Vec<DesktopEntry>,
    name_index: HashMap<String, usize>,
}

impl AppLauncher {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            name_index: HashMap::new(),
        }
    }

    pub fn load_desktop_files(&mut self) {
        let xdg_data_dirs = std::env::var("XDG_DATA_DIRS")
            .unwrap_or_else(|_| "/usr/local/share:/usr/share".to_string());

        let mut search_paths: Vec<PathBuf> = xdg_data_dirs
            .split(':')
            .map(PathBuf::from)
            .map(|p| p.join("applications"))
            .collect();

        if let Ok(home) = std::env::var("HOME") {
            search_paths.insert(0, PathBuf::from(home).join(".local/share/applications"));
        }

        for path in search_paths {
            if path.is_dir() {
                self.scan_directory(&path);
            }
        }

        tracing::info!("Loaded {} desktop entries", self.entries.len());
    }

    fn scan_directory(&mut self, path: &PathBuf) {
        if let Ok(entries) = fs::read_dir(path) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "desktop").unwrap_or(false) {
                    if let Some(entry) = self.parse_desktop_file(&path) {
                        let name = entry.name.clone();
                        let idx = self.entries.len();
                        self.name_index.insert(name.to_lowercase(), idx);
                        self.entries.push(entry);
                    }
                }
            }
        }
    }

    fn parse_desktop_file(&self, path: &PathBuf) -> Option<DesktopEntry> {
        let content = fs::read_to_string(path).ok()?;
        let mut name = None;
        let mut exec = None;
        let mut icon = None;
        let mut categories = Vec::new();

        for line in content.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.is_empty() {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                match key.trim() {
                    "Name" => name = Some(value.trim().to_string()),
                    "Exec" => exec = Some(value.trim().to_string()),
                    "Icon" => icon = Some(value.trim().to_string()),
                    "Categories" => {
                        categories = value
                            .trim()
                            .split(';')
                            .filter(|s| !s.is_empty())
                            .map(String::from)
                            .collect();
                    }
                    "NoDisplay" => {
                        if value.trim() == "true" {
                            return None;
                        }
                    }
                    _ => {}
                }
            }
        }

        let name = name?;
        let exec = exec?;

        Some(DesktopEntry {
            name,
            exec,
            icon,
            categories,
        })
    }

    pub fn search(&self, query: &str) -> Vec<&DesktopEntry> {
        let query_lower = query.to_lowercase();
        self.entries
            .iter()
            .filter(|e| e.name.to_lowercase().contains(&query_lower))
            .collect()
    }

    pub fn spawn(&self, name: &str) -> std::io::Result<std::process::Child> {
        let idx = self
            .name_index
            .get(&name.to_lowercase())
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "App not found"))?;
        let entry = &self.entries[*idx];

        let mut parts = entry.exec.split_whitespace();
        let program = parts
            .next()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "No exec command"))?;

        std::process::Command::new(program).args(parts).spawn()
    }
}

impl Default for AppLauncher {
    fn default() -> Self {
        Self::new()
    }
}
