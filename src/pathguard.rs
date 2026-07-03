//! Env-rooted path containment — the single guard for every file the
//! engine touches on behalf of a statement (GIGI_EMIT_DIR writes,
//! GIGI_INGEST_DIR reads).
//!
//! Two layers, in order:
//!
//! 1. **Component-level lexical screen, BEFORE joining.** Any `Prefix`
//!    (drive / UNC), `RootDir` (rooted), or `ParentDir` (`..`)
//!    component rejects the path outright. This kills `C:file`,
//!    `C:\x`, `\\server\share`, `\foo`, `/foo`, and `../x` uniformly
//!    and portably — including the two Windows shapes
//!    (`C:file`, `\foo`) that `Path::is_absolute()` misses and that
//!    `Path::join` silently promotes to a full path replacement.
//!    `CurDir` (`.`) components are stripped as noise; a path with no
//!    remaining components (``, `.`, `./`) is rejected too.
//! 2. **Canonical verification, canonical-to-canonical only.** After
//!    joining the screened path onto the root, `fs::canonicalize`
//!    resolves symlinks/junctions and the result must `starts_with`
//!    the canonicalized root. Windows canonicalize yields `\\?\`
//!    verbatim paths — comparing canonical against canonical is what
//!    makes that prefix a non-issue.
//!
//! Read vs write:
//!
//! - `must_exist = true` (INGEST): the source has to exist to be
//!   ingested, so the candidate itself is canonicalized and returned.
//!   A missing file surfaces as [`PathGuardError::Io`] with
//!   `ErrorKind::NotFound`.
//! - `must_exist = false` (EMIT): parent directories are created (the
//!   emit executor's `create_dir_all` has always implied them), the
//!   PARENT is canonicalized and containment-checked, and the
//!   UNcanonicalized `root.join(screened)` is returned so downstream
//!   receipts keep printing the path the operator configured.

use std::path::{Component, Path, PathBuf};

/// Why a path was refused (or could not be resolved) by [`contain`].
#[derive(Debug)]
pub enum PathGuardError {
    /// The environment variable naming the containment root is not set
    /// (or is set to the empty string). Fail closed.
    RootUnset { var: String },
    /// The root itself cannot be canonicalized (missing directory,
    /// permissions). Names the env var and the value it held.
    RootUnresolvable {
        var: String,
        root: PathBuf,
        source: std::io::Error,
    },
    /// Rejected by the lexical component screen: drive/UNC prefix,
    /// rooted, `..`, or no usable components at all.
    Escape {
        path: String,
        root: PathBuf,
        reason: &'static str,
    },
    /// Canonical resolution landed OUTSIDE the canonical root — a
    /// symlink or junction inside the root points out of it.
    NotContained { path: PathBuf, root: PathBuf },
    /// Filesystem error resolving the candidate under the root. For
    /// `must_exist = true`, `ErrorKind::NotFound` means the file is
    /// simply absent under the root.
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
}

impl std::fmt::Display for PathGuardError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PathGuardError::RootUnset { var } => {
                write!(f, "containment root {var} is not set")
            }
            PathGuardError::RootUnresolvable { var, root, source } => write!(
                f,
                "containment root {var}='{}' cannot be resolved: {source}",
                root.display()
            ),
            PathGuardError::Escape { path, root, reason } => write!(
                f,
                "path '{path}' escapes containment root '{}': {reason}",
                root.display()
            ),
            PathGuardError::NotContained { path, root } => write!(
                f,
                "resolved path '{}' is not under containment root '{}'",
                path.display(),
                root.display()
            ),
            PathGuardError::Io { path, source } => {
                write!(f, "cannot resolve '{}': {source}", path.display())
            }
        }
    }
}

impl std::error::Error for PathGuardError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            PathGuardError::RootUnresolvable { source, .. }
            | PathGuardError::Io { source, .. } => Some(source),
            _ => None,
        }
    }
}

/// Resolve `user_path` strictly under the directory named by the
/// `root_env` environment variable.
///
/// See the module docs for the two-layer semantics. `root_env` is a
/// parameter (not a constant) so every caller — and every parallel
/// test — names its own knob.
pub fn contain(
    root_env: &str,
    user_path: &str,
    must_exist: bool,
) -> Result<PathBuf, PathGuardError> {
    // (a) the root, from the environment. Empty counts as unset —
    // "" would otherwise join into relative-to-cwd paths.
    let root = match std::env::var(root_env) {
        Ok(v) if !v.trim().is_empty() => PathBuf::from(v),
        _ => {
            return Err(PathGuardError::RootUnset {
                var: root_env.to_string(),
            })
        }
    };

    // (b) lexical component screen BEFORE joining.
    let mut screened = PathBuf::new();
    for comp in Path::new(user_path).components() {
        match comp {
            Component::Prefix(_) => {
                return Err(PathGuardError::Escape {
                    path: user_path.to_string(),
                    root,
                    reason: "drive/UNC-prefixed paths are not allowed; \
                             use a path relative to the root",
                })
            }
            Component::RootDir => {
                return Err(PathGuardError::Escape {
                    path: user_path.to_string(),
                    root,
                    reason: "absolute paths are not allowed; use a path \
                             relative to the root",
                })
            }
            Component::ParentDir => {
                return Err(PathGuardError::Escape {
                    path: user_path.to_string(),
                    root,
                    reason: "'..' components are not allowed",
                })
            }
            Component::CurDir => {}
            Component::Normal(seg) => screened.push(seg),
        }
    }
    if screened.as_os_str().is_empty() {
        return Err(PathGuardError::Escape {
            path: user_path.to_string(),
            root,
            reason: "path has no usable components (resolves to the \
                     root itself)",
        });
    }

    if must_exist {
        // (c)+(d) read mode: root must exist, candidate must exist,
        // and the canonical candidate must sit under the canonical
        // root (symlink/junction defense). Return the canonical path —
        // it is the artifact the check verified.
        let canonical_root =
            std::fs::canonicalize(&root).map_err(|e| PathGuardError::RootUnresolvable {
                var: root_env.to_string(),
                root: root.clone(),
                source: e,
            })?;
        let candidate = canonical_root.join(&screened);
        let canonical = std::fs::canonicalize(&candidate).map_err(|e| PathGuardError::Io {
            path: candidate.clone(),
            source: e,
        })?;
        if !canonical.starts_with(&canonical_root) {
            return Err(PathGuardError::NotContained {
                path: canonical,
                root: canonical_root,
            });
        }
        Ok(canonical)
    } else {
        // (d) write mode: create parent dirs (as the emit executor
        // always has), canonicalize the PARENT, require containment,
        // and hand back the uncanonicalized candidate so receipts keep
        // the operator-configured spelling.
        let candidate = root.join(&screened);
        let parent = candidate
            .parent()
            .expect("screened path is non-empty, so the candidate has a parent")
            .to_path_buf();
        std::fs::create_dir_all(&parent).map_err(|e| PathGuardError::Io {
            path: parent.clone(),
            source: e,
        })?;
        let canonical_root =
            std::fs::canonicalize(&root).map_err(|e| PathGuardError::RootUnresolvable {
                var: root_env.to_string(),
                root: root.clone(),
                source: e,
            })?;
        let canonical_parent =
            std::fs::canonicalize(&parent).map_err(|e| PathGuardError::Io {
                path: parent.clone(),
                source: e,
            })?;
        if !canonical_parent.starts_with(&canonical_root) {
            return Err(PathGuardError::NotContained {
                path: canonical_parent,
                root: canonical_root,
            });
        }
        Ok(candidate)
    }
}
