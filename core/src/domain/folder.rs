use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitoredFolder {
    pub id: String,
    pub path: String,
    pub name: String,
    #[serde(default)]
    pub only_local_checks: bool,
    /// Whether this folder is expected to hold code projects.
    ///
    /// The Uninitialized category answers "a project you forgot to `git init`",
    /// which only makes sense where every subfolder is meant to be a project.
    /// Pointed at a general-purpose folder (Documents, say) the same rule turns
    /// every ordinary directory into a finding and buries the repositories that
    /// are actually there, so such a folder is registered with this off.
    ///
    /// Defaults to on, both for a new folder and for one stored before the flag
    /// existed, so an upgrade keeps the behavior the user already had.
    #[serde(default = "enabled")]
    pub detect_uninitialized: bool,
}

/// `serde`'s default for a missing `bool` is `false`; this one defaults to on.
fn enabled() -> bool {
    true
}

impl MonitoredFolder {
    #[must_use]
    pub fn new(
        path: String,
        name: String,
        only_local_checks: bool,
        detect_uninitialized: bool,
    ) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            path,
            name,
            only_local_checks,
            detect_uninitialized,
        }
    }
}

/// How a candidate path relates to a folder that is already monitored.
///
/// Overlapping folders are rejected rather than de-duplicated: a scan runs each
/// monitored folder independently, so a repository under both would be visited
/// twice in the same pass, with two `git fetch` processes racing inside one
/// `.git`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FolderOverlap {
    /// The same directory.
    Same,
    /// The candidate sits inside the monitored folder.
    Inside,
    /// The candidate contains the monitored folder.
    Contains,
}

impl FolderOverlap {
    /// Why the folder was rejected, phrased for the add/edit form.
    #[must_use]
    pub fn message(self, existing: &MonitoredFolder) -> String {
        let overlap = match self {
            Self::Same => {
                return format!("This folder is already monitored as \"{}\".", existing.name);
            }
            Self::Inside => "is inside",
            Self::Contains => "contains",
        };

        format!(
            "This folder {overlap} \"{}\" ({}), which is already monitored. \
             Overlapping folders scan the same repositories twice in one pass.",
            existing.name, existing.path
        )
    }
}

/// Resolve `..`, `.` and symlinks when the path exists.
///
/// A path typed into the form may not exist yet, and comparing it as written is
/// the best available answer in that case.
fn resolve(path: &str) -> PathBuf {
    let path = Path::new(path.trim());
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    pub folders: Vec<MonitoredFolder>,
}

impl Config {
    /// The first monitored folder that overlaps `path`.
    ///
    /// `skip_id` excludes one folder from the comparison so that editing a
    /// folder without moving it does not report the folder against itself.
    ///
    /// Paths are compared by component, so `/a/bc` is correctly *not* inside
    /// `/a/b`. A difference only in case is not detected on a case-insensitive
    /// volume unless both paths exist.
    #[must_use]
    pub fn overlapping_folder(
        &self,
        path: &str,
        skip_id: Option<&str>,
    ) -> Option<(&MonitoredFolder, FolderOverlap)> {
        let candidate = resolve(path);

        self.folders
            .iter()
            .filter(|folder| Some(folder.id.as_str()) != skip_id)
            .find_map(|folder| {
                let existing = resolve(&folder.path);

                let overlap = if candidate == existing {
                    FolderOverlap::Same
                } else if candidate.starts_with(&existing) {
                    FolderOverlap::Inside
                } else if existing.starts_with(&candidate) {
                    FolderOverlap::Contains
                } else {
                    return None;
                };

                Some((folder, overlap))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(paths: &[&str]) -> Config {
        Config {
            folders: paths
                .iter()
                .map(|p| MonitoredFolder::new((*p).to_string(), (*p).to_string(), false, true))
                .collect(),
        }
    }

    #[test]
    fn a_folder_stored_before_the_flag_existed_still_detects_uninitialized() {
        // The upgrade path: `config.json` written by an older version has no
        // `detectUninitialized` key, and must keep the behavior it had.
        let json = r#"{"folders":[{"id":"1","path":"/a/dev","name":"Dev"}]}"#;
        let config: Config = serde_json::from_str(json).unwrap();
        assert!(config.folders[0].detect_uninitialized);
        assert!(!config.folders[0].only_local_checks);
    }

    #[test]
    fn an_unrelated_folder_does_not_overlap() {
        let config = config(&["/a/dev"]);
        assert!(config.overlapping_folder("/a/work", None).is_none());
    }

    #[test]
    fn a_sibling_sharing_a_name_prefix_does_not_overlap() {
        // The bug a string `starts_with` would introduce.
        let config = config(&["/a/dev"]);
        assert!(config.overlapping_folder("/a/development", None).is_none());
    }

    #[test]
    fn the_same_path_overlaps() {
        let config = config(&["/a/dev"]);
        let (_, overlap) = config.overlapping_folder("/a/dev", None).unwrap();
        assert_eq!(overlap, FolderOverlap::Same);
    }

    #[test]
    fn a_child_of_a_monitored_folder_overlaps() {
        let config = config(&["/a/dev"]);
        let (_, overlap) = config.overlapping_folder("/a/dev/work", None).unwrap();
        assert_eq!(overlap, FolderOverlap::Inside);
    }

    #[test]
    fn a_parent_of_a_monitored_folder_overlaps() {
        let config = config(&["/a/dev"]);
        let (_, overlap) = config.overlapping_folder("/a", None).unwrap();
        assert_eq!(overlap, FolderOverlap::Contains);
    }

    #[test]
    fn a_folder_does_not_overlap_itself_when_skipped() {
        let config = config(&["/a/dev"]);
        let id = config.folders[0].id.clone();
        assert!(config.overlapping_folder("/a/dev", Some(&id)).is_none());
    }

    #[test]
    fn skipping_one_folder_still_reports_another() {
        let config = config(&["/a/dev", "/b/work"]);
        let id = config.folders[0].id.clone();
        let (existing, _) = config.overlapping_folder("/b/work", Some(&id)).unwrap();
        assert_eq!(existing.path, "/b/work");
    }

    #[test]
    fn a_trailing_slash_is_still_the_same_folder() {
        let config = config(&["/a/dev"]);
        let (_, overlap) = config.overlapping_folder("/a/dev/", None).unwrap();
        assert_eq!(overlap, FolderOverlap::Same);
    }
}
