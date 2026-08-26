//! Reading graphs from edge-list files.
//!
//! One edge per line, `start,end,weight`, comma-delimited, any line ending. A
//! caller whose data is not 0-indexed passes `min_node_index`, and every index
//! is shifted to 0 here — once, on the way in.
//!
//! **A line starting with `#` is a comment, and every file must carry the one
//! comment that is read: `# nodes = N`.** It states the graph's node count,
//! which the edges cannot — a node with no edges is invisible to
//! `highest index + 1`, so a count taken from the data is short by exactly the
//! nodes hardest to notice, and short silently. It is required rather than
//! optional on purpose: two ways to arrive at a node count means one that is
//! right and one that is quietly wrong, chosen by whether whoever wrote the
//! file remembered. The header is checked against the file's own indices and
//! against the count the caller allows.
//!
//! Everything GET writes carries it — `RunResult::save_results` emits a
//! loadable edge list — so a run's output is a file the loader will take back
//! without editing.
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
//! testable without a Python interpreter: the Python boundary raises them as
//! a `UserWarning`, through `warnings`, and the Rust-native route (`get-run`,
//! which has no interpreter to raise one on) prints them to stderr instead —
//! see `crate::emit_load_warnings_maybe`.

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
    /// The node count the file stated in its `# nodes = N` header.
    ///
    /// Required, and there is deliberately no second way to arrive at it: a
    /// trailing node with no edges is invisible to `highest index + 1`, so a
    /// format accepting both spellings would have one that is right and one
    /// that is quietly wrong by exactly the nodes hardest to notice.
    pub num_nodes: usize,
}

impl EdgeFile {
    /// Build the graph these edges describe, sizing it from the data itself.
    ///
    /// # Why the count is the file's own, not the loader's
    ///
    /// [`load_edge_folder`] takes **one** `num_nodes` for the whole folder,
    /// which suits a set of same-sized graphs and does not suit a reference
    /// set: those come from real data and differ in size, and the loader's
    /// `num_nodes` is an upper bound used to reject out-of-range indices, not
    /// a description of any one file. So a caller passes a generous cap to the
    /// loader — which still catches a wild index — and each file states its
    /// own size, which is what this builds from.
    ///
    /// **The size comes from the file's `# nodes = N` header, always.** Edges
    /// cannot supply it: nothing in them distinguishes "node 9 exists but has
    /// no edges" from "there is no node 9", so a count inferred as
    /// `highest index + 1` is short by exactly the trailing isolated nodes, and
    /// short silently. That is not cosmetic for a reference set — the degree,
    /// clustering and spectral histograms each count an isolated node as a real
    /// observation and normalize over the node count, so a lost node shifts
    /// every distribution consistently in one direction. `parse_edge_list` has
    /// already checked the header against the indices the file uses and against
    /// the count the caller allows, so this is a construction, not a decision.
    pub fn to_graph(&self, max_edge_multiplicity: u32) -> Graph {
        let mut graph = Graph::new(self.num_nodes, max_edge_multiplicity);
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
    /// A `# nodes` header whose value is not a whole number of nodes.
    NonNumericNodeCount,
    /// A second `# nodes` header. One file states one node count; a second is a
    /// contradiction rather than an override, and honouring either would be a
    /// guess. Carries the line the first one was on.
    RepeatedNodeCount { first: usize },
    /// A `# nodes` header naming fewer nodes than the file's own edges use.
    /// Carries the count and the smallest one that would fit the data.
    NodeCountBelowIndices { declared: usize, needed: usize },
    /// A `# nodes` header above the count this run allows — the header's
    /// counterpart to [`RowProblem::NodeOutOfRange`]. Carries both.
    NodeCountAboveCap { declared: usize, cap: usize },
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
            RowProblem::NonNumericNodeCount => write!(
                f,
                "the `# nodes` header is not a whole number of nodes; write it as \
                 `# nodes = 200`"
            ),
            RowProblem::RepeatedNodeCount { first } => write!(
                f,
                "a second `# nodes` header, line {first} having already given one; a \
                 file states one node count, and two is a contradiction rather than \
                 an override"
            ),
            RowProblem::NodeCountBelowIndices { declared, needed } => write!(
                f,
                "the `# nodes` header says {declared}, but this file's own edges need \
                 at least {needed}; the header is what the graph is sized from, so a \
                 count below the data would drop edges without a word"
            ),
            RowProblem::NodeCountAboveCap { declared, cap } => write!(
                f,
                "the `# nodes` header says {declared}, above the {cap} this run allows; \
                 an index that high is rejected row by row, and a header claiming one \
                 is rejected for the same reason"
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
    /// No `# nodes = N` header. A file-level failure rather than a row one:
    /// there is no line to point at, which is the whole problem.
    MissingNodeCount {
        /// The file that did not state its size.
        path: String,
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
            GraphLoadError::MissingNodeCount { path } => write!(
                f,
                "`{path}` has no `# nodes = N` header, and a node count cannot be read \
                 from edges: a node with no edges is invisible there, so an inferred \
                 count is short by exactly the nodes that were hardest to notice. \
                 State it, e.g. `# nodes = 200`"
            ),
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
    let mut declared_nodes: Option<(usize, usize)> = None;

    for (index, raw_line) in text.lines().enumerate() {
        let line = index + 1;
        let trimmed = raw_line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // A `#` line is a comment, and one spelling of comment is load-bearing:
        // `# nodes = 200` is how a file states a size its edges cannot imply.
        // Anything else after the `#` is ignored, so a file may carry its own
        // provenance without the parser growing an opinion about it.
        if let Some(comment) = trimmed.strip_prefix('#') {
            let Some(value) = node_count_header(comment) else {
                continue;
            };

            let count = match value.trim().parse::<i64>() {
                Ok(count) if count >= 0 => count as usize,
                _ => {
                    return Err(GraphLoadError::Row {
                        path: source.to_string(),
                        line,
                        problem: RowProblem::NonNumericNodeCount,
                    });
                }
            };

            if let Some((_, first)) = declared_nodes {
                return Err(GraphLoadError::Row {
                    path: source.to_string(),
                    line,
                    problem: RowProblem::RepeatedNodeCount { first },
                });
            }

            if count > num_nodes {
                return Err(GraphLoadError::Row {
                    path: source.to_string(),
                    line,
                    problem: RowProblem::NodeCountAboveCap {
                        declared: count,
                        cap: num_nodes,
                    },
                });
            }

            declared_nodes = Some((count, line));
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

    // Checked after the rows, not while reading them: the header may come last,
    // and a file is not required to put it first for the check to mean anything.
    let Some((count, header_line)) = declared_nodes else {
        return Err(GraphLoadError::MissingNodeCount {
            path: source.to_string(),
        });
    };

    let mut needed = 0;
    for edge in &parsed {
        if edge.u + 1 > needed {
            needed = edge.u + 1;
        }
        if edge.v + 1 > needed {
            needed = edge.v + 1;
        }
    }

    if count < needed {
        return Err(GraphLoadError::Row {
            path: source.to_string(),
            line: header_line,
            problem: RowProblem::NodeCountBelowIndices {
                declared: count,
                needed,
            },
        });
    }

    let (edges, duplicates) = canonicalize(&parsed, min_node_index);
    warnings.extend(duplicates);

    Ok(EdgeFile {
        edges,
        warnings,
        source: source.to_string(),
        num_nodes: count,
    })
}

/// The value of a `# nodes = N` header, or `None` for any other comment.
///
/// `comment` is what followed the `#`. The key is matched case-insensitively
/// and the `=` is required, so `# nodes: 5` and `# 200 nodes` are prose and are
/// skipped rather than half-read. A file that means to state a size and
/// misspells it gets no count, which the caller sees as inference — the reason
/// a malformed *value* is an error rather than a shrug.
fn node_count_header(comment: &str) -> Option<&str> {
    let (key, value) = comment.split_once('=')?;
    if key.trim().eq_ignore_ascii_case("nodes") {
        Some(value)
    } else {
        None
    }
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
    ///
    /// The `# nodes` header every file must carry is **appended**, not
    /// prepended, so a fixture's own rows keep the line numbers its assertions
    /// name. Tests about the header itself write their own and call
    /// [`parse_edge_list`] directly.
    fn parse(text: &str) -> Result<EdgeFile, GraphLoadError> {
        let body = text.trim_end_matches('\n');
        parse_edge_list(&format!("{body}\n# nodes = 10\n"), "test", 10, 3, 0)
    }

    /// [`parse`]'s settings, but the text exactly as written — for the tests
    /// about the header itself, which supply their own or deliberately none.
    fn parse_raw(text: &str) -> Result<EdgeFile, GraphLoadError> {
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

    /// The whole point of the header: a graph whose last nodes have no edges.
    /// The edges cannot say so, which is why the count is stated and not
    /// inferred.
    #[test]
    fn the_node_count_header_sizes_the_graph_past_its_highest_edge() {
        let loaded = parse_raw("# nodes = 6\n0,1,1\n1,2,1\n").expect("valid");

        assert_eq!(loaded.num_nodes, 6);
        assert_eq!(loaded.to_graph(3).num_nodes, 6);
        assert!(loaded.warnings.is_empty(), "{:?}", loaded.warnings);

        // Nodes 3, 4 and 5 are real and isolated: present in the graph, absent
        // from every edge, and counted by anything that reads degrees.
        let graph = loaded.to_graph(3);
        assert_eq!(graph.neighbor_count(5), 0);
    }

    /// Missing is an error, not a fallback. The alternative — infer when the
    /// header is absent — is two ways to reach a node count, one of which is
    /// wrong by exactly the nodes nobody can see.
    #[test]
    fn a_file_that_does_not_state_its_size_is_rejected() {
        match parse_raw("0,1,1\n1,2,1\n") {
            Err(GraphLoadError::MissingNodeCount { path }) => assert_eq!(path, "test"),
            other => panic!("expected a missing-header rejection, got {other:?}"),
        }
    }

    /// Spelling the parser accepts, and prose it leaves alone. A comment that
    /// is not a header must not be half-read, or a provenance note becomes a
    /// silent resize.
    #[test]
    fn comments_are_skipped_and_only_the_nodes_key_is_read() {
        let loaded =
            parse_raw("# converted from MUTAG\n#NODES=7\n0,1,1\n# trailing note\n").expect("valid");

        assert_eq!(loaded.num_nodes, 7);
        assert_eq!(loaded.edges, vec![(0, 1, 1)]);

        // Neither of these is a header, so the file states nothing and is
        // rejected — a near-miss spelling must not be read as a count.
        assert!(matches!(
            parse_raw("# 7 nodes\n# nodes: 7\n0,1,1\n"),
            Err(GraphLoadError::MissingNodeCount { .. })
        ));
    }

    /// The header is a count, not an index, so `min_node_index` must not touch
    /// it. A 1-indexed file of 5 nodes says 5, not 6 — and shifting it would be
    /// the kind of off-by-one that produces a valid-looking graph.
    #[test]
    fn the_node_count_is_not_shifted_by_min_node_index() {
        let loaded = parse_edge_list("# nodes = 5\n1,2,1\n2,3,1\n", "test", 10, 3, 1)
            .expect("a 1-indexed file that states five nodes");

        assert_eq!(loaded.num_nodes, 5);
        assert_eq!(loaded.edges, vec![(0, 1, 1), (1, 2, 1)]);
    }

    #[test]
    fn a_malformed_or_repeated_node_count_is_rejected() {
        assert_eq!(
            problem(parse_raw("# nodes = many\n0,1,1\n")),
            (1, RowProblem::NonNumericNodeCount)
        );
        assert_eq!(
            problem(parse_raw("# nodes = -4\n0,1,1\n")),
            (1, RowProblem::NonNumericNodeCount)
        );
        assert_eq!(
            problem(parse_raw("# nodes = 5\n0,1,1\n# nodes = 6\n")),
            (3, RowProblem::RepeatedNodeCount { first: 1 })
        );
    }

    /// Both directions the header can disagree with its surroundings: below the
    /// file's own edges, and above what the caller allows.
    #[test]
    fn a_node_count_that_fits_neither_the_data_nor_the_cap_is_rejected() {
        // The header is checked after the rows, so it is caught wherever it sits.
        assert_eq!(
            problem(parse_raw("0,1,1\n2,4,1\n# nodes = 3\n")),
            (
                3,
                RowProblem::NodeCountBelowIndices {
                    declared: 3,
                    needed: 5,
                }
            )
        );
        assert_eq!(
            problem(parse_raw("# nodes = 11\n0,1,1\n")),
            (
                1,
                RowProblem::NodeCountAboveCap {
                    declared: 11,
                    cap: 10,
                }
            )
        );
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
        let loaded =
            parse_edge_list("# nodes = 3\n1,2,1\n2,3,2\n", "test", 3, 3, 1).expect("valid");

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
        // The user reads their own file, not our 0-based copy of it — and the
        // line is the file's own line 3, header included, since that is the
        // line they would go to.
        let loaded =
            parse_edge_list("# nodes = 3\n1,2,1\n2,1,3\n", "test", 3, 3, 1).expect("valid");

        assert_eq!(
            loaded.warnings,
            vec![LoadWarning::DuplicateEdge {
                edge: (1, 2),
                kept: 3,
                line: Some(3),
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
    ///
    /// Each file gains the `# nodes = 10` header the loader requires, appended
    /// for the same reason [`parse`] appends it.
    fn folder_of(name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
        let folder = std::env::temp_dir().join(format!("get_graph_io_{name}"));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).expect("temp folder");

        for (file_name, text) in files {
            let mut file = std::fs::File::create(folder.join(file_name)).expect("temp file");
            file.write_all(format!("{text}# nodes = 10\n").as_bytes())
                .expect("write");
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
    fn every_regular_file_is_read_whatever_it_is_called() {
        // There is no extension convention: an edge list is whatever the
        // caller put in the folder. Every other folder test here uses `.csv`,
        // so a filter narrowing the load to one extension — and silently
        // dropping data the caller meant to include — would pass all of them.
        let folder = folder_of(
            "any_extension",
            &[("a.csv", "0,1,1\n"), ("b.txt", "1,2,1\n"), ("c", "2,3,1\n")],
        );

        let loaded = load_edge_folder(&folder, 10, 3, 0).expect("valid");

        assert_eq!(loaded.len(), 3);
        assert_eq!(loaded[0].edges, vec![(0, 1, 1)]);
        assert_eq!(loaded[1].edges, vec![(1, 2, 1)]);
        assert_eq!(loaded[2].edges, vec![(2, 3, 1)]);

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn a_sub_directory_is_skipped_rather_than_read() {
        // Handed to load_edge_file a directory fails the read, so skipping it
        // is what keeps a folder with any structure in it loadable at all.
        let folder = folder_of("with_subdir", &[("a.csv", "0,1,1\n")]);
        std::fs::create_dir_all(folder.join("nested")).expect("nested folder");

        let loaded = load_edge_folder(&folder, 10, 3, 0).expect("valid");

        assert_eq!(loaded.len(), 1);
        assert_eq!(loaded[0].edges, vec![(0, 1, 1)]);

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    #[test]
    fn an_empty_folder_loads_as_an_empty_set() {
        let folder = folder_of("empty", &[]);

        let loaded = load_edge_folder(&folder, 10, 3, 0).expect("an empty folder is readable");

        assert!(loaded.is_empty());

        std::fs::remove_dir_all(&folder).expect("cleanup");
    }

    /// The variants are what the tests above assert on, deliberately, so the
    /// wording stays free to improve. What is not free to change is that a
    /// message still names the numbers telling the user what to fix — a
    /// dropped interpolation leaves prose that reads fine and helps nobody.
    #[test]
    fn a_rejection_renders_carrying_the_numbers_that_locate_it() {
        let contains_all = |rendered: &str, numbers: &[&str]| {
            for number in numbers {
                assert!(
                    rendered.contains(number),
                    "`{number}` missing from: {rendered}"
                );
            }
        };

        contains_all(
            &RowProblem::NodeOutOfRange {
                index: 42,
                low: 7,
                high: 31,
            }
            .to_string(),
            &["42", "7", "31"],
        );
        contains_all(
            &RowProblem::WeightAboveCap { weight: 48, cap: 6 }.to_string(),
            &["48", "6"],
        );
        contains_all(
            &RowProblem::NodeCountBelowIndices {
                declared: 13,
                needed: 27,
            }
            .to_string(),
            &["13", "27"],
        );
        contains_all(
            &RowProblem::NodeCountAboveCap {
                declared: 91,
                cap: 24,
            }
            .to_string(),
            &["91", "24"],
        );
        contains_all(&RowProblem::ColumnCount(5).to_string(), &["5"]);
        contains_all(
            &RowProblem::RepeatedNodeCount { first: 18 }.to_string(),
            &["18"],
        );
        contains_all(&RowProblem::NonNumeric("weight").to_string(), &["weight"]);

        // A rejected row reports where it was and keeps the problem's own
        // words, rather than replacing them with a summary of its own.
        let row = GraphLoadError::Row {
            path: "graphs/mutag_17.csv".to_string(),
            line: 42,
            problem: RowProblem::NegativeWeight(-3),
        }
        .to_string();
        contains_all(&row, &["graphs/mutag_17.csv", "42"]);
        assert!(
            row.contains(&RowProblem::NegativeWeight(-3).to_string()),
            "{row}"
        );
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
    fn to_graph_sizes_each_file_from_its_own_header() {
        // The point of the method: one folder-wide `num_nodes` is an upper
        // bound for validation, and each file's real size is its own business.
        let small = parse_raw("# nodes = 3\n0,1,1\n1,2,1\n").expect("a three-node file");
        let large = parse_raw("# nodes = 6\n0,1,1\n1,5,1\n").expect("a six-node file");

        assert_eq!(small.to_graph(1).num_nodes, 3);
        assert_eq!(large.to_graph(1).num_nodes, 6);
    }

    #[test]
    fn to_graph_carries_every_edge_across() {
        let file = parse_raw("# nodes = 3\n0,1,1\n1,2,1\n0,2,1\n").expect("a valid triangle");

        let graph = file.to_graph(1);

        assert_eq!(graph.num_nodes, 3);
        for node in 0..3 {
            assert_eq!(
                graph.neighbor_count(node),
                2,
                "every node of a triangle has degree 2"
            );
        }
    }

    /// A file with no edges is still a graph, and its header still says how
    /// big: five nodes, none of them connected. Under an inferred count this
    /// case was indistinguishable from an empty graph.
    #[test]
    fn to_graph_of_an_edgeless_file_is_as_big_as_its_header_says() {
        let file = parse_raw("# nodes = 5\n").expect("a header alone is a valid file");

        let graph = file.to_graph(1);
        assert_eq!(graph.num_nodes, 5);
        assert_eq!(graph.neighbor_count(4), 0);
        assert_eq!(file.warnings, vec![LoadWarning::EmptyFile]);

        // Zero is expressible too, and means what it says.
        let empty = parse_raw("# nodes = 0\n").expect("zero nodes is a valid claim");
        assert_eq!(empty.to_graph(1).num_nodes, 0);
    }

    /// The failure the header exists to remove, kept as a test of the new
    /// behaviour rather than deleted: node 3 appears in no edge, and the file
    /// is the only thing that can say it is there.
    #[test]
    fn to_graph_sees_a_trailing_isolated_node_because_the_file_states_it() {
        let file = parse_raw("# nodes = 4\n0,1,1\n1,2,1\n").expect("a valid file");

        assert_eq!(
            file.to_graph(1).num_nodes,
            4,
            "the header is the whole reason the fourth node survives the format"
        );
    }
}
