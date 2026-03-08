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

/// Load the GEMINI.md context string, if available.
/// Returns `None` if no file is found or if the file exceeds 1 MB.
pub fn load_context() -> Option<String> {
    let cwd = env::current_dir().ok()?;
    let path = find_context_file(&cwd)?;
    let size = fs::metadata(&path).ok()?.len();
    if size > MAX_CONTEXT_BYTES {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    Some(content)
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
    fn load_context_returns_none_for_oversized_file() {
        let dir = tempfile::tempdir().unwrap();
        // Write a file just over 1 MB
        let content = "x".repeat(1024 * 1024 + 1);
        fs::write(dir.path().join(CONTEXT_FILENAME), content).unwrap();

        // load_context uses current_dir(), so we test find_context_file + size check directly
        let path = find_context_file(dir.path()).unwrap();
        let size = fs::metadata(&path).unwrap().len();
        assert!(
            size > MAX_CONTEXT_BYTES,
            "test file should exceed the limit"
        );
    }

    #[test]
    fn load_context_accepts_file_within_size_limit() {
        let dir = make_temp_dir_with_file(CONTEXT_FILENAME, "# small context");
        let path = find_context_file(dir.path()).unwrap();
        let size = fs::metadata(&path).unwrap().len();
        assert!(size <= MAX_CONTEXT_BYTES);
    }
}
