use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const CONTEXT_FILENAME: &str = "GEMINI.md";

/// Search for GEMINI.md starting from `cwd` up to the git root (or fs root).
/// Returns the path of the first file found, or `None`.
fn find_context_file(start: &Path) -> Option<PathBuf> {
    for dir in start.ancestors() {
        let candidate = dir.join(CONTEXT_FILENAME);
        if candidate.is_file() {
            return Some(candidate);
        }
        // Stop at a git repository boundary
        if dir.join(".git").exists() {
            break;
        }
    }
    None
}

pub(crate) const MAX_CONTEXT_BYTES: u64 = 1024 * 1024; // 1 MB

/// Read a context file at `path`, returning `None` if missing or over 1 MB.
fn read_context_file(path: &Path) -> Option<String> {
    use std::io::Read;
    let file = fs::File::open(path).ok()?;
    let mut buf = Vec::new();
    file.take(MAX_CONTEXT_BYTES + 1)
        .read_to_end(&mut buf)
        .ok()?;
    if buf.len() > MAX_CONTEXT_BYTES as usize {
        return None;
    }
    String::from_utf8(buf).ok()
}

/// Load the global GEMINI.md from `~/.gemini/GEMINI.md`.
fn load_global_context(home: &Path) -> Option<String> {
    read_context_file(&home.join(".gemini").join(CONTEXT_FILENAME))
}

/// Load the GEMINI.md context string, if available.
///
/// Reads from two locations (in order) and concatenates them:
/// 1. `~/.gemini/GEMINI.md` (global)
/// 2. The first `GEMINI.md` found from cwd up to the git root (local)
///
/// Returns `None` if neither file is found.
pub fn load_context() -> Option<String> {
    let home = env::var("HOME").ok().map(PathBuf::from);
    let global = home.as_deref().and_then(load_global_context);

    let cwd = env::current_dir().ok()?;
    let local = find_context_file(&cwd).and_then(|p| read_context_file(&p));

    match (global, local) {
        (Some(g), Some(l)) => Some(format!("{}\n\n{}", g, l)),
        (Some(g), None) => Some(g),
        (None, Some(l)) => Some(l),
        (None, None) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn make_temp_dir_with_file(filename: &str, content: &str) -> TempDir {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join(filename), content).unwrap();
        dir
    }

    fn make_fake_home(content: &str) -> TempDir {
        let home = tempfile::tempdir().unwrap();
        let gemini_dir = home.path().join(".gemini");
        fs::create_dir(&gemini_dir).unwrap();
        fs::write(gemini_dir.join(CONTEXT_FILENAME), content).unwrap();
        home
    }

    #[test]
    fn finds_context_file_in_same_dir() {
        let dir = make_temp_dir_with_file(CONTEXT_FILENAME, "# Context");
        let result = find_context_file(dir.path());
        assert!(result.is_some());
        assert_eq!(result.unwrap(), dir.path().join(CONTEXT_FILENAME));
    }

    #[test]
    fn finds_context_file_in_parent_dir() {
        let parent = tempfile::tempdir().unwrap();
        fs::write(parent.path().join(CONTEXT_FILENAME), "# Context").unwrap();
        let child = parent.path().join("subdir");
        fs::create_dir(&child).unwrap();

        let result = find_context_file(&child);
        assert!(result.is_some());
        assert_eq!(result.unwrap(), parent.path().join(CONTEXT_FILENAME));
    }

    #[test]
    fn stops_at_git_boundary() {
        let grandparent = tempfile::tempdir().unwrap();
        // GEMINI.md is in grandparent, but .git is in parent — should not find it
        fs::write(grandparent.path().join(CONTEXT_FILENAME), "# Context").unwrap();
        let parent = grandparent.path().join("repo");
        fs::create_dir(&parent).unwrap();
        fs::create_dir(parent.join(".git")).unwrap();
        let child = parent.join("src");
        fs::create_dir(&child).unwrap();

        let result = find_context_file(&child);
        assert!(result.is_none());
    }

    #[test]
    fn returns_none_when_no_file_exists() {
        let dir = tempfile::tempdir().unwrap();
        // Create a .git to bound the search within the temp dir
        fs::create_dir(dir.path().join(".git")).unwrap();
        let result = find_context_file(dir.path());
        assert!(result.is_none());
    }

    #[test]
    fn read_context_file_returns_none_for_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join(CONTEXT_FILENAME);
        let content = "x".repeat(1024 * 1024 + 1);
        fs::write(&path, content).unwrap();
        assert!(
            read_context_file(&path).is_none(),
            "read_context_file should return None for files over 1 MB"
        );
    }

    #[test]
    fn read_context_file_accepts_file_within_size_limit() {
        let dir = make_temp_dir_with_file(CONTEXT_FILENAME, "# small context");
        let path = dir.path().join(CONTEXT_FILENAME);
        assert_eq!(read_context_file(&path).as_deref(), Some("# small context"));
    }

    #[test]
    fn load_global_context_reads_home_gemini_md() {
        let home = make_fake_home("# global context");
        let result = load_global_context(home.path());
        assert_eq!(result.as_deref(), Some("# global context"));
    }

    #[test]
    fn load_global_context_returns_none_when_missing() {
        let home = tempfile::tempdir().unwrap();
        let result = load_global_context(home.path());
        assert!(result.is_none());
    }

    #[test]
    fn load_global_context_returns_none_for_oversized_file() {
        let home = tempfile::tempdir().unwrap();
        let gemini_dir = home.path().join(".gemini");
        fs::create_dir(&gemini_dir).unwrap();
        let content = "x".repeat(1024 * 1024 + 1);
        fs::write(gemini_dir.join(CONTEXT_FILENAME), content).unwrap();
        let result = load_global_context(home.path());
        assert!(result.is_none());
    }

    #[test]
    fn concatenates_global_and_local_with_blank_line() {
        let home = make_fake_home("# global");
        let local_dir = tempfile::tempdir().unwrap();
        fs::write(local_dir.path().join(CONTEXT_FILENAME), "# local").unwrap();
        fs::create_dir(local_dir.path().join(".git")).unwrap();

        let global = load_global_context(home.path());
        let local = find_context_file(local_dir.path()).and_then(|p| read_context_file(&p));

        let result = match (global, local) {
            (Some(g), Some(l)) => Some(format!("{}\n\n{}", g, l)),
            (Some(g), None) => Some(g),
            (None, Some(l)) => Some(l),
            (None, None) => None,
        };
        assert_eq!(result.as_deref(), Some("# global\n\n# local"));
    }

    #[test]
    fn returns_global_only_when_no_local() {
        let home = make_fake_home("# global only");
        let result = load_global_context(home.path());
        assert_eq!(result.as_deref(), Some("# global only"));
    }
}
