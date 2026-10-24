use git2::{Delta, DiffOptions, IndexAddOption, Oid, Repository, Signature};
use std::path::{Path, PathBuf};

pub struct GitSSOT {
    repo: Repository,
}

#[derive(Debug)]
pub struct TrackedChange {
    pub path: PathBuf,
    pub old_content: Option<Vec<u8>>,
    pub new_content: Option<Vec<u8>>,
    pub status: Delta,
}

impl GitSSOT {
    /// Open the repository at or above the specified path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, git2::Error> {
        let repo = Repository::discover(path)?;
        Ok(Self { repo })
    }

    /// دسترسی عمومی به شیء مخزن گیت
    pub fn repo(&self) -> &Repository {
        &self.repo
    }

    /// دریافت هش کامیت فعلی مخزن (Current HEAD OID String)
    pub fn current_head(&self) -> Result<String, git2::Error> {
        let head = self.repo.head()?;
        let commit = head.peel_to_commit()?;
        Ok(commit.id().to_string())
    }

    /// Loads the path and raw content of all tracked source files in the repository
    pub fn load_all_tracked_sources(&self) -> Result<Vec<(String, Vec<u8>)>, git2::Error> {
        let index = self.repo.index()?;
        let workdir = self.repo.workdir().unwrap_or(Path::new("."));
        let mut sources = Vec::new();

        for entry in index.iter() {
            let path_str = match std::str::from_utf8(&entry.path) {
                Ok(p) => p,
                Err(_) => continue,
            };

            // Filter for supported source files (Rust)
            if !path_str.ends_with(".rs") {
                continue;
            }

            let full_path = workdir.join(path_str);
            if full_path.is_file() {
                if let Ok(content) = std::fs::read(&full_path) {
                    sources.push((path_str.to_string(), content));
                }
            }
        }

        Ok(sources)
    }

    /// Extract all tracked and modified source files ignoring untracked/ignored entries
    pub fn get_semantic_changes(&self) -> Result<Vec<TrackedChange>, git2::Error> {
        let mut diff_opts = DiffOptions::new();
        diff_opts.include_untracked(false);
        diff_opts.include_ignored(false);

        let head_tree = self.repo.head().ok().and_then(|h| h.peel_to_tree().ok());
        let diff = self
            .repo
            .diff_tree_to_workdir_with_index(head_tree.as_ref(), Some(&mut diff_opts))?;

        let mut changes = Vec::new();
        let workdir = self.repo.workdir().unwrap_or(Path::new("."));

        for delta in diff.deltas() {
            let old_file = delta.old_file();
            let new_file = delta.new_file();

            let rel_path = match new_file.path().or_else(|| old_file.path()) {
                Some(p) => p,
                None => continue,
            };

            // Filter for supported source extensions (e.g. Rust)
            if rel_path.extension().and_then(|s| s.to_str()) != Some("rs") {
                continue;
            }

            let full_path = workdir.join(rel_path);

            // Fetch previous blob from Git Object Database
            let old_content = if old_file.id().is_zero() {
                None
            } else {
                self.repo
                    .find_blob(old_file.id())
                    .ok()
                    .map(|b| b.content().to_vec())
            };

            // Fetch current disk content
            let new_content = if full_path.exists() {
                std::fs::read(&full_path).ok()
            } else {
                None
            };

            changes.push(TrackedChange {
                path: rel_path.to_path_buf(),
                old_content,
                new_content,
                status: delta.status(),
            });
        }

        Ok(changes)
    }

    /// Stages all modified tracked files and records an AST-aware commit directly into the Git ODB
    pub fn stage_and_commit(&self, message: &str) -> Result<Oid, git2::Error> {
        let mut index = self.repo.index()?;

        // Stage all modifications from tracked files (equivalent to git add)
        index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
        index.write()?;
        let tree_id = index.write_tree()?;
        let tree = self.repo.find_tree(tree_id)?;

        // Retrieve current HEAD as the parent commit
        let parent_commit = match self.repo.head() {
            Ok(head_ref) => Some(head_ref.peel_to_commit()?),
            Err(_) => None,
        };

        let mut parents = Vec::new();
        if let Some(ref parent) = parent_commit {
            parents.push(parent);
        }

        // Generate Git author/committer signature
        let sig = self
            .repo
            .signature()
            .or_else(|_| Signature::now("Vora AI", "ai@vora.dev"))?;

        // Create the direct commit object in the Git repository
        let commit_id = self
            .repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)?;

        Ok(commit_id)
    }
}
