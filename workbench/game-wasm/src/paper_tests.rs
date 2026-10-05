//! Tests of the algorithms of Hiraoka, Imori, Takahashi and Sudo (arXiv 2609.14356): A_TC3, A_C4 and A_US5.
//!
//! Four kinds of evidence that the rules were transcribed correctly:
//!   * every rule of each algorithm, on a row written out by hand from the paper's definition;
//!   * the two traces of Figure 2 of the paper (A_C4), whose rule sequences the figure prints;
//!   * every connected graph of up to seven vertices (up to isomorphism), from every start vertex, against every
//!     adversary: the algorithm must explore each graph of the class the paper proves it for;
//!   * a scan of every reachable position for what the paper counts as a failure but this engine would
//!     silently turn into a stay: an undefined row, or a request for a colour no neighbour has.

use super::*;
use std::collections::HashSet;

fn rule_of(f: Formula) -> Rule {
    let (_, k) = f.paper_algorithm().unwrap();
    Rule { k, table: Map::default(), default: Some(f), terminating: true }
}

fn graph(n: usize, start: usize, edges: &[(u8, u8)]) -> Graph {
    let list: Vec<String> = edges.iter().map(|(a, b)| format!("{}-{}", a, b)).collect();
    Graph::parse(&format!("{} {} {}", n, start, list.join(","))).unwrap()
}

// ---------------------------------------------------------------- the rules, one row each
/// (row, action, rule number) for an algorithm, each row derived by hand from the paper's rule.
fn check_rows(f: Formula, rows: &[(&str, Option<(&str, u8)>)]) {
    let (name, k) = f.paper_algorithm().unwrap();
    for (text, want) in rows {
        let row = Row::parse(text).unwrap();
        let got = f.numbered(&row, k).map(|(a, n)| (a.text(), n));
        assert_eq!(got, want.map(|(a, n)| (a.to_string(), n)), "{} on row {}", name, text);
    }
}

#[test]
fn atc3_rules_d1_to_d11_on_hand_derived_rows() {
    // colours 0 init, 1 l0, 2 l1; the bag has one digit per colour
    check_rows(
        Formula::ATC3,
        &[
            ("0.000", Some(("2>stop", 1))),  // D1: no neighbour at all
            ("0.200", Some(("1>0", 2))),     // D2: only init neighbours
            ("0.110", Some(("2>0", 3))),     // D3: init and l0
            ("0.111", Some(("2>0", 3))),     // D3 before D4
            ("0.101", Some(("0>2", 4))),     // D4: init and l1
            ("0.001", Some(("2>2", 5))),     // D5: l1 only
            ("0.021", Some(("2>2", 5))),     // D5 before D6
            ("0.010", Some(("2>1", 6))),     // D6: l0 only
            ("1.101", Some(("1>0", 7))),     // D7: init and l1
            ("1.111", Some(("1>0", 7))),     // D7 before D8
            ("1.011", Some(("2>1", 8))),     // D8: l0 and l1
            ("1.001", Some(("2>stop", 9))),  // D9: l1 only
            ("1.010", None),                 // omitted: unreachable
            ("1.200", None),                 // omitted
            ("2.110", Some(("1>0", 10))),    // D10: init and l0
            ("2.111", Some(("1>0", 10))),    // D10 before D11
            ("2.011", Some(("2>1", 11))),    // D11: l0 and l1
            ("2.100", None),                 // omitted
        ],
    );
    // fewer colours than three: no answer at all
    assert_eq!(Formula::ATC3.numbered(&Row::parse("0.20").unwrap(), 2), None);
}

#[test]
fn ac4_rules_c1_to_c14_on_hand_derived_rows() {
    // colours 0 init, 1 fin, 2 front, 3 path
    check_rows(
        Formula::AC4,
        &[
            ("0.0000", Some(("1>stop", 1))), // C1: nothing around
            ("3.0100", Some(("1>stop", 1))), // C1 for any own colour but fin: only fin around
            ("0.1000", Some(("2>0", 2))),    // C2: init neighbours only
            ("3.1000", Some(("2>0", 2))),    // C2 for a path vertex too
            ("0.0001", Some(("1>3", 3))),    // C3: path only (no front, no init)
            ("0.0101", Some(("1>3", 3))),    // C3 with fin around
            ("0.0010", Some(("1>2", 4))),    // C4: front, no init
            ("0.0011", Some(("1>2", 4))),    // C4 before C5
            ("0.1011", Some(("3>0", 5))),    // C5: init, front and path
            ("0.1001", Some(("2>0", 6))),    // C6: init and path
            ("0.1010", Some(("2>2", 7))),    // C7: init and front
            ("0.1100", None),                // an init vertex seeing only init and fin: unreachable
            ("2.0010", Some(("3>2", 8))),    // C8: front vertex seeing a front neighbour
            ("2.1011", Some(("3>2", 8))),    // C8 before C9
            ("2.1001", Some(("2>0", 9))),    // C9: no front neighbour, an init one
            ("2.0101", Some(("1>3", 10))),   // C10: otherwise
            ("3.1100", Some(("2>0", 11))),   // C11: path vertex, init and no front
            ("3.1010", Some(("3>0", 12))),   // C12: path vertex, init and front
            ("3.0010", Some(("1>2", 13))),   // C13: path vertex, front and no init
            ("3.0101", Some(("1>3", 14))),   // C14: otherwise
            ("1.1000", None),                // fin: no rule
            ("1.0000", None),
        ],
    );
}

#[test]
fn aus5_rules_u1_to_u19_on_hand_derived_rows() {
    // colours 0 init, 1 path, 2 neigh, 3 fin, 4 head
    check_rows(
        Formula::AUS5,
        &[
            ("0.10000", Some(("1>0", 1))),      // U1: init neighbours only
            ("0.20000", Some(("1>0", 1))),
            ("0.00100", Some(("1>stay", 2))),   // U2: neigh, no head
            ("0.10100", Some(("1>stay", 2))),   // U1 needs no neigh: U2 answers
            ("0.01001", Some(("2>4", 3))),      // U3: head and path
            ("0.01101", Some(("2>4", 3))),      // U2 needs no head: U3 answers
            ("0.00001", Some(("4>4", 4))),      // U4: head, no path
            ("0.00010", Some(("4>stay", 5))),   // U5: otherwise
            ("0.01000", Some(("4>stay", 5))),
            ("4.10100", Some(("3>0", 6))),      // U6: init and neigh, no path, no head
            ("4.00101", Some(("4>2", 7))),      // U7: head and neigh
            ("4.10101", Some(("4>2", 7))),      // U6 needs no head
            ("4.01001", Some(("1>4", 8))),      // U8: head and path
            ("4.00001", Some(("3>4", 9))),      // U9: head
            ("4.11000", Some(("4>0", 10))),     // U10: path and init
            ("4.11100", Some(("4>0", 10))),     // U6 needs no path
            ("4.01000", Some(("4>1", 11))),     // U11: path
            ("4.10000", Some(("1>0", 12))),     // U12: init
            ("4.00010", Some(("3>stop", 13))),  // U13: otherwise
            ("4.00000", Some(("3>stop", 13))),
            ("1.00101", Some(("0>4", 14))),     // U14: neigh and head
            ("1.00100", Some(("1>2", 15))),     // U15: neigh
            ("1.10000", Some(("4>stay", 16))),  // U16: no head
            ("1.00001", Some(("4>4", 17))),     // U17: head
            ("2.01000", Some(("0>1", 18))),     // U18: path, no head
            ("2.00001", Some(("0>4", 19))),     // U19: otherwise
            ("2.01001", Some(("0>4", 19))),     // path and head: U19
            ("3.10000", None),                  // fin: no rule
        ],
    );
}

// ---------------------------------------------------------------- Figure 2 of the paper
fn rule_sequence(g: &Graph, f: Formula, pref: &[u8]) -> Vec<u8> {
    let rule = rule_of(f);
    match play(g, &rule, pref, 1000) {
        Outcome::Explores { trace, .. } => {
            assert_eq!(trace.stopped_at, Some(g.s), "the play must end with a stop on the start vertex");
            trace.steps.iter().map(|s| rule.rule_number(&s.row).expect("a numbered rule")).collect()
        }
        _ => panic!("the play must explore"),
    }
}

#[test]
fn ac4_reproduces_the_rule_sequences_printed_in_figure_two() {
    // (a) a bridge and a cycle: s - a, and the cycle a b c d; the figure prints
    //     C2, C7, C8, (C9, C7, C8) twice, C9, C4, C10, C14 twice, C1
    let g = graph(5, 0, &[(0, 1), (1, 2), (2, 3), (3, 4), (1, 4)]); // s=0, a=1, b=2, c=3, d=4
    assert_eq!(rule_sequence(&g, Formula::AC4, &[2, 3, 4, 1, 0]), vec![2, 7, 8, 9, 7, 8, 9, 7, 8, 9, 4, 10, 14, 14, 1]);
    // (b) the complete bipartite block K3,2 with the start r in the side of three; the figure prints
    //     C2, C7, C8, C9, C7, C8, C9, C5, C3, C13, C10, C14, C1
    // r=0, x1=1, x2=2 | y1=3, y2=4
    let k32 = graph(5, 0, &[(0, 3), (0, 4), (1, 3), (1, 4), (2, 3), (2, 4)]);
    assert_eq!(rule_sequence(&k32, Formula::AC4, &[3, 1, 2, 4, 0]), vec![2, 7, 8, 9, 7, 8, 9, 5, 3, 13, 10, 14, 1]);
}

// ---------------------------------------------------------------- small graphs and their classes
/// A representative of every connected graph on `n` vertices up to isomorphism, as an edge list.
pub(crate) fn connected_graphs(n: usize) -> Vec<Vec<(u8, u8)>> {
    let pairs: Vec<(usize, usize)> = (0..n).flat_map(|a| (a + 1..n).map(move |b| (a, b))).collect();
    let index = |a: usize, b: usize| pairs.iter().position(|&p| p == (a.min(b), a.max(b))).unwrap();
    fn permutations(items: &mut Vec<usize>, at: usize, out: &mut Vec<Vec<usize>>) {
        if at == items.len() {
            out.push(items.clone());
            return;
        }
        for i in at..items.len() {
            items.swap(at, i);
            permutations(items, at + 1, out);
            items.swap(at, i);
        }
    }
    let mut perms = Vec::new();
    permutations(&mut (0..n).collect(), 0, &mut perms);
    let maps: Vec<Vec<usize>> = perms.iter().map(|p| pairs.iter().map(|&(a, b)| index(p[a], p[b])).collect()).collect();
    let total = 1usize << pairs.len();
    let mut seen = vec![false; total];
    let mut reps = Vec::new();
    for mask in 0..total {
        if seen[mask] {
            continue;
        }
        for m in &maps {
            let mut image = 0usize;
            for (i, &j) in m.iter().enumerate() {
                if mask >> i & 1 == 1 {
                    image |= 1 << j;
                }
            }
            seen[image] = true;
        }
        let edges: Vec<(u8, u8)> = pairs.iter().enumerate().filter(|(i, _)| mask >> i & 1 == 1).map(|(_, &(a, b))| (a as u8, b as u8)).collect();
        if n == 1 || connected(n, &edges) {
            reps.push(edges);
        }
    }
    reps
}

fn connected(n: usize, edges: &[(u8, u8)]) -> bool {
    let mut seen = vec![false; n];
    let mut todo = vec![0usize];
    seen[0] = true;
    while let Some(x) = todo.pop() {
        for &(a, b) in edges {
            for (u, v) in [(a as usize, b as usize), (b as usize, a as usize)] {
                if u == x && !seen[v] {
                    seen[v] = true;
                    todo.push(v);
                }
            }
        }
    }
    seen.iter().all(|&s| s)
}

/// The blocks of a connected graph (maximal biconnected subgraphs and bridges), as edge lists (Tarjan).
fn blocks(n: usize, edges: &[(u8, u8)]) -> Vec<Vec<(u8, u8)>> {
    struct State {
        adj: Vec<Vec<usize>>,
        disc: Vec<usize>,
        low: Vec<usize>,
        time: usize,
        stack: Vec<(usize, usize)>,
        out: Vec<Vec<(u8, u8)>>,
    }
    fn dfs(s: &mut State, u: usize, parent: Option<usize>) {
        s.time += 1;
        s.disc[u] = s.time;
        s.low[u] = s.time;
        for v in s.adj[u].clone() {
            if s.disc[v] == 0 {
                s.stack.push((u, v));
                dfs(s, v, Some(u));
                s.low[u] = s.low[u].min(s.low[v]);
                if s.low[v] >= s.disc[u] {
                    let mut block = Vec::new();
                    loop {
                        let e = s.stack.pop().unwrap();
                        block.push((e.0.min(e.1) as u8, e.0.max(e.1) as u8));
                        if e == (u, v) {
                            break;
                        }
                    }
                    s.out.push(block);
                }
            } else if Some(v) != parent && s.disc[v] < s.disc[u] {
                s.stack.push((u, v));
                s.low[u] = s.low[u].min(s.disc[v]);
            }
        }
    }
    let mut s = State { adj: vec![Vec::new(); n], disc: vec![0; n], low: vec![0; n], time: 0, stack: Vec::new(), out: Vec::new() };
    for &(a, b) in edges {
        s.adj[a as usize].push(b as usize);
        s.adj[b as usize].push(a as usize);
    }
    dfs(&mut s, 0, None);
    s.out
}

fn vertices_of(block: &[(u8, u8)]) -> Vec<u8> {
    let mut v: Vec<u8> = block.iter().flat_map(|&(a, b)| [a, b]).collect();
    v.sort();
    v.dedup();
    v
}

fn has_edge(block: &[(u8, u8)], a: u8, b: u8) -> bool {
    block.contains(&(a.min(b), a.max(b)))
}

fn is_clique(block: &[(u8, u8)]) -> bool {
    let v = vertices_of(block).len();
    block.len() == v * (v - 1) / 2
}

fn is_triangle_free(block: &[(u8, u8)]) -> bool {
    let v = vertices_of(block);
    for &a in &v {
        for &b in &v {
            for &c in &v {
                if a < b && b < c && has_edge(block, a, b) && has_edge(block, b, c) && has_edge(block, a, c) {
                    return false;
                }
            }
        }
    }
    true
}

fn is_cycle(block: &[(u8, u8)]) -> bool {
    let v = vertices_of(block);
    v.len() >= 3 && block.len() == v.len() && v.iter().all(|&x| block.iter().filter(|&&(a, b)| a == x || b == x).count() == 2)
}

fn is_complete_bipartite(block: &[(u8, u8)]) -> bool {
    // two-colour the block; it is K(p,q) when every pair across the sides is an edge
    let v = vertices_of(block);
    let mut side: Vec<Option<bool>> = vec![None; 32];
    side[v[0] as usize] = Some(false);
    let mut todo = vec![v[0]];
    while let Some(x) = todo.pop() {
        for &(a, b) in block {
            let other = if a == x { b } else if b == x { a } else { continue };
            match side[other as usize] {
                None => {
                    side[other as usize] = Some(!side[x as usize].unwrap());
                    todo.push(other);
                }
                Some(s) if s == side[x as usize].unwrap() => return false,
                _ => {}
            }
        }
    }
    let p = v.iter().filter(|&&x| side[x as usize] == Some(false)).count();
    block.len() == p * (v.len() - p)
}

/// Trees and simple cycles (the class of A_TC3).
fn in_tree_or_cycle(n: usize, edges: &[(u8, u8)]) -> bool {
    edges.len() == n - 1 || (edges.len() == n && n >= 3 && (0..n as u8).all(|x| edges.iter().filter(|&&(a, b)| a == x || b == x).count() == 2))
}
/// Every block is a bridge, a cycle or a complete bipartite graph (the class of A_C4).
fn in_gcb(n: usize, edges: &[(u8, u8)]) -> bool {
    blocks(n, edges).iter().all(|b| b.len() == 1 || is_cycle(b) || is_complete_bipartite(b))
}
/// Every block is a clique or triangle-free (the class of A_US5).
fn in_gktf(n: usize, edges: &[(u8, u8)]) -> bool {
    blocks(n, edges).iter().all(|b| is_clique(b) || is_triangle_free(b))
}
/// Connected with at most one cycle and no vertex of degree above three (a subcubic pseudotree).
fn is_subcubic_pseudotree(n: usize, edges: &[(u8, u8)]) -> bool {
    edges.len() <= n && (0..n as u8).all(|x| edges.iter().filter(|&&(a, b)| a == x || b == x).count() <= 3)
}

#[test]
fn the_class_tests_agree_with_the_paper() {
    // the number of connected graphs on 1 to 6 vertices, up to isomorphism (OEIS A001349): the enumeration is complete
    let counts: Vec<usize> = (1..=6).map(|n| connected_graphs(n).len()).collect();
    assert_eq!(counts, vec![1, 1, 2, 6, 21, 112]);
    // examples from the paper: the diamond K4 - e is phi-free but not in G_KTF (its block has a triangle and is no clique)
    let diamond = [(0u8, 1u8), (0, 2), (1, 2), (1, 3), (2, 3)];
    assert!(!in_gktf(4, &diamond));
    // K5 is in G_KTF, but not in G_CB
    let k5: Vec<(u8, u8)> = (0..5u8).flat_map(|a| (a + 1..5).map(move |b| (a, b))).collect();
    assert!(in_gktf(5, &k5) && !in_gcb(5, &k5));
    // cacti are in G_CB; K3 is a cycle; K4 is neither a cycle nor bipartite
    assert!(in_gcb(3, &[(0, 1), (1, 2), (0, 2)]));
    assert!(!in_gcb(4, &[(0, 1), (0, 2), (0, 3), (1, 2), (1, 3), (2, 3)]));
    // K(2,3) is in G_CB; K(2,3) plus the edge inside the side of three is not (the block has a triangle)
    assert!(in_gcb(5, &[(0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4)]));
    assert!(!in_gcb(5, &[(0, 2), (0, 3), (0, 4), (1, 2), (1, 3), (1, 4), (2, 3)]));
    // the class of trees and cycles: P4 and C5 yes, the paw (a triangle with a pendant vertex) no
    assert!(in_tree_or_cycle(4, &[(0, 1), (1, 2), (2, 3)]) && in_tree_or_cycle(5, &[(0, 1), (1, 2), (2, 3), (3, 4), (0, 4)]));
    assert!(!in_tree_or_cycle(4, &[(0, 1), (1, 2), (0, 2), (2, 3)]));
}

/// What a scan of every reachable position finds that the paper counts as a failure.
#[derive(Debug, Default)]
struct Scan {
    positions: usize,
    /// reachable positions where the rule has no action
    undefined: usize,
    /// reachable positions where the rule asks for a colour no neighbour has (the paper: the run ends, a failure;
    /// this engine would stay in place)
    absent: usize,
    /// stops anywhere but on the start vertex with every vertex visited
    bad_stops: usize,
}

fn scan(g: &Graph, rule: &Rule) -> Scan {
    let root = Pos { chi: 0, cur: g.s, vis: 1 << g.s, term: false };
    let mut seen: HashSet<Pos> = HashSet::new();
    seen.insert(root);
    let mut todo = vec![root];
    let mut s = Scan::default();
    while let Some(p) = todo.pop() {
        if p.term {
            if !(p.cur == g.s && p.vis == g.full()) {
                s.bad_stops += 1;
            }
            continue;
        }
        let row = row_at(g, p.chi, p.cur);
        let Some(act) = rule.action(&row) else {
            s.undefined += 1;
            continue;
        };
        if act.target != STAY && act.target != STOP && options(g, p.chi, p.cur, act.target).is_empty() {
            s.absent += 1;
        }
        for (_, q) in successors(g, &p, act, true) {
            if seen.insert(q) {
                todo.push(q);
            }
        }
    }
    s.positions = seen.len();
    s
}

/// For every connected graph on up to `max_n` vertices and every start vertex: if the graph is in the class, the
/// algorithm must explore it (exact game) with a clean scan.  Returns, per algorithm, how many (graph, start) pairs
/// were in the class and how many outside it failed.
fn check_classes(max_n: usize) -> [(usize, usize); 3] {
    let algos: [(Formula, fn(usize, &[(u8, u8)]) -> bool); 3] = [(Formula::ATC3, in_tree_or_cycle), (Formula::AC4, in_gcb), (Formula::AUS5, in_gktf)];
    let mut tally = [(0usize, 0usize); 3];
    for n in 1..=max_n {
        for edges in connected_graphs(n) {
            for (i, (f, in_class)) in algos.iter().enumerate() {
                let member = in_class(n, &edges);
                let rule = rule_of(*f);
                if !member && n > 5 {
                    continue; // outside the class only the small graphs are played, to show the class tests are not vacuous
                }
                for start in 0..n {
                    let g = graph(n, start, &edges);
                    let explores = matches!(exact_game(&g, &rule, 20_000_000), Outcome::Explores { .. });
                    if member {
                        assert!(explores, "{} must explore {} from vertex {}", f.paper_algorithm().unwrap().0, edges.iter().map(|(a, b)| format!("{}-{}", a, b)).collect::<Vec<_>>().join(","), start);
                        let sc = scan(&g, &rule);
                        assert_eq!((sc.undefined, sc.absent, sc.bad_stops), (0, 0, 0), "{} on {:?} from {}: {:?}", f.paper_algorithm().unwrap().0, edges, start, sc);
                        tally[i].0 += 1;
                    } else if !explores {
                        tally[i].1 += 1;
                    }
                }
            }
        }
    }
    tally
}

#[test]
fn each_algorithm_explores_every_small_graph_of_its_class_from_every_start() {
    let tally = check_classes(7);
    // how many (graph, start) pairs of each class were checked (graphs of up to seven vertices up to isomorphism, every start vertex): pinned,
    // so a change in the enumeration or in a class test cannot quietly shrink what is verified
    assert_eq!(tally.map(|t| t.0), [167, 776, 1117], "{:?}", tally);
    // and each algorithm fails somewhere outside its class: the class tests are not vacuous.  (For A_TC3 the paper
    // proves it, Theorem 13: three colours cannot explore every subcubic pseudotree.)
    assert!(tally.iter().all(|&(_, outside_failures)| outside_failures > 0), "{:?}", tally);
}

#[test]
fn three_colours_fail_on_some_subcubic_pseudotree_as_theorem_13_says() {
    // Theorem 13: no single rule with three colours explores every subcubic pseudotree (the obstruction family has nine
    // graphs of at most five vertices).  A_TC3 has three colours, so it must fail on one of them.
    let rule = rule_of(Formula::ATC3);
    let mut failures = Vec::new();
    for n in 2..=5 {
        for edges in connected_graphs(n) {
            if is_subcubic_pseudotree(n, &edges) && !in_tree_or_cycle(n, &edges) {
                for start in 0..n {
                    if !matches!(exact_game(&graph(n, start, &edges), &rule, 5_000_000), Outcome::Explores { .. }) {
                        failures.push((n, edges.clone(), start));
                    }
                }
            }
        }
    }
    assert!(!failures.is_empty(), "A_TC3 explores every subcubic pseudotree of at most five vertices, which Theorem 13 rules out");
}

// ---------------------------------------------------------------- through the request protocol
#[test]
fn the_requests_name_the_algorithms_and_their_colours() {
    for (name, k) in [("atc3", 3), ("ac4", 4), ("aus5", 5), ("agen6", 6)] {
        // too few colours is an error that says so, not a silent undefined row
        let few = handle(&format!("cmd exact\nk {}\ngraph 3 0 0-1,1-2\ndefault {}\n", k - 1, name));
        assert!(few.contains("\"status\":\"error\"") && few.contains(&format!("{} needs {} colours", name, k)), "{}: {}", name, few);
        // the paper's model is chosen without being asked, and the algorithm explores a path
        let ok = handle(&format!("cmd exact\nk {}\ngraph 3 0 0-1,1-2\ndefault {}\n", k, name));
        assert!(ok.contains("\"status\":\"explores\"") && ok.contains("\"model\":\"paper\""), "{}: {}", name, ok);
        // classic can still be asked for
        let classic = handle(&format!("cmd exact\nk {}\nmodel classic\ngraph 3 0 0-1,1-2\ndefault {}\n", k, name));
        assert!(classic.contains("\"model\":\"classic\""), "{}: {}", name, classic);
    }
    // more colours than the algorithm needs are left unused
    let more = handle("cmd exact\nk 6\ngraph 4 0 0-1,0-2,1-3\ndefault atc3\n");
    assert!(more.contains("\"status\":\"explores\""), "{}", more);
    // the answer for one row carries the paper's rule number, and a gap in the paper is an undefined row
    let ans = handle("cmd answer\nk 3\nrow 1.001\ndefault atc3\n");
    assert!(ans.contains("\"paint\":2,\"target\":-2") && ans.contains("\"rule\":9"), "{}", ans);
    assert!(handle("cmd answer\nk 3\nrow 1.010\ndefault atc3\n").contains("\"defined\":false"));
    assert!(handle("cmd answer\nk 4\nrow 1.1000\ndefault ac4\n").contains("\"defined\":false"), "fin has no rule in A_C4");
    assert!(handle("cmd answer\nk 5\nrow 3.10000\ndefault aus5\n").contains("\"defined\":false"), "fin has no rule in A_US5");
}

#[test]
fn a_single_vertex_is_explored_by_each_algorithm_in_one_step() {
    for name in ["atc3", "ac4", "aus5"] {
        let (_, k) = Formula::parse(name).unwrap().paper_algorithm().unwrap();
        let out = handle(&format!("cmd exact\nk {}\ngraph 1 0\ndefault {}\n", k, name));
        assert!(out.contains("\"status\":\"explores\"") && out.contains("\"stopped_at\":0"), "{}: {}", name, out);
    }
}
