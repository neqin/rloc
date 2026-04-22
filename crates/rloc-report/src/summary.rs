use std::collections::BTreeMap;

use rloc_core::{MetricsSummary, ScanReport};

use crate::{GroupSummary, ScanGroupBy};

pub fn summary(report: &ScanReport) -> &MetricsSummary {
    &report.summary
}

pub fn groups_for(report: &ScanReport, group_by: ScanGroupBy) -> Vec<GroupSummary> {
    let mut grouped = BTreeMap::<String, MetricsSummary>::new();

    for file in &report.files {
        let key = match group_by {
            ScanGroupBy::Language => file.language.as_str().to_owned(),
            ScanGroupBy::Category => file.category.as_str().to_owned(),
            ScanGroupBy::Dir => file
                .path
                .parent()
                .map(|parent| {
                    if parent.as_str().is_empty() {
                        ".".to_owned()
                    } else {
                        parent.as_str().to_owned()
                    }
                })
                .unwrap_or_else(|| ".".to_owned()),
            ScanGroupBy::File => file.path.as_str().to_owned(),
        };

        grouped.entry(key).or_default().add_file(file);
    }

    let mut groups = grouped
        .into_iter()
        .map(|(key, metrics)| GroupSummary::from_metrics(group_by, key, &metrics))
        .collect::<Vec<_>>();
    sort_groups(&mut groups);
    groups
}

pub fn groups(report: &ScanReport, groupings: &[ScanGroupBy]) -> Vec<GroupSummary> {
    let mut result = Vec::new();

    for grouping in groupings {
        result.extend(groups_for(report, *grouping));
    }

    result
}

pub(crate) fn sort_groups(groups: &mut [GroupSummary]) {
    groups.sort_by(|left, right| {
        right
            .sloc
            .cmp(&left.sloc)
            .then_with(|| right.lines.cmp(&left.lines))
            .then_with(|| left.key.cmp(&right.key))
    });
}
