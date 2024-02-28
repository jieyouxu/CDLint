use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::hash::Hash;

use anyhow::bail;
use ariadne::{Color, Fmt, Label, Report, ReportKind};
use indexmap::{IndexMap, IndexSet};
use petgraph::{
    algo::tarjan_scc,
    dot::{Config as DotConfig, Dot},
    graph::{DiGraph, EdgeIndex, NodeIndex},
    prelude::EdgeRef,
    visit::IntoNodeReferences,
    Direction,
};
use tracing::*;

use crate::config::Config;
use crate::custom_difficulty::CustomDifficulty;
use crate::late_lints::VANILLA_ENEMY_DESCRIPTORS;
use crate::Diagnostics;

/// Enemy descriptors may cyclically reference each other via their "Base" field, but this is not
/// handled by Custom Difficulty and can crash the game.
///
/// This lint assumes that there are no undefined Enemy Descriptors used in "Base" references.
///
/// We first build a directed graph from Enemy Descriptor nodes and "based-on" directed edges. If
/// we detect any cycle in the graph, then it can potentially crash the game so we should
/// report an error for it.
///
/// A graphviz plot can be generated optionally to show the "based-on" relationships between
/// enemy descriptors.
pub fn lint_cyclic_enemy_descriptor_references<'d>(
    config: &Config,
    cd: &CustomDifficulty,
    path: &'d String,
    diag: &mut Diagnostics<'d>,
) -> anyhow::Result<()> {
    // An unweighted directed graph consisting of Enemy Descriptor nodes and "based-on" directed
    // edges.
    let mut defined_descriptors: BTreeSet<String> = BTreeSet::new();
    defined_descriptors.extend(VANILLA_ENEMY_DESCRIPTORS.iter().map(ToString::to_string));
    defined_descriptors.extend(
        config
            .extra_enemy_descriptors
            .iter()
            .map(ToString::to_string),
    );

    let mut graph: IndexMap<String, IndexSet<String>> = IndexMap::new();

    for (name, ed) in &cd.enemy_descriptors.val {
        let name = &name.val;
        let ed = &ed.val;

        defined_descriptors.insert(name.to_string());

        if !defined_descriptors.contains(&ed.base.val) {
            // We haven't seen this descriptor, it is not a vanilla or custom descriptor,
            // this might be an undefined reference that would be handled by another lint.
            // Bail now!
            bail!(format!(
                "undefined Enemy Descriptor \"{}\" encountered",
                ed.base.val
            ));
        }

        graph
            .entry(name.to_string())
            .and_modify(|e| {
                e.insert(ed.base.val.to_string());
            })
            .or_insert_with(|| IndexSet::from([ed.base.val.to_string()]));
    }

    trace!("graph =\n{:#?}", graph);

    // Assign IDs to each of the Enemy Descriptors.
    let mut vertices = IndexSet::new();
    vertices.extend(
        config
            .extra_enemy_descriptors
            .iter()
            .map(ToString::to_string),
    );
    vertices.extend(VANILLA_ENEMY_DESCRIPTORS.iter().map(ToString::to_string));
    vertices.extend(graph.keys().map(ToString::to_string));
    let mut string_edges = IndexSet::new();
    for (name, adjs) in &graph {
        for adj in adjs {
            string_edges.insert((name.to_owned(), adj.to_owned()));
        }
    }

    trace!("vertices = {:#?}", vertices);
    trace!("string_edges = {:#?}", string_edges);

    let mut digraph: DiGraph<String, ()> = DiGraph::new();
    let mut name_to_id: BTreeMap<String, NodeIndex> = BTreeMap::new();
    let mut id_to_name: BTreeMap<NodeIndex, String> = BTreeMap::new();
    for node in vertices {
        let node_idx = digraph.add_node(node.to_string());
        name_to_id.insert(node.to_string(), node_idx);
        id_to_name.insert(node_idx, node.to_string());
    }

    trace!("name_to_id = {:#?}", name_to_id);
    trace!("id_to_name = {:#?}", id_to_name);

    let mut edges = IndexSet::new();

    for (v, w) in string_edges {
        trace!(?v, ?w);
        let edge_idx = digraph.add_edge(
            *name_to_id.get(&v).unwrap(),
            *name_to_id.get(&w).unwrap(),
            (),
        );
        edges.insert(edge_idx);
    }

    let mut cycles = elementary_circuits(&digraph);
    let self_cycles = cycles
        .extract_if(|cycle| cycle.len() == 1)
        .map(|v| v[0])
        .collect::<Vec<_>>();

    if !cycles.is_empty() {
        diag.push(
            Report::build(ReportKind::Error, path, cd.enemy_descriptors.span.start)
                .with_message("cycle detected in Enemy Descriptor \"Base\" references")
                .finish(),
        );
    }

    let unspanned_enemy_descriptors = cd
        .enemy_descriptors
        .val
        .iter()
        .map(|(name, ed)| {
            (
                name.val.to_owned(),
                (ed.val.base.val.to_owned(), name.span, ed.val.base.span),
            )
        })
        .collect::<IndexMap<_, _>>();

    trace!(?unspanned_enemy_descriptors);

    for self_cycle in self_cycles {
        let node_idx = digraph
            .edge_references()
            .find(|er| er.id() == self_cycle)
            .map(|er| er.source())
            .unwrap();
        let name = id_to_name.get(&node_idx).unwrap();
        let self_cycle_idx = unspanned_enemy_descriptors
            .get_index_of(name.as_str())
            .unwrap();
        let Some(rest) = unspanned_enemy_descriptors.get_range((self_cycle_idx + 1)..) else {
            break;
        };
        trace!(?rest);

        for (other_name, (based_on, other_name_span, ed_base_span)) in rest {
            if based_on == name {
                diag.push(
                    Report::build(ReportKind::Error, path, other_name_span.start)
                        .with_message(format!(
                            "\"{}\" is self-referential, but \"{}\" references it later, which will cause a crash",
                            name.fg(Color::Blue),
                            other_name.fg(Color::Blue)
                        ))
                        .with_label(
                            Label::new((path, ed_base_span.into_range()))
                                .with_color(Color::Red)
                                .with_message(format!(
                                    "\"{}\" references \"{}\" here",
                                    other_name.fg(Color::Blue),
                                    name.fg(Color::Blue)
                                ))
                        )
                        .with_help(format!(
                            "consider moving the self-referential \"{}\" to the end of the Enemy Descriptors list",
                            name.fg(Color::Blue)
                        ))
                        .finish(),
                );
            }
        }
    }

    for (i, cycle) in cycles.iter().enumerate() {
        let mut cycle_string = String::new();

        let mut cycle_nodes = IndexSet::new();
        cycle.iter().for_each(|edge_idx| {
            let (src, dst) = digraph
                .edge_references()
                .find(|er| er.id() == *edge_idx)
                .map(|er| (er.source(), er.target()))
                .unwrap();

            cycle_nodes.insert(src);
            cycle_nodes.insert(dst);
        });

        for (j, node_idx) in cycle_nodes.iter().enumerate() {
            let name = id_to_name.get(node_idx).unwrap();
            let partial = if j == 0 {
                format!("\"{}\"", name.fg(Color::Blue))
            } else {
                format!(" -> \"{}\"", name.fg(Color::Blue))
            };

            cycle_string.push_str(&partial);
        }

        cycle_string.push_str(&format!(" -> \"{}\"", {
            let node_idx = cycle_nodes.first().unwrap();
            let name = id_to_name.get(node_idx).unwrap();
            name.fg(Color::Blue)
        }));

        diag.push(
            Report::build(ReportKind::Error, path, cd.enemy_descriptors.span.start)
                .with_message(format!("cycle [{}]: {}", i + 1, cycle_string))
                .finish(),
        );
    }

    if config.generate_cyclic_reference_graph {
        trace!(
            "{:?}",
            Dot::with_config(&digraph, &[DotConfig::EdgeNoLabel])
        );

        let exe_path = std::env::current_exe()?;
        let out_dir = exe_path.parent().unwrap();
        let out_file = out_dir.join("cyclic_enemy_descriptor_references.dot");
        std::fs::write(
            out_file,
            format!(
                "{:?}",
                Dot::with_config(&digraph, &[DotConfig::EdgeNoLabel])
            ),
        )?;
    }

    Ok(())
}

index_vec::define_index_type! {
    struct NameIdx = usize;
}

index_vec::define_index_type! {
    struct SccIdx = usize;
}

// Taken from
// <https://github.com/blockprotocol/incubator/blob/main/libs/turbine/lib/codegen/src/graph.rs>.
// MIT or Apache 2.0 license.

type ElementaryCircuit = Vec<EdgeIndex>;

/// The main loop of the cycle-enumeration algorithm of Johnson.
fn johnson_cycle_search(
    graph: &DiGraph<NodeIndex, EdgeIndex>,
    start: NodeIndex,
) -> Vec<Vec<EdgeIndex>> {
    let mut circuits = vec![];

    let mut path = vec![start];
    let mut blocked: HashSet<_> = std::iter::once(start).collect();

    let mut blocked_subgraph: HashMap<NodeIndex, HashSet<NodeIndex>> = HashMap::new();

    let mut stack = vec![graph
        .neighbors_directed(start, Direction::Outgoing)
        .fuse()
        .peekable()];

    let mut closed = vec![false];

    while let Some(neighbours) = stack.last_mut() {
        if neighbours.peek().is_none() {
            // exhausted; no more neighbours to process
            stack.pop();
            let node = path.pop().expect("infallible; non-empty");

            if closed.pop().expect("infallible; non-empty") {
                if let Some(last) = closed.last_mut() {
                    *last = true;
                }

                let mut unblock = vec![node];

                while let Some(node) = unblock.pop() {
                    if blocked.contains(&node) {
                        blocked.remove(&node);

                        if let Some(nodes) = blocked_subgraph.remove(&node) {
                            unblock.extend(nodes.into_iter());
                        }
                    }
                }
            } else {
                for neighbour in graph.neighbors_directed(node, Direction::Outgoing) {
                    let subgraph = blocked_subgraph.entry(neighbour).or_default();
                    subgraph.insert(node);
                }
            }

            continue;
        }

        // Reason: we resume the iterator in the next phase after some time,
        //  this means we do not consume the iterator and we also do not want to hold
        //  a mutable reference to the iterator while iterating through
        #[allow(clippy::while_let_on_iterator)]
        while let Some(node) = neighbours.next() {
            if node == start {
                let mut circuit = path.clone();
                circuit.push(node);

                circuits.push(circuit);

                *closed.last_mut().expect("infallible; closed is non-empty") = true;
            } else if !blocked.contains(&node) {
                path.push(node);
                closed.push(false);
                stack.push(
                    graph
                        .neighbors_directed(node, Direction::Outgoing)
                        .fuse()
                        .peekable(),
                );
                blocked.insert(node);

                break;
            }
        }
    }

    // convert to stable path identifiers
    circuits
        .into_iter()
        .map(|circuit| {
            circuit
                .windows(2)
                .map(|window| {
                    *graph
                        .edge_weight(
                            graph
                                .find_edge(window[0], window[1])
                                .expect("infallible; must exist"),
                        )
                        .expect("infallible; must exist")
                })
                .collect()
        })
        .collect()
}

/// Modified [`tarjan_scc`], which instead of returning `NodeIndex`, returns the weight.
///
///
/// This is important as we assume that the weight is constant, while node indices are not!
///
/// Returns a [`IndexSet`], as it preserves insertion order, but also allows for fast lookups
/// (needed to verify containment).
fn scc<N, E>(graph: &DiGraph<N, E>) -> impl Iterator<Item = IndexSet<N>> + '_
where
    N: Copy + Hash + Eq,
{
    // ensure that we use the canonical node weight, this is is done by using the graph weight, we
    // convert to `HashSet` as inclusion in `filter_map` is a lot faster that way
    tarjan_scc(&graph).into_iter().filter_map(|scc| {
        (scc.len() > 1).then(|| {
            scc.into_iter()
                .filter_map(|index| graph.node_weight(index).copied())
                .collect()
        })
    })
}

/// Dispatch function for [`elementary_circuits`]
///
/// We generate all cycles of `graph` through binary partition.
///
/// 1. Pick a node `v` in `G` a. Generate all cycles of `G` which contain the node `v` b.
///    Recursively generate all cycles of `G \\ v`
///
/// This is accomplished through the following:
///
/// 1. Compute the strongly connected components `SCC` of `G`
/// 2. Select and remove a biconnected component `C` from `SCC`. Select a non-tree edge `(u, v)` of
///    a depth first search of `G[C]`
/// 3. For each simple cycle `P` containing `v` in `G[C]`, yield `P`
/// 4. Add the biconnected components of `G[C \\ v]` to `SCC`
fn directed_cycle_search(mut graph: DiGraph<NodeIndex, EdgeIndex>) -> Vec<Vec<EdgeIndex>> {
    let mut components: Vec<_> = scc(&graph).collect();
    let mut circuits = vec![];

    while let Some(component) = components.pop() {
        // filter using the weight, as the index is not stable!
        let mut subgraph = graph.filter_map(
            |_, weight| component.contains(weight).then_some(*weight),
            |_, weight| Some(*weight),
        );

        let node = component
            .first()
            .copied()
            .expect("infallible; `IndexSet` has at least 2 nodes");

        let subgraph_node = subgraph
            .node_references()
            .find_map(|(index, weight)| (*weight == node).then_some(index))
            .expect("infallible; must exist");

        let graph_node = graph
            .node_references()
            .find_map(|(index, weight)| (*weight == node).then_some(index))
            .expect("infallible; must exist");

        circuits.extend(johnson_cycle_search(&subgraph, subgraph_node));

        // delete `node` after searching `graph`, to make sure we can find `v`
        // unlike networkx, subgraph views do not share the same nodes as the graph, therefore need
        // to remove them from both
        graph.remove_node(graph_node);
        subgraph.remove_node(subgraph_node);

        components.extend(scc(&subgraph));
    }

    circuits
}

/// Find elementary circuits of a graph
///
/// Implementation of the algorithm described in
/// <https://networkx.org/documentation/stable/_modules/networkx/algorithms/cycles.html#simple_cycles>
/// without the added optional length requirement which is only valid for directed graphs.
///
/// Complexity: $O((n+e)(c+1))$ for $n$ nodes, $e$ edges and $c$ simple circuits.
fn elementary_circuits<N, E>(graph: &DiGraph<N, E>) -> Vec<ElementaryCircuit> {
    // first report all self loops, they are not processed otherwise
    let mut circuits: Vec<_> = graph
        .edge_references()
        .filter(|edge| edge.source() == edge.target())
        .map(|edge| vec![edge.id()])
        .collect();

    // explicitly convert our graph into a graph where each weight has the original weight index,
    // node weights are not important and are therefore discarded.
    // we need the `EdgeIndex` as weight, because we remove edges, which will force reordering
    // in that case we could mark the wrong edge as circuit.
    let mut graph = graph.filter_map(|index, _| Some(index), |index, _| Some(index));
    let mut traversed = HashSet::new();

    // remove all self-loops and parallel edges
    graph.retain_edges(|graph, edge| {
        let (source, target) = graph
            .edge_endpoints(edge)
            .expect("infallible; edge must exist in graph");

        // filter out any parallel edges
        if traversed.contains(&(source, target)) {
            return false;
        }

        traversed.insert((source, target));

        // remove all self loops
        source != target
    });

    circuits.extend(directed_cycle_search(graph));

    circuits
        .into_iter()
        .map(|path| path.into_iter().collect())
        .collect()
}
