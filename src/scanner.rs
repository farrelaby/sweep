use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use chrono::{DateTime, Utc};
use walkdir::WalkDir;

use crate::config;

const SKIP_DIRS: &[&str] = &[".git", ".svn", ".hg"];

#[derive(Debug, Clone)]
pub struct ScannedDir {
    pub path: PathBuf,
    pub size: u64,
    pub last_modified: DateTime<Utc>,
    pub package_manager: Option<String>,
    pub error: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProjectInfo {
    pub path: PathBuf,
    pub name: String,
    pub package_manager: Option<String>,
    pub children: Vec<PathBuf>,
}

#[derive(Debug, Default)]
pub struct ScanOutput {
    pub target_dirs: Vec<ScannedDir>,
    pub projects: Vec<ProjectInfo>,
    pub errors: Vec<String>,
}

pub fn scan(base_path: &Path) -> ScanOutput {
    let mut lock_files: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let mut errors: Vec<String> = Vec::new();

    // Pass 1: Walk tree to find lock files only.
    // filter_entry skips entering known target dirs and VCS dirs entirely.
    let walk = WalkDir::new(base_path)
        .follow_links(false)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !(SKIP_DIRS.contains(&name.as_ref())
                || e.file_type().is_dir() && config::is_known_target(name.as_ref()))
        });

    for entry in walk {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("Cannot access: {}", e));
                continue;
            }
        };

        let file_name = entry.file_name().to_string_lossy().to_string();

        if entry.file_type().is_file()
            && let Some(pm) = config::lookup_package_manager(&file_name)
            && let Some(parent) = entry.path().parent()
        {
            lock_files
                .entry(parent.to_path_buf())
                .or_default()
                .push(pm.to_string());
        }
    }

    // Pass 2: Build projects and find their target children via read_dir.
    let mut projects: Vec<ProjectInfo> = lock_files
        .into_iter()
        .map(|(path, managers)| {
            let name = path
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_default();
            ProjectInfo {
                path,
                name,
                package_manager: managers.first().cloned(),
                children: Vec::new(),
            }
        })
        .collect();

    let mut target_dirs: Vec<ScannedDir> = Vec::new();

    for project in &mut projects {
        let top_level = match fs::read_dir(&project.path) {
            Ok(rd) => rd,
            Err(e) => {
                errors.push(format!("Cannot read {:?}: {}", project.path, e));
                continue;
            }
        };

        for entry in top_level.flatten() {
            let name = entry.file_name().to_string_lossy().to_string();
            if entry.file_type().is_ok_and(|ft| ft.is_dir()) && config::is_known_target(&name) {
                let target_path = entry.path();
                target_dirs.push(ScannedDir {
                    path: target_path.clone(),
                    size: 0,
                    last_modified: DateTime::UNIX_EPOCH,
                    package_manager: project.package_manager.clone(),
                    error: None,
                });
                project.children.push(target_path);
            }
        }
    }

    // Pass 3: Shallow walk for orphan targets (no lock file parent).
    let mut orphan_targets: Vec<PathBuf> = WalkDir::new(base_path)
        .follow_links(false)
        .max_depth(2)
        .into_iter()
        .filter_entry(|e| {
            let name = e.file_name().to_string_lossy();
            !SKIP_DIRS.contains(&name.as_ref())
        })
        .flatten()
        .filter(|e| {
            e.file_type().is_dir() && config::is_known_target(&e.file_name().to_string_lossy())
        })
        .map(|e| e.path().to_path_buf())
        .collect();

    orphan_targets.sort();
    orphan_targets.dedup_by(|a, b| b.starts_with(a));

    for target_path in orphan_targets {
        let is_associated = projects.iter().any(|p| p.children.contains(&target_path));
        if is_associated || target_dirs.iter().any(|d| d.path == target_path) {
            continue;
        }

        target_dirs.push(ScannedDir {
            path: target_path,
            size: 0,
            last_modified: DateTime::UNIX_EPOCH,
            package_manager: None,
            error: None,
        });
    }

    ScanOutput {
        target_dirs,
        projects,
        errors,
    }
}

pub fn scan_target_size(path: &Path) -> Result<(u64, DateTime<Utc>), std::io::Error> {
    let dir = scan_target(path)?;
    Ok((dir.size, dir.last_modified))
}

fn scan_target(path: &Path) -> Result<ScannedDir, std::io::Error> {
    let mut size = 0u64;
    let mut last_modified: Option<std::time::SystemTime> = None;
    let mut errors: Vec<String> = Vec::new();

    let meta = std::fs::metadata(path)?;
    if let Ok(m) = meta.modified() {
        last_modified = Some(m);
    }

    if meta.is_dir() {
        for entry in WalkDir::new(path).into_iter() {
            let entry = match entry {
                Ok(e) => e,
                Err(e) => {
                    errors.push(format!("Cannot access {:?}: {}", e.path(), e));
                    continue;
                }
            };

            let file_meta = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    errors.push(format!("Cannot read metadata {:?}: {}", entry.path(), e));
                    continue;
                }
            };

            if file_meta.is_file() {
                size += file_meta.len();
            }

            if let Ok(m) = file_meta.modified()
                && last_modified.is_none_or(|lm| m > lm)
            {
                last_modified = Some(m);
            }
        }
    }

    let dt = last_modified
        .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
        .and_then(|d| DateTime::from_timestamp(d.as_secs() as i64, d.subsec_nanos()))
        .unwrap_or(DateTime::UNIX_EPOCH);

    let error = if errors.is_empty() {
        None
    } else {
        Some(errors.join("; "))
    };

    Ok(ScannedDir {
        path: path.to_path_buf(),
        size,
        last_modified: dt,
        package_manager: None,
        error,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_dir() -> tempfile::TempDir {
        tempfile::tempdir().expect("failed to create temp dir")
    }

    fn create_dir(path: &Path) {
        fs::create_dir_all(path).expect("failed to create dir");
    }

    fn create_file(path: &Path, size: u64) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("failed to create parent");
        }
        let content = vec![0u8; size as usize];
        fs::write(path, content).expect("failed to write file");
    }

    #[test]
    fn test_scan_finds_node_modules() {
        let dir = create_test_dir();
        let project = dir.path().join("my-app");
        create_file(&project.join("package-lock.json"), 100);
        create_dir(&project.join("node_modules"));
        create_file(&project.join("node_modules/lib/index.js"), 1024);

        let output = scan(dir.path());
        assert_eq!(output.target_dirs.len(), 1);
        assert_eq!(output.target_dirs[0].path, project.join("node_modules"));
        // Fast scan returns size=0; sizes computed separately via scan_target_size
        assert_eq!(output.target_dirs[0].size, 0);
    }

    #[test]
    fn test_scan_detects_package_manager() {
        let dir = create_test_dir();
        let project = dir.path().join("my-app");
        create_file(&project.join("pnpm-lock.yaml"), 50);
        create_dir(&project.join("node_modules"));
        create_file(&project.join("node_modules/pkg/index.js"), 512);

        let output = scan(dir.path());
        assert!(!output.target_dirs.is_empty());
        assert_eq!(
            output.target_dirs[0].package_manager.as_deref(),
            Some("pnpm")
        );
    }

    #[test]
    fn test_scan_ignores_non_targets() {
        let dir = create_test_dir();
        create_dir(&dir.path().join("src"));
        create_file(&dir.path().join("src/main.rs"), 200);
        create_dir(&dir.path().join(".git"));
        create_file(&dir.path().join(".git/config"), 100);

        let output = scan(dir.path());
        assert!(output.target_dirs.is_empty());
    }

    #[test]
    fn test_scan_multiple_projects() {
        let dir = create_test_dir();
        let app1 = dir.path().join("app1");
        let app2 = dir.path().join("app2");
        create_file(&app1.join("package-lock.json"), 50);
        create_dir(&app1.join("node_modules"));
        create_file(&app1.join("node_modules/pkg/index.js"), 1024);
        create_file(&app2.join("Cargo.lock"), 50);
        create_dir(&app2.join("target"));
        create_file(&app2.join("target/debug/app"), 2048);

        let output = scan(dir.path());
        assert_eq!(output.target_dirs.len(), 2);
        assert_eq!(output.projects.len(), 2);
    }

    #[test]
    fn test_scan_multiple_targets_same_project() {
        let dir = create_test_dir();
        let project = dir.path().join("my-app");
        create_file(&project.join("package-lock.json"), 50);
        create_dir(&project.join("node_modules"));
        create_file(&project.join("node_modules/pkg/index.js"), 500);
        create_dir(&project.join(".next"));
        create_file(&project.join(".next/bundle.js"), 1000);

        let output = scan(dir.path());
        assert_eq!(output.target_dirs.len(), 2);
        assert_eq!(output.projects.len(), 1);
        assert_eq!(output.projects[0].children.len(), 2);
    }

    #[test]
    fn test_scan_target_size() {
        let dir = create_test_dir();
        let target = dir.path().join("node_modules");
        create_dir(&target);
        create_file(&target.join("lib/index.js"), 1024);

        let (size, _last_modified) = scan_target_size(&target).unwrap();
        assert!(size >= 1024);
    }
}
