use std::fs;
use std::path::Path;

const MIGRATION_DIRS: &[&str] = &[
    "goals_migrations",
    "logs_migrations",
    "memory_migrations",
    "migrations",
];

fn main() {
    for dir in MIGRATION_DIRS {
        visit_dir(Path::new(dir));
    }
}

fn visit_dir(dir: &Path) {
    if !dir.exists() {
        return;
    }

    println!("cargo:rerun-if-changed={}", dir.display());

    let entries = match fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        println!("cargo:rerun-if-changed={}", path.display());
        if path.is_dir() {
            visit_dir(path.as_path());
        }
    }
}
