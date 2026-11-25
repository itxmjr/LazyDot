use std::path::PathBuf;

/// What is sitting at a home path that a repo file wants to occupy.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Occupant {
    File,
    Directory,
    /// A symlink pointing somewhere other than our repo copy.
    Symlink(PathBuf),
}

/// The state of a single link unit (one file or folded directory).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EntryState {
    /// The home path is a symlink to the repo copy.
    Linked,
    /// The repo copy exists but nothing is at the home path.
    NotLinked,
    /// The repo copy exists but something else occupies the home path.
    Conflict(Occupant),
    /// The file lives only in $HOME and has not been adopted yet.
    Untracked,
    /// The file lives only in $HOME but is a symlink managed by some other tool.
    ForeignLink(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Entry {
    /// Location in $HOME.
    pub target: PathBuf,
    /// Location inside the dotfiles repo (None for untracked entries).
    pub source: Option<PathBuf>,
    pub state: EntryState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PackageStatus {
    /// Not in the repo yet; can be adopted.
    Untracked,
    /// Every entry is linked.
    Stowed,
    /// In the repo, nothing linked.
    Unstowed,
    /// Some entries linked, some not.
    Partial,
    /// At least one home path is occupied by something else.
    Conflict,
}

impl PackageStatus {
    pub fn label(self) -> &'static str {
        match self {
            PackageStatus::Untracked => "untracked",
            PackageStatus::Stowed => "stowed",
            PackageStatus::Unstowed => "unstowed",
            PackageStatus::Partial => "partially stowed",
            PackageStatus::Conflict => "conflict",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Package {
    pub name: String,
    /// True when the package exists in the dotfiles repo.
    pub tracked: bool,
    pub entries: Vec<Entry>,
}

impl Package {
    pub fn status(&self) -> PackageStatus {
        if !self.tracked {
            return PackageStatus::Untracked;
        }
        if self
            .entries
            .iter()
            .any(|e| matches!(e.state, EntryState::Conflict(_)))
        {
            return PackageStatus::Conflict;
        }
        let linked = self
            .entries
            .iter()
            .filter(|e| e.state == EntryState::Linked)
            .count();
        match linked {
            0 => PackageStatus::Unstowed,
            n if n == self.entries.len() => PackageStatus::Stowed,
            _ => PackageStatus::Partial,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(state: EntryState) -> Entry {
        Entry {
            target: "/h/x".into(),
            source: Some("/r/x".into()),
            state,
        }
    }

    #[test]
    fn status_aggregation() {
        let mut p = Package {
            name: "x".into(),
            tracked: true,
            entries: vec![],
        };
        assert_eq!(p.status(), PackageStatus::Unstowed);
        p.entries = vec![entry(EntryState::Linked), entry(EntryState::Linked)];
        assert_eq!(p.status(), PackageStatus::Stowed);
        p.entries.push(entry(EntryState::NotLinked));
        assert_eq!(p.status(), PackageStatus::Partial);
        p.entries.push(entry(EntryState::Conflict(Occupant::File)));
        assert_eq!(p.status(), PackageStatus::Conflict);
        p.tracked = false;
        assert_eq!(p.status(), PackageStatus::Untracked);
    }
}
