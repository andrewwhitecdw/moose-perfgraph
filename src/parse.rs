//! Parser for the performance sections of MOOSE console output.
//!
//! MOOSE prints three `VariadicTable`-formatted sections when timing is
//! enabled: `Performance Graph:` (the full call tree with Self/Total
//! columns), `Heaviest Branch:` (the single most expensive path), and
//! `Heaviest Sections:` (a flat Self-time ranking). The tables are ASCII
//! pipe-delimited with `-` border lines; section names are indented by
//! two spaces per depth level.

/// One row of a performance table.
#[derive(Debug, Clone, PartialEq)]
pub struct SectionRow {
    /// Section name with leading indentation removed.
    pub name: String,
    /// Tree depth from the original indentation (`name_indent / 2`).
    pub depth: usize,
    /// Number of calls (as printed).
    pub calls: f64,
    /// Self time in seconds.
    pub self_time: f64,
    /// Self time as a percentage of the run.
    pub self_pct: f64,
    /// Total time in seconds (0 for tables without a Total column).
    pub total_time: f64,
    /// Total time as a percentage of the run.
    pub total_pct: f64,
}

/// Everything extracted from one console output.
#[derive(Debug, Clone, Default)]
pub struct PerfData {
    /// Rows of the `Performance Graph:` table (empty when absent).
    pub graph: Vec<SectionRow>,
    /// Rows of the `Heaviest Branch:` table.
    pub heaviest_branch: Vec<SectionRow>,
    /// Rows of the `Heaviest Sections:` table.
    pub heaviest_sections: Vec<SectionRow>,
}

/// Known section headers and their table flavour.
const HEADERS: [(&str, TableKind); 3] = [
    ("Performance Graph:", TableKind::Tree),
    ("Heaviest Branch:", TableKind::Tree),
    ("Heaviest Sections:", TableKind::Flat),
];

#[derive(Debug, Clone, Copy, PartialEq)]
enum TableKind {
    /// 10-column tree table with Self and Total columns.
    Tree,
    /// 6-column flat ranking with only Self.
    Flat,
}

/// Parse console output text into [`PerfData`]. Missing sections yield
/// empty vectors; malformed table rows are skipped.
pub fn parse(text: &str) -> PerfData {
    let lines: Vec<&str> = text.lines().collect();
    let mut data = PerfData::default();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i].trim_end();
        if let Some((_, kind)) = HEADERS.iter().find(|(h, _)| line == *h) {
            let (rows, next) = parse_table(&lines[i + 1..], *kind);
            match line {
                "Performance Graph:" => data.graph = rows,
                "Heaviest Branch:" => data.heaviest_branch = rows,
                _ => data.heaviest_sections = rows,
            }
            i = i + 1 + next;
        } else {
            i += 1;
        }
    }
    data
}

/// Parse the table following a header. Returns the rows and the number of
/// lines consumed.
fn parse_table(lines: &[&str], kind: TableKind) -> (Vec<SectionRow>, usize) {
    let mut rows = Vec::new();
    let mut header_cols: Option<Vec<String>> = None;
    let mut consumed = 0;

    for line in lines {
        let t = line.trim_end();
        if t.is_empty() {
            break;
        }
        if t.starts_with('-') {
            consumed += 1;
            continue;
        }
        if !t.starts_with('|') {
            break;
        }
        let cells: Vec<String> = t
            .split('|')
            .skip(1)
            .map(|c| c.trim_end().to_string())
            .collect();
        consumed += 1;

        match &header_cols {
            None => {
                header_cols = Some(cells.clone());
            }
            Some(header) => {
                if let Some(row) = make_row(header, &cells, kind) {
                    rows.push(row);
                }
            }
        }
    }
    (rows, consumed)
}

fn make_row(header: &[String], cells: &[String], kind: TableKind) -> Option<SectionRow> {
    // Header cells may carry centering padding; data cells keep leading
    // indentation (it encodes tree depth). The flat table calls the
    // self-time column "Self(s)".
    let pos = |name: &str| {
        header
            .iter()
            .position(|h| h.trim() == name || (name == "Self" && h.trim() == "Self(s)"))
    };
    let num = |cells: &[String], i: usize| -> Option<f64> {
        cells
            .get(i)
            .map(|s| s.trim_end_matches('%').trim().to_string())
            .and_then(|s| s.parse::<f64>().ok())
    };
    // The tree table repeats "Avg."/"%" after both Self and Total; pick the
    // first "%" to the right of the corresponding time column.
    let pct_after = |after: Option<usize>| -> f64 {
        header
            .iter()
            .enumerate()
            .filter(|(_, h)| *h == "%")
            .map(|(i, _)| i)
            .find(|&i| Some(i) > after)
            .and_then(|i| num(cells, i))
            .unwrap_or(0.0)
    };

    let section = cells.first()?.trim_end().to_string();
    if section.is_empty() {
        return None;
    }
    let (name, depth) = split_indent(&section);

    let calls = num(cells, pos("Calls")?)?;
    let self_time = num(cells, pos("Self")?)?;
    let self_pct = pct_after(pos("Self"));
    let (total_time, total_pct) = match kind {
        TableKind::Tree => (
            num(cells, pos("Total")?).unwrap_or(0.0),
            pct_after(pos("Total")),
        ),
        TableKind::Flat => (0.0, 0.0),
    };
    Some(SectionRow {
        name,
        depth,
        calls,
        self_time,
        self_pct,
        total_time,
        total_pct,
    })
}

/// Split leading indentation from a section name; depth is indent/2.
fn split_indent(cell: &str) -> (String, usize) {
    let indent = cell.len() - cell.trim_start().len();
    (cell.trim_start().to_string(), indent / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = "\
Performance Graph:\n\
------------------------------------------------------------------------------------------------------------\n\
| Section            | Calls | Self      | Avg.  | %    | Memory | Total     | Avg.  | %    | Memory |\n\
------------------------------------------------------------------------------------------------------------\n\
| ROOT               | 1     | 0.001     | 0.001 | 0.0  | 0.0    | 3.100     | 3.100 | 100.0| 1.7    |\n\
------------------------------------------------------------------------------------------------------------\n\
Heaviest Sections:\n\
--------------------------------------------------------------------------------\n\
| Section            | Calls | Self(s)   | Avg.  | %    | Mem(MB) |\n\
--------------------------------------------------------------------------------\n\
| ROOT               | 1     | 0.001     | 0.001 | 0.1  | 0.0     |\n\
|   Solve            | 1     | 1.500     | 1.500 | 50.0 | 1.0     |\n\
|     Compute Jacobian | 10  | 0.800     | 0.080 | 26.7 | 0.5     |\n\
|     Compute Residual | 10  | 0.400     | 0.040 | 13.3 | 0.2     |\n\
--------------------------------------------------------------------------------\n\
";

    #[test]
    fn parses_heaviest_sections_table() {
        let data = parse(SAMPLE);
        assert_eq!(data.graph.len(), 1);
        assert_eq!(data.graph[0].name, "ROOT");
        assert_eq!(data.heaviest_sections.len(), 4);
        let solve = &data.heaviest_sections[1];
        assert_eq!(solve.name, "Solve");
        assert_eq!(solve.depth, 1);
        assert_eq!(solve.calls, 1.0);
        assert_eq!(solve.self_time, 1.5);
        let jac = &data.heaviest_sections[2];
        assert_eq!(jac.name, "Compute Jacobian");
        assert_eq!(jac.depth, 2);
        assert_eq!(jac.self_time, 0.8);
    }

    #[test]
    fn parses_tree_table_with_totals() {
        let sample = "\
Performance Graph:\n\
--------------------------------------------------------------------------------\n\
| Section            | Calls | Self      | Avg.  | %    | Memory | Total     | Avg.  | %    | Memory |\n\
--------------------------------------------------------------------------------\n\
| ROOT               | 1     | 0.001     | 0.001 | 0.1  | 0.0    | 3.000     | 3.000 | 100.0| 1.7    |\n\
|   Solve            | 1     | 1.500     | 1.500 | 50.0 | 1.0    | 2.900     | 2.900 | 96.7 | 1.6    |\n\
--------------------------------------------------------------------------------\n";
        let data = parse(sample);
        assert_eq!(data.graph.len(), 2);
        assert_eq!(data.graph[0].total_time, 3.0);
        assert_eq!(data.graph[1].total_time, 2.9);
        assert_eq!(data.graph[1].depth, 1);
    }

    #[test]
    fn empty_input_yields_empty_data() {
        let data = parse("no perf output here\n");
        assert!(data.graph.is_empty());
        assert!(data.heaviest_branch.is_empty());
        assert!(data.heaviest_sections.is_empty());
    }
}
