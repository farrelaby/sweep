pub const KNOWN_TARGETS: &[&str] = &[
    "node_modules",
    "target",
    ".next",
    "__pycache__",
    ".venv",
    "venv",
    "dist",
    "build",
    "vendor",
    "Pods",
    "bower_components",
    ".tox",
];

pub const LOCK_FILE_MAPPINGS: &[(&str, &str)] = &[
    ("package-lock.json", "npm"),
    ("pnpm-lock.yaml", "pnpm"),
    ("bun.lock", "bun"),
    ("deno.lock", "deno"),
    ("yarn.lock", "yarn"),
    ("Cargo.lock", "cargo"),
    // ("go.sum", "go"),
    ("Gemfile.lock", "bundler"),
    ("composer.lock", "composer"),
    ("uv.lock", "uv"),
];

pub fn is_known_target(dir_name: &str) -> bool {
    KNOWN_TARGETS.contains(&dir_name)
}

pub fn lookup_package_manager(lock_file_name: &str) -> Option<&'static str> {
    LOCK_FILE_MAPPINGS
        .iter()
        .find(|(name, _)| *name == lock_file_name)
        .map(|(_, pm)| *pm)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_all_known_targets_are_recognized() {
        for target in KNOWN_TARGETS {
            assert!(is_known_target(target), "target {} should be known", target);
        }
    }

    #[test]
    fn test_random_name_is_not_target() {
        assert!(!is_known_target("random_folder"));
        assert!(!is_known_target(".git"));
        assert!(!is_known_target("src"));
    }

    #[test]
    fn test_all_lock_file_mappings_work() {
        for (lock_file, pm) in LOCK_FILE_MAPPINGS {
            assert_eq!(lookup_package_manager(lock_file), Some(*pm));
        }
    }

    #[test]
    fn test_unknown_lock_file_returns_none() {
        assert_eq!(lookup_package_manager("unknown.lock"), None);
    }
}
