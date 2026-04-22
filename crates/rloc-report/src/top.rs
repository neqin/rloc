use rloc_core::ScanReport;

use crate::{GroupSummary, ScanGroupBy, TopFileEntry, summary};

pub fn top_files(report: &ScanReport, limit: usize) -> Vec<TopFileEntry> {
    let mut files = report
        .files
        .iter()
        .map(TopFileEntry::from)
        .collect::<Vec<_>>();
    files.sort_by(|left, right| {
        right
            .sloc
            .cmp(&left.sloc)
            .then_with(|| right.lines.cmp(&left.lines))
            .then_with(|| left.path.cmp(&right.path))
    });
    files.truncate(limit);
    files
}

pub fn top_dirs(report: &ScanReport, limit: usize) -> Vec<GroupSummary> {
    let dirs = summary::groups_for(report, ScanGroupBy::Dir);
    let mut selected: Vec<GroupSummary> = Vec::with_capacity(limit);

    for dir in dirs {
        if selected.len() >= limit {
            break;
        }

        if selected
            .iter()
            .any(|picked| directories_overlap(&picked.key, &dir.key))
        {
            continue;
        }

        selected.push(dir);
    }

    selected
}

fn directories_overlap(left: &str, right: &str) -> bool {
    left == right || is_ancestor_dir(left, right) || is_ancestor_dir(right, left)
}

fn is_ancestor_dir(ancestor: &str, descendant: &str) -> bool {
    if ancestor == "." || descendant == "." {
        return false;
    }

    let ancestor = ancestor.replace('\\', "/");
    let descendant = descendant.replace('\\', "/");

    descendant
        .strip_prefix(ancestor.as_str())
        .is_some_and(|suffix| suffix.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use super::directories_overlap;

    #[test]
    fn windows_separators_are_treated_as_directory_boundaries() {
        assert!(directories_overlap("src", "src\\nested"));
        assert!(directories_overlap("src\\nested", "src"));
    }
}
