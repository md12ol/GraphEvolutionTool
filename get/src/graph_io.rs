//! Reading graphs from edge-list files.
//!
//! One edge per line, `start,end,weight`, comma-delimited, any line ending. A
//! caller whose data is not 0-indexed passes `min_node_index`, and every index
//! is shifted to 0 here — once, on the way in.
//!
//! **Everything is validated before anything is built.** The whole text is
//! checked first and the edge list is returned only if every row survives, so a
//! rejected file leaves no half-built graph behind. That matters because
//! [`Graph::set_edge`](crate::graph::Graph::set_edge) absorbs the failures this
//! module reports: it returns early on a bad endpoint or a self-loop, and clamps
//! an over-cap weight. A graph built first and checked after would already have
//! lost the offending edge, leaving nothing to name.
//!
//! Two failures are warnings rather than errors, because the file still
//! describes a graph: a repeated edge (the last occurrence wins) and a
//! zero-weight edge (kept as given, which is no edge at all). Warnings are
//! returned to the caller rather than printed here, which keeps this module
//! testable without a Python interpreter: the Python boundary is what raises
//! them, through `warnings`. There is no consumer on the Rust side — the
//! `get-run` binary has no loader, so nothing there can produce one.

use std::path::Path;

use crate::graph::Graph;

/// A validated edge list, plus whatever was worth saying about it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EdgeFile {
    /// Every surviving edge as `(u, v, multiplicity)`, already shifted to
    /// 0-indexing and with duplicates collapsed. First-appearance order.
    pub edges: Vec<(usize, usize, u32)>,
    /// Non-fatal findings, in the order they were met.
    pub warnings: Vec<LoadWarning>,
    /// What to call this in a message — a path, or a test's own label.
    pub source: String,
}

impl EdgeFile {
    /// Build the graph these edges describe, sizing it from the data itself.
    ///
    /// # Why the node count is inferred rather than passed
    ///
    /// [`load_edge_folder`] takes **one** `num_nodes` for the whole folder,
    /// which suits a set of same-sized graphs and does not suit a reference
    /// set: those come from real data and differ in size, and the loader's
    /// `num_nodes` is an upper bound used to reject out-of-range indices, not
    /// a description of any one file. So a caller passes a generous cap to the
    /// loader — which still catches a wild index — and gets each graph's real
    /// size from here.
    ///
    /// **The count is `highest index + 1`, so a trailing isolated node cannot
    /// be seen.** Nothing in the file distinguishes "node 9 exists but has no
    /// edges" from "there is no node 9". A file whose graph genuinely has one
    /// is one node short, silently. Where the count is known from elsewhere —
    /// TUDataset's `graph_indicator` file gives it exactly — prefer that and
    /// use this as a check.
    pub fn to_graph(&self, max_edge_multiplicity: u32) -> Graph {
        // Indices are 0-based by the time they reach here, so a graph with no
        // edges at all is the only one with no nodes.
        let mut num_nodes = 0;
        for &(u, v, _) in &self.edges {
            if u + 1 > num_nodes {
                num_nodes = u + 1;
            }
            if v + 1 > num_nodes {
                num_nodes = v + 1;
            }
        }

        let mut graph = Graph::new(num_nodes, max_edge_multiplicity);
        graph.set_edges(&self.edges);
        graph
    }
}

/// Why one row was rejected.
///
/// Kept apart from its prose for the same reason `ConfigError::Validation` is: a
/// test asserts on the variant instead of matching wording that changes every
/// time someone improves the message. Not linked, because `config` is
/// crate-internal and this type is public — a link would render as a dead one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RowProblem {
    /// Not three comma-separated fields. Carries how many there were.
    ColumnCount(usize),
    /// A field that is not an integer. Carries which one — `start`, `end` or
    /// `weight`.
    NonNumeric(&'static str),
    /// `start == end`. This graph has no representation for a self-loop, and in
    /// caller data it usually means the indices are wrong.
    SelfLoop(i64),
    /// A node index outside `min_node_index ..= min_node_index + num_nodes - 1`.
    /// Carries the index and the range it missed.
    NodeOutOfRange { index: i64, low: i64, high: i64 },
    /// A negative weight. Carries it.
    NegativeWeight(i64),
    /// A weight above the configured `max_edge_multiplicity`. Carries the weight
    /// and the cap.
    WeightAboveCap { weight: i64, cap: u32 },
}

impl std::fmt::Display for RowProblem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            RowProblem::ColumnCount(found) => write!(
                f,
                "expected 3 comma-separated fields, `start,end,weight`, but found {found}"
            ),
            RowProblem::NonNumeric(field) => {
                write!(f, "the `{field}` field is not a whole number")
            }
            RowProblem::SelfLoop(node) => write!(
                f,
                "({node}, {node}) is a self-loop, which this graph cannot represent; \
                 a self-loop in caller data usually means the indices are wrong, so it \
                 is reported rather than dropped"
            ),
            RowProblem::NodeOutOfRange { index, low, high } => write!(
                f,
                "node {index} is outside {low}..={high}; an out-of-range edge would be \
                 dropped without a word, so it is rejected instead"
            ),
            RowProblem::NegativeWeight(weight) => write!(
                f,
                "weight {weight} is negative, and an edge multiplicity counts copies of \
                 an edge"
            ),
            RowProblem::WeightAboveCap { weight, cap } => write!(
                f,
                "weight {weight} is above this config's max_edge_multiplicity of {cap}; \
                 raise the cap or lower the weight rather than having it silently clamped"
            ),
        }
    }
}

/// Why a file could not be turned into a graph.
#[derive(Debug)]
pub enum GraphLoadError {
    /// The file or folder could not be read.
    Io {
        /// What was being read.
        path: String,
        /// What the operating system said.
        source: std::io::Error,
    },
    /// One row did not survive validation.
    Row {
        /// The file the row is in.
        path: String,
        /// Its 1-based line number, so the user can go straight to it.
        line: usize,
        /// What was wrong with it.
        problem: RowProblem,
    },
}

impl std::fmt::Display for GraphLoadError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GraphLoadError::Io { path, source } => {
                write!(f, "could not read `{path}`: {source}")
            }
            GraphLoadError::Row {
                path,
                line,
                problem,
            } => write!(f, "`{path}`, line {line}: {problem}"),
        }
    }
}

impl std::error::Error for GraphLoadError {}

/// Something worth telling the caller that does not stop the load.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadWarning {
    /// The same pair appeared more than once. Comparison is canonical —
    /// `(min(u, v), max(u, v))` — so `2,5` and `5,2` collide, and the last
    /// occurrence is the one kept.
    DuplicateEdge {
        /// The canonical pair, in the caller's own indexing.
        edge: (usize, usize),
        /// The multiplicity that survived.
        kept: u32,
        /// Where the winning occurrence was, when it came from a file.
        line: Option<usize>,
    },
    /// A zero-weight edge, which is no edge at all. Kept as given.
    ZeroWeight {
        /// The pair, in the caller's own indexing.
        edge: (usize, usize),
        /// Where it was, when it came from a file.
        line: Option<usize>,
    },
    /// The file held no edges. The graph is empty.
    EmptyFile,
}

impl std::fmt::Display for LoadWarning {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LoadWarning::DuplicateEdge {
                edge: (u, v),
                kept,
                line,
            } => {
                write!(f, "edge ({u}, {v}) appears more than once")?;
                if let Some(line) = line {
                    write!(f, ", last at line {line}")?;
                }
                write!(f, "; kept with multiplicity {kept}")
            }
            LoadWarning::ZeroWeight { edge: (u, v), line } => {
                write!(f, "edge ({u}, {v})")?;
                if let Some(line) = line {
                    write!(f, " at line {line}")?;
                }
                write!(f, " has weight 0, which is no edge at all; kept as given")
            }
            LoadWarning::EmptyFile => write!(f, "holds no edges; the graph will be empty"),
        }
    }
}

/// One validated edge together with where it came from.
///
/// The line is `None` for edges handed over by a setter call, which have no
/// file behind them. It exists so duplicate detection can be written once and
/// used by both routes.
#[derive(Debug, Clone, Copy)]
pub(crate) struct SourcedEdge {
    pub(crate) u: usize,
    pub(crate) v: usize,
    pub(crate) weight: u32,
    pub(crate) line: Option<usize>,
}

/// Collapse repeated pairs, last occurrence winning, and report each collapse.
///
/// Pairs are compared canonically, so `(2, 5)` and `(5, 2)` are the same edge —
/// an undirected edge written either way round must not slip through as two.
/// Surviving edges keep first-appearance order, which makes the result a
/// function of the input alone rather than of a hash order.
///
/// `shift` is subtracted from nothing here; it is only what the reported pairs
/// are shifted *back* by, so a warning names the indices the caller wrote
/// rather than the internal 0-based ones.
pub(crate) fn canonicalize(
    edges: &[SourcedEdge],
    shift: i64,
) -> (Vec<(usize, usize, u32)>, Vec<LoadWarning>) {
    let mut kept: Vec<(usize, usize, u32)> = Vec::with_capacity(edges.len());
    let mut warnings = Vec::new();

    for edge in edges {
        let pair = (edge.u.min(edge.v), edge.u.max(edge.v));

        // Linear scan rather than a map: it keeps first-appearance order without
        // a second structure, and an edge list is small enough that the cost
        // never shows.
        let mut existing = None;
        for (index, &(u, v, _)) in kept.iter().enumerate() {
            if (u, v) == pair {
                existing = Some(index);
                break;
            }
        }

        match existing {
            Some(index) => {
                kept[index].2 = edge.weight;
                warnings.push(LoadWarning::DuplicateEdge {
                    edge: unshift(pair, shift),
                    kept: edge.weight,
                    line: edge.line,
                });
            }
            None => kept.push((pair.0, pair.1, edge.weight)),
        }
    }

    (kept, warnings)
}

/// Put a canonical pair back into the caller's indexing, for a message.
fn unshift(pair: (usize, usize), shift: i64) -> (usize, usize) {
    let restore = |node: usize| -> usize {
        let raw = node as i64 + shift;
        // A negative result cannot happen for an index that passed the range
        // check, and saturating beats panicking inside a warning either way.
        raw.max(0) as usize
    };
    (restore(pair.0), restore(pair.1))
}

/// Parse and validate an edge list held in memory.
///
/// `source` is what a message calls this text. `min_node_index` is the index the
/// caller's own data starts at; every index is shifted by it, so the returned
/// edges are 0-based whatever went in.
///
/// Blank lines are skipped rather than rejected — every text file ends in one.
pub fn parse_edge_list(
    text: &str,
    source: &str,
    num_nodes: usize,
    max_edge_multiplicity: u32,
    min_node_index: i64,
) -> Result<EdgeFile, GraphLoadError> {
    // An empty graph has no valid index at all, and this keeps the arithmetic
    // below from underflowing into a range that accepts one.
    let highest = min_node_index + num_nodes as i64 - 1;

    let mut parsed: Vec<SourcedEdge> = Vec::new();
    let mut warnings = Vec::new();

    for (index, raw_line) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let fields: Vec<&str> = trimmed.split(',').collect();
        if fields.len() != 3 {
            return Err(GraphLoadError::Row {
                path: source.to_string(),
                line,
                problem: RowProblem::ColumnCount(fields.len()),
            });
        }

        // Parsed as signed, deliberately: a negative index or weight is a
        // distinct, reportable mistake, and reading into `usize` would collapse
        // it into "not a number".
        let start = parse_field(fields[0], "start", source, line)?;
        let end = parse_field(fields[1], "end", source, line)?;
        let weight = parse_field(fields[2], "weight", source, line)?;

        if start == end {
            return Err(GraphLoadError::Row {
                path: source.to_string(),
                line,
                problem: RowProblem::SelfLoop(start),
            });
        }

        for node in [start, end] {
            if node < min_node_index || node > highest {
                return Err(GraphLoadError::Row {
                    path: source.to_string(),
                    line,
                    problem: RowProblem::NodeOutOfRange {
                        index: node,
                        low: min_node_index,
                        high: highest,
                    },
                });
            }
        }

        if weight < 0 {
            return Err(GraphLoadError::Row {
                path: source.to_string(),
                line,
                problem: RowProblem::NegativeWeight(weight),
            });
        }

        if weight > max_edge_multiplicity as i64 {
            return Err(GraphLoadError::Row {
                path: source.to_string(),
                line,
                problem: RowProblem::WeightAboveCap {
                    weight,
                    cap: max_edge_multiplicity,
                },
            });
        }

        if weight == 0 {
            warnings.push(LoadWarning::ZeroWeight {
                edge: (start as usize, end as usize),
                line: Some(line),
            });
        }

        parsed.push(SourcedEdge {
            u: (start - min_node_index) as usize,
            v: (end - min_node_index) as usize,
            weight: weight as u32,
            line: Some(line),
        });
    }

    if parsed.is_empty() {
        warnings.push(LoadWarning::EmptyFile);
    }

    let (edges, duplicates) = canonicalize(&parsed, min_node_index);
    warnings.extend(duplicates);

    Ok(EdgeFile {
        edges,
        warnings,
        source: source.to_string(),
    })
}

/// Read one field as a whole number, naming it if it is not one.
fn parse_field(
    field: &str,
    name: &'static str,
    source: &str,
    line: usize,
) -> Result<i64, GraphLoadError> {
    match field.trim().parse::<i64>() {
        Ok(value) => Ok(value),
        Err(_) => Err(GraphLoadError::Row {
            path: source.to_string(),
            line,
            problem: RowProblem::NonNumeric(name),
        }),
    }
}

/// Read and validate one edge-list file.
pub fn load_edge_file(
    path: &Path,
    num_nodes: usize,
    max_edge_multiplicity: u32,
    min_node_index: i64,
) -> Result<EdgeFile, GraphLoadError> {
    let source = path.display().to_string();
    let text = std::fs::read_to_string(path).map_err(|error| GraphLoadError::Io {
        path: source.clone(),
        source: error,
    })?;

    parse_edge_list(
        &text,
        &source,
        num_nodes,
        max_edge_multiplicity,
        min_node_index,
    )
}

/// Read every edge-list file in a folder, one file per graph.
///
/// **The order is sorted by file name, and each file's name is on the result it
/// produced.** Filesystem order is not reproducible across machines, and a
/// reference set is consumed positionally, so leaving the order to the
/// directory would make a run's numbers depend on how its data happened to be
/// written to disk.
///
/// Sub-directories are skipped; every regular file is read, since an extension
/// convention would silently drop data the caller meant to include.
pub fn load_edge_folder(
    folder: &Path,
    num_nodes: usize,
    max_edge_multiplicity: u32,
    min_node_index: i64,
) -> Result<Vec<EdgeFile>, GraphLoadError> {
    let folder_name = folder.display().to_string();

    let entries = std::fs::read_dir(folder).map_err(|error| GraphLoadError::Io {
        path: folder_name.clone(),
        source: error,
    })?;

    let mut paths = Vec::new();
    for entry in entries {
        let entry = entry.map_err(|error| GraphLoadError::Io {
            path: folder_name.clone(),
            source: error,
        })?;

        let path = entry.path();
        if path.is_file() {
            paths.push(path);
        }
    }

    paths.sort();

    let mut loaded = Vec::with_capacity(paths.len());
    for path in &paths {
        loaded.push(load_edge_file(
            path,
            num_nodes,
            max_edge_multiplicity,
            min_node_index,
        )?);
    }

    Ok(loaded)
}

#[cfg(test)]
mod tests {
    use super::{
        EdgeFile, GraphLoadError, LoadWarning, RowProblem, load_edge_file, load_edge_folder,
        parse_edge_list,
    };
    use std::io::Write;

    /// Parse with the settings most tests want: 10 nodes, cap 3, 0-indexed.
    fn parse(text: &str) -> Result<EdgeFile, GraphLoadError> {
        parse_edge_list(text, "test", 10, 3, 0)
    }

    /// The `problem` of a rejection, or a panic naming what came back instead.
    fn problem(result: Result<EdgeFile, GraphLoadError>) -> (usize, RowProblem) {
        match result {
            Err(GraphLoadError::Row { line, problem, .. }) => (line, problem),
            Err(other) => panic!("expected a rejected row, got {other}"),
            Ok(file) => panic!("expected a rejection, got {} edges", file.edges.len()),
        }
    }

    #[test]
    fn a_well_formed_file_loads_with_no_warnings() {
        let loaded = parse("0,1,1\n1,2,3\n").expect("valid");

        assert_eq!(loaded.edges, vec![(0, 1, 1), (1, 2, 3)]);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);
    }

    #[test]
    fn blank_lines_and_spacing_are_tolerated() {
        // Trailing newlines, a blank separator line, padded fields and \r\n all
        // arrive from real editors; none of them is a malformed row.
        let loaded = parse(" 0 , 1 , 2 \n\n1,2,1\r\n").expect("valid");

        assert_eq!(loaded.edges, vec![(0, 1, 2), (1, 2, 1)]);
    }

    #[test]
    fn a_self_loop_is_rejected_and_names_its_line() {
        assert_eq!(
            problem(parse("0,1,1\n4,4,1\n")),
            (2, RowProblem::SelfLoop(4))
        );
    }

    #[test]
    fn a_row_with_the_wrong_column_count_is_rejected() {
        assert_eq!(problem(parse("0,1\n")), (1, RowProblem::ColumnCount(2)));
        assert_eq!(problem(parse("0,1,1,1\n")), (1, RowProblem::ColumnCount(4)));
    }

    #[test]
    fn a_non_numeric_field_is_rejected_and_names_which() {
        assert_eq!(
            problem(parse("a,1,1\n")),
            (1, RowProblem::NonNumeric("start"))
        );
        assert_eq!(
            problem(parse("0,b,1\n")),
            (1, RowProblem::NonNumeric("end"))
        );
        assert_eq!(
            problem(parse("0,1,heavy\n")),
            (1, RowProblem::NonNumeric("weight"))
        );
    }

    #[test]
    fn an_out_of_range_node_is_rejected_rather_than_dropped() {
        // The whole point of the check: `Graph::set_edge` would discard this
        // edge in silence, so the file must not be allowed to build one.
        assert_eq!(
            problem(parse("0,10,1\n")),
            (
                1,
                RowProblem::NodeOutOfRange {
                    index: 10,
                    low: 0,
                    high: 9
                }
            )
        );
    }

    #[test]
    fn a_negative_weight_is_rejected_rather_than_read_as_unparseable() {
        assert_eq!(
            problem(parse("0,1,-2\n")),
            (1, RowProblem::NegativeWeight(-2))
        );
    }

    #[test]
    fn a_negative_node_index_reports_the_range_it_missed() {
        assert_eq!(
            problem(parse("-1,1,1\n")),
            (
                1,
                RowProblem::NodeOutOfRange {
                    index: -1,
                    low: 0,
                    high: 9
                }
            )
        );
    }

    #[test]
    fn a_weight_above_the_cap_is_rejected_rather_than_clamped() {
        assert_eq!(
            problem(parse("0,1,4\n")),
            (1, RowProblem::WeightAboveCap { weight: 4, cap: 3 })
        );
    }

    #[test]
    fn a_repeated_edge_overwrites_with_a_warning() {
        let loaded = parse("0,1,1\n2,3,1\n0,1,3\n").expect("valid");

        // Last occurrence wins, and it keeps the position of the first, so the
        // result does not depend on iteration order anywhere.
        assert_eq!(loaded.edges, vec![(0, 1, 3), (2, 3, 1)]);
        assert_eq!(
            loaded.warnings,
            vec![LoadWarning::DuplicateEdge {
                edge: (0, 1),
                kept: 3,
                line: Some(3),
            }]
        );
    }

    #[test]
    fn a_reversed_repeat_is_the_same_edge() {
        // `5,2` after `2,5` is one edge written twice, not two edges.
        let loaded = parse("2,5,1\n5,2,2\n").expect("valid");

        assert_eq!(loaded.edges, vec![(2, 5, 2)]);
        assert_eq!(loaded.warnings.len(), 1, "{:?}", loaded.warnings);
    }

    #[test]
    fn a_zero_weight_edge_warns_and_is_kept() {
        let loaded = parse("0,1,0\n").expect("valid");

        assert_eq!(loaded.edges, vec![(0, 1, 0)]);
        assert_eq!(
            loaded.warnings,
            vec![LoadWarning::ZeroWeight {
                edge: (0, 1),
                line: Some(1),
            }]
        );
    }

    #[test]
    fn an_empty_file_warns_and_yields_no_edges() {
        let loaded = parse("\n\n").expect("valid");

        assert!(loaded.edges.is_empty());
        assert_eq!(loaded.warnings, vec![LoadWarning::EmptyFile]);
    }

    #[test]
    fn min_node_index_shifts_every_index_to_zero() {
        let loaded = parse_edge_list("1,2,1\n2,3,2\n", "test", 3, 3, 1).expect("valid");

        assert_eq!(loaded.edges, vec![(0, 1, 1), (1, 2, 2)]);
    }

    #[test]
    fn min_node_index_moves_the_accepted_range_rather_than_widening_it() {
        // 0 is below a 1-indexed file's own range: shifting is not the same as
        // accepting both indexings.
        assert_eq!(
            problem(parse_edge_list("0,1,1\n", "test", 3, 3, 1)),
            (
                1,
                RowProblem::NodeOutOfRange {
                    index: 0,
                    low: 1,
                    high: 3
                }
            )
        );
    }

    #[test]
    fn a_warning_names_the_indices_the_caller_wrote() {
        // The user reads their own file, not our 0-based copy of it.
        let loaded = parse_edge_list("1,2,1\n2,1,3\n", "test", 3, 3, 1).expect("valid");

        assert_eq!(
            loaded.warnings,
            vec![LoadWarning::DuplicateEdge {
                edge: (1, 2),
                kept: 3,
                line: Some(2),
            }]
        );
    }

    #[test]
    fn a_missing_file_is_a_loud_error() {
        let missing = std::path::Path::new("no_such_directory_here/no_such_file.csv");

        match load_edge_file(missing, 10, 3, 0) {
            Err(GraphLoadError::Io { path, .. }) => assert!(path.contains("no_such_file")),
            other => panic!("expected an Io error, got {other:?}"),
        }
    }

    /// Write `files` into a fresh temporary folder and hand back its path.
    fn folder_of(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("get_graph_io_{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp folder");

        for (file_name, text) in files {
            let mut file = std::fs::File::create(folder.join(file_name)).expect("temp file");
            file.write_all(text.as_bytes()).expect("write");
        }

        folder
    }

    #[test]
    fn a_folder_is_read_in_sorted_order_and_says_which_file_is_which() {
        let folder = folder_of(
            "sorted",
            &[
                ("c.csv", "0,1,1\n"),
                ("a.csv", "1,2,1\n"),
                ("b.csv", "2,3,1\n"),
            ],
        );

        let loaded = load_edge_folder(&folder, 10, 3, 0).expect("valid");

        let names: Vec<String> = loaded
            .iter()
            .map(|file| {
                std::path::Path::new(&file.source)
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned()
            })
            .collect();
        assert_eq!(names, vec!["a.csv", "b.csv", "c.csv"]);
        assert_eq!(loaded[0].edges, vec![(1, 2, 1)]);

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn one_bad_file_rejects_the_whole_folder_and_names_it() {
        let folder = folder_of("one_bad", &[("a.csv", "0,1,1\n"), ("b.csv", "4,4,1\n")]);

        match load_edge_folder(&folder, 10, 3, 0) {
            Err(GraphLoadError::Row { path, line, .. }) => {
                assert!(path.ends_with("b.csv"), "{path}");
                assert_eq!(line, 1);
            }
            other => panic!("expected a rejected row, got {other:?}"),
        }

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_missing_folder_is_a_loud_error() {
        let missing = std::path::Path::new("no_such_directory_here");

        match load_edge_folder(missing, 10, 3, 0) {
            Err(GraphLoadError::Io { path, .. }) => assert!(path.contains("no_such_directory")),
            other => panic!("expected an Io error, got {other:?}"),
        }
    }

    #[test]
    fn to_graph_sizes_each_file_from_its_own_highest_index() {
        // The point of the method: one folder-wide `num_nodes` is an upper
        // bound for validation, and each file's real size comes from its data.
        let small = parse("0,1,1\n1,2,1").expect("a valid three-node file");
        let large = parse("0,1,1\n1,5,1").expect("a valid six-node file");

        assert_eq!(small.to_graph(1).num_nodes, 3);
        assert_eq!(large.to_graph(1).num_nodes, 6);
    }

    #[test]
    fn to_graph_carries_every_edge_across() {
        let file = parse("0,1,1\n1,2,1\n0,2,1").expect("a valid triangle");

        let graph = file.to_graph(1);

        assert_eq!(graph.num_nodes, 3);
        for node in 0..3 {
            assert_eq!(
                graph.degree(node),
                2,
                "every node of a triangle has degree 2"
            );
        }
    }

    #[test]
    fn to_graph_of_an_edgeless_file_is_a_graph_with_no_nodes() {
        let file = parse("").expect("an empty file is a valid empty edge list");

        assert_eq!(file.to_graph(1).num_nodes, 0);
    }

    #[test]
    fn to_graph_cannot_see_a_trailing_isolated_node() {
        // Documented limitation, pinned so it is a known cost rather than a
        // surprise: nothing in the file says node 3 exists, so a graph whose
        // real size is 4 with node 3 isolated comes back as 3 nodes. Where the
        // count is known from elsewhere, pass it rather than inferring.
        let file = parse("0,1,1\n1,2,1").expect("a valid file");

        assert_eq!(
            file.to_graph(1).num_nodes,
            3,
            "inference reports the nodes that appear in edges, and only those"
        );
    }
}
