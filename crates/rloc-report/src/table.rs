use std::{fmt::Write as _, io::IsTerminal};

use rloc_core::ScanReport;

use crate::{ScanRenderOptions, summary, top};

const MAX_GROUP_COL_WIDTH: usize = 28;

#[derive(Debug, Clone, Copy)]
enum Alignment {
    Left,
    Right,
}

#[derive(Debug, Clone, Copy)]
struct Column<'a> {
    header: &'a str,
    alignment: Alignment,
    max_width: Option<usize>,
}

#[derive(Debug, Clone, Copy)]
enum HeadingTone {
    Info,
    Warning,
}

impl<'a> Column<'a> {
    const fn left(header: &'a str, max_width: Option<usize>) -> Self {
        Self {
            header,
            alignment: Alignment::Left,
            max_width,
        }
    }

    const fn right(header: &'a str) -> Self {
        Self {
            header,
            alignment: Alignment::Right,
            max_width: None,
        }
    }
}

pub fn render(report: &ScanReport, options: &ScanRenderOptions) -> String {
    let mut output = String::new();
    let use_color = should_use_color();

    writeln!(
        &mut output,
        "{}",
        render_section_heading("Summary", HeadingTone::Info, use_color)
    )
    .expect("write to string");
    write_group_table(
        &mut output,
        std::iter::once(("summary", &report.summary)),
        use_color,
    );

    let category_groups = summary::groups_for(report, crate::ScanGroupBy::Category);
    if !category_groups.is_empty() {
        writeln!(&mut output).expect("write to string");
        writeln!(
            &mut output,
            "{}",
            render_section_heading("Category totals", HeadingTone::Info, use_color)
        )
        .expect("write to string");
        write_category_totals(&mut output, &category_groups, use_color);
    }

    for grouping in &options.group_by {
        let groups = summary::groups_for(report, *grouping);
        if groups.is_empty() {
            continue;
        }

        writeln!(&mut output).expect("write to string");
        writeln!(
            &mut output,
            "{}",
            render_section_heading(
                &format!("Groups by {}", grouping.as_str()),
                HeadingTone::Info,
                use_color
            )
        )
        .expect("write to string");

        let rows = groups
            .into_iter()
            .map(|group| {
                vec![
                    group.key,
                    group.files.to_string(),
                    group.lines.to_string(),
                    group.code.to_string(),
                    group.mixed.to_string(),
                    group.comment.to_string(),
                    group.doc.to_string(),
                    group.blank.to_string(),
                    group.bytes.to_string(),
                    group.sloc.to_string(),
                ]
            })
            .collect::<Vec<_>>();
        write_table(
            &mut output,
            &group_summary_columns("group"),
            &rows,
            use_color,
        );
    }

    if let Some(limit) = options.top_files {
        let files = top::top_files(report, limit);
        if !files.is_empty() {
            writeln!(&mut output).expect("write to string");
            writeln!(
                &mut output,
                "{}",
                render_section_heading("Top files", HeadingTone::Info, use_color)
            )
            .expect("write to string");

            let rows = files
                .into_iter()
                .map(|file| {
                    vec![
                        file.language.as_str().to_owned(),
                        file.category.as_str().to_owned(),
                        file.lines.to_string(),
                        file.code.to_string(),
                        file.mixed.to_string(),
                        file.comment.to_string(),
                        file.doc.to_string(),
                        file.blank.to_string(),
                        file.bytes.to_string(),
                        file.sloc.to_string(),
                        file.path.to_string(),
                    ]
                })
                .collect::<Vec<_>>();
            write_table(&mut output, &top_file_columns(), &rows, use_color);
        }
    }

    if let Some(limit) = options.top_dirs {
        let dirs = top::top_dirs(report, limit);
        if !dirs.is_empty() {
            writeln!(&mut output).expect("write to string");
            writeln!(
                &mut output,
                "{}",
                render_section_heading("Top dirs", HeadingTone::Info, use_color)
            )
            .expect("write to string");

            let rows = dirs
                .into_iter()
                .map(|dir| {
                    vec![
                        dir.files.to_string(),
                        dir.lines.to_string(),
                        dir.code.to_string(),
                        dir.mixed.to_string(),
                        dir.comment.to_string(),
                        dir.doc.to_string(),
                        dir.blank.to_string(),
                        dir.bytes.to_string(),
                        dir.sloc.to_string(),
                        dir.key,
                    ]
                })
                .collect::<Vec<_>>();
            write_table(&mut output, &top_dir_columns(), &rows, use_color);
        }
    }

    if !report.warnings.is_empty() {
        writeln!(&mut output).expect("write to string");
        writeln!(
            &mut output,
            "{}",
            render_section_heading("Warnings", HeadingTone::Warning, use_color)
        )
        .expect("write to string");
        for warning in &report.warnings {
            writeln!(&mut output, "- {}", crate::format_warning(warning)).expect("write to string");
        }
    }

    output.trim_end().to_owned()
}

fn write_group_table<'a>(
    output: &mut String,
    rows: impl IntoIterator<Item = (&'a str, &'a rloc_core::MetricsSummary)>,
    use_color: bool,
) {
    let rows = rows
        .into_iter()
        .map(|(group, summary)| {
            vec![
                group.to_owned(),
                summary.files.to_string(),
                summary.lines.to_string(),
                summary.code.to_string(),
                summary.mixed.to_string(),
                summary.comment.to_string(),
                summary.doc.to_string(),
                summary.blank.to_string(),
                summary.bytes.to_string(),
                summary.sloc.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    write_table(output, &group_summary_columns("group"), &rows, use_color);
}

fn write_category_totals(output: &mut String, rows: &[crate::GroupSummary], use_color: bool) {
    let rows = rows
        .iter()
        .map(|row| {
            vec![
                row.key.clone(),
                row.files.to_string(),
                row.lines.to_string(),
                row.sloc.to_string(),
            ]
        })
        .collect::<Vec<_>>();
    let columns = [
        Column::left("category", Some(MAX_GROUP_COL_WIDTH)),
        Column::right("files"),
        Column::right("lines"),
        Column::right("sloc"),
    ];
    write_table(output, &columns, &rows, use_color);
}

fn group_summary_columns<'a>(first_header: &'a str) -> [Column<'a>; 10] {
    [
        Column::left(first_header, Some(MAX_GROUP_COL_WIDTH)),
        Column::right("files"),
        Column::right("lines"),
        Column::right("code"),
        Column::right("mixed"),
        Column::right("comment"),
        Column::right("doc"),
        Column::right("blank"),
        Column::right("bytes"),
        Column::right("sloc"),
    ]
}

fn top_file_columns() -> [Column<'static>; 11] {
    [
        Column::left("language", None),
        Column::left("category", None),
        Column::right("lines"),
        Column::right("code"),
        Column::right("mixed"),
        Column::right("comment"),
        Column::right("doc"),
        Column::right("blank"),
        Column::right("bytes"),
        Column::right("sloc"),
        Column::left("path", None),
    ]
}

fn top_dir_columns() -> [Column<'static>; 10] {
    [
        Column::right("files"),
        Column::right("lines"),
        Column::right("code"),
        Column::right("mixed"),
        Column::right("comment"),
        Column::right("doc"),
        Column::right("blank"),
        Column::right("bytes"),
        Column::right("sloc"),
        Column::left("group", None),
    ]
}

fn write_table(output: &mut String, columns: &[Column<'_>], rows: &[Vec<String>], use_color: bool) {
    let widths = column_widths(columns, rows);
    writeln!(
        output,
        "{}",
        decorate_table_header(&format_header_row(columns, &widths), use_color)
    )
    .expect("write to string");

    for row in rows {
        let values = row.iter().map(String::as_str).collect::<Vec<_>>();
        writeln!(output, "{}", format_row(columns, &widths, &values)).expect("write to string");
    }
}

fn column_widths(columns: &[Column<'_>], rows: &[Vec<String>]) -> Vec<usize> {
    columns
        .iter()
        .enumerate()
        .map(|(index, column)| {
            let header_width = column.header.chars().count();
            let data_width = rows
                .iter()
                .filter_map(|row| row.get(index))
                .map(|value| fitted_width(value, column.max_width))
                .max()
                .unwrap_or(0);
            header_width.max(data_width)
        })
        .collect()
}

fn fitted_width(value: &str, max_width: Option<usize>) -> usize {
    let width = value.chars().count();
    max_width.map(|limit| width.min(limit)).unwrap_or(width)
}

fn format_row(columns: &[Column<'_>], widths: &[usize], values: &[&str]) -> String {
    columns
        .iter()
        .zip(widths.iter().copied())
        .zip(values.iter().copied())
        .map(|((column, width), value)| format_cell(value, width, *column))
        .collect::<Vec<_>>()
        .join("  ")
}

fn format_header_row(columns: &[Column<'_>], widths: &[usize]) -> String {
    columns
        .iter()
        .zip(widths.iter().copied())
        .map(|(column, width)| format!("{:<width$}", column.header))
        .collect::<Vec<_>>()
        .join("  ")
}

fn format_cell(value: &str, width: usize, column: Column<'_>) -> String {
    let fitted = fit_cell(value, width, column.max_width);
    match column.alignment {
        Alignment::Left => format!("{fitted:<width$}"),
        Alignment::Right => format!("{fitted:>width$}"),
    }
}

fn fit_cell(value: &str, width: usize, max_width: Option<usize>) -> String {
    let target_width = max_width.map(|limit| width.min(limit)).unwrap_or(width);
    truncate_middle(value, target_width)
}

fn truncate_middle(value: &str, width: usize) -> String {
    let value_width = value.chars().count();
    if value_width <= width {
        return value.to_owned();
    }

    if width <= 3 {
        return ".".repeat(width);
    }

    let available = width - 3;
    let suffix_width = (available * 2) / 3;
    let prefix_width = available - suffix_width;
    let prefix = value.chars().take(prefix_width).collect::<String>();
    let suffix = value
        .chars()
        .rev()
        .take(suffix_width)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<String>();
    format!("{prefix}...{suffix}")
}

fn should_use_color() -> bool {
    std::env::var_os("NO_COLOR").is_none()
        && std::env::var_os("TERM").is_none_or(|term| term != "dumb")
        && std::io::stdout().is_terminal()
}

fn render_section_heading(title: &str, tone: HeadingTone, use_color: bool) -> String {
    if !use_color {
        return title.to_owned();
    }

    let code = match tone {
        HeadingTone::Info => "1;36",
        HeadingTone::Warning => "1;33",
    };
    format!("\u{1b}[{code}m{title}\u{1b}[0m")
}

fn decorate_table_header(row: &str, use_color: bool) -> String {
    if use_color {
        format!("\u{1b}[2m{row}\u{1b}[0m")
    } else {
        row.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::{HeadingTone, decorate_table_header, render_section_heading};

    #[test]
    fn section_headings_are_colored_when_enabled() {
        assert_eq!(
            render_section_heading("Groups by language", HeadingTone::Info, true),
            "\u{1b}[1;36mGroups by language\u{1b}[0m"
        );
        assert_eq!(
            render_section_heading("Warnings", HeadingTone::Warning, true),
            "\u{1b}[1;33mWarnings\u{1b}[0m"
        );
    }

    #[test]
    fn header_rows_are_dimmed_when_enabled() {
        assert_eq!(
            decorate_table_header("group  files", true),
            "\u{1b}[2mgroup  files\u{1b}[0m"
        );
        assert_eq!(decorate_table_header("group  files", false), "group  files");
    }
}
