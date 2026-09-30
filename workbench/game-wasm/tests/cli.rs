//! The command-line tool `explore`: exit statuses, output shapes and errors.
//! The tool calls the same `handle` as the WebAssembly build, so these also pin the request it builds.

use std::path::PathBuf;
use std::process::{Command, Output};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_explore")).args(args).output().expect("the explore binary runs")
}
fn stdout(o: &Output) -> String {
    String::from_utf8_lossy(&o.stdout).into_owned()
}
fn stderr(o: &Output) -> String {
    String::from_utf8_lossy(&o.stderr).into_owned()
}
fn code(o: &Output) -> i32 {
    o.status.code().expect("exited normally")
}

/// A temporary file that removes itself.
struct Temp(PathBuf);
impl Temp {
    fn new(name: &str, text: &str) -> Temp {
        let path = std::env::temp_dir().join(format!("explore-test-{}-{}", std::process::id(), name));
        std::fs::write(&path, text).unwrap();
        Temp(path)
    }
    fn path(&self) -> &str {
        self.0.to_str().unwrap()
    }
}
impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

const TREE: &str = "4 0 0-1,0-2,1-3";
const PATH4: &str = "4 0 0-1,1-2,2-3";

#[test]
fn play_reproduces_a_hand_trace_and_exits_zero() {
    let o = run(&["play", "--graph", TREE, "--rule", "agen6", "--prefer", "1 2 3 0"]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let json = stdout(&o);
    assert!(json.contains("\"status\":\"explores\"") && json.contains("\"model\":\"paper\""), "{}", json);
    assert_eq!(json.matches("{\"cur\":").count(), 24, "the tree trace of the hand-trace workbook has 24 actions");
    assert!(json.contains("\"stopped_at\":0"), "{}", json);
}

#[test]
fn exact_search_of_agen6_exits_zero_and_is_repeatable() {
    let a = run(&["exact", "--graph", TREE, "--rule", "agen6"]);
    let b = run(&["exact", "--graph", TREE, "--rule", "agen6"]);
    assert_eq!(code(&a), 0);
    assert!(stdout(&a).contains("\"status\":\"explores\""));
    assert_eq!(stdout(&a), stdout(&b), "the same request gives the same answer");
}

#[test]
fn exit_statuses_follow_the_outcome() {
    // 1 fails: a table that stops on vertex 1
    let wrong = Temp::new("wrong.txt", "# stops on the wrong vertex\n0.10 1>0   # walk to vertex 1\n0.01 0>stop\n");
    let o = run(&["exact", "--graph", "2 0 0-1", "--k", "2", "--table", wrong.path()]);
    assert_eq!(code(&o), 1, "{}", stdout(&o));
    assert!(stdout(&o).contains("\"reason\":\"stopped_off_start\""));
    // 2 undefined: no rule at all
    assert_eq!(code(&run(&["play", "--graph", "2 0 0-1", "--k", "2"])), 2);
    // 3 overflow: sigma* on Sketch I needs far more than the smallest cap
    assert_eq!(code(&run(&["exact", "--preset", "sketch1", "--rule", "sigma", "--cap", "1000"])), 3);
    // 4 unrefuted: every walk explores, which is not a proof
    assert_eq!(code(&run(&["walks", "--graph", PATH4, "--rule", "agen6"])), 4);
    // 5 unfinished: the budget runs out
    assert_eq!(code(&run(&["play", "--graph", PATH4, "--rule", "sigma", "--model", "paper", "--budget", "10"])), 5);
}

#[test]
fn a_table_file_may_carry_comments_and_blank_lines() {
    let t = Temp::new("table.txt", "\n# header\n0.10 1>0 # go\n\n0.01 0>1\n1.10 1>stop\n");
    let o = run(&["exact", "--graph", "2 0 0-1", "--k", "2", "--table", t.path()]);
    assert_eq!(code(&o), 0, "{}", stdout(&o));
    assert!(stdout(&o).contains("\"stopped_at\":0"));
}

#[test]
fn the_graph_can_come_from_a_file_or_a_preset() {
    let g = Temp::new("graph.txt", "4 0\n0-1,0-2,\n1-3\n");
    let o = run(&["exact", "--graph-file", g.path(), "--rule", "agen6"]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let p = run(&["walks", "--preset", "sketch1", "--rule", "sigma", "--k", "5", "--walks", "4"]);
    assert!(matches!(code(&p), 0 | 1 | 4), "{}", stdout(&p));
}

#[test]
fn summary_is_one_line() {
    let o = run(&["exact", "--graph", TREE, "--rule", "agen6", "--summary"]);
    let text = stdout(&o);
    assert_eq!(text.lines().count(), 1, "{}", text);
    assert!(text.starts_with("explores (paper model)") && text.contains("stopped on vertex 0"), "{}", text);
    let bad = run(&["exact", "--graph", "2 0 0-1", "--k", "2", "--rule", "agen6", "--summary"]);
    assert_eq!(code(&bad), 64);
    assert!(stdout(&bad).starts_with("error: agen6 needs 6 colours"), "{}", stdout(&bad));
}

#[test]
fn request_shows_what_would_be_sent_without_running_it() {
    let o = run(&["play", "--graph", TREE, "--rule", "agen6", "--prefer", "1,2,3,0", "--request"]);
    assert_eq!(code(&o), 0);
    let text = stdout(&o);
    for line in ["cmd play", "k 6", "graph 4 0 0-1,0-2,1-3", "default agen6", "prefer 1 2 3 0"] {
        assert!(text.lines().any(|l| l == line), "missing {:?} in {:?}", line, text);
    }
    assert!(!text.contains("status"), "nothing is run");
    // edge pieces separated by spaces (or split over lines) are joined, not silently dropped
    let spaced = run(&["exact", "--graph", "3 0 0-1, 1-2, 0-2", "--rule", "agen6", "--request"]);
    assert!(stdout(&spaced).lines().any(|l| l == "graph 3 0 0-1,1-2,0-2"), "{}", stdout(&spaced));
    assert_eq!(code(&run(&["exact", "--graph", "1 0", "--rule", "agen6"])), 0, "a single vertex has no edges");
}

#[test]
fn answer_looks_up_one_row() {
    let o = run(&["answer", "--rule", "agen6", "--row", "4.001000"]);
    assert_eq!(code(&o), 0, "{}", stderr(&o));
    let json = stdout(&o);
    assert!(json.contains("\"paint\":2") && json.contains("\"target\":-2"), "{}", json);
    assert_eq!(code(&run(&["answer", "--rule", "agen6"])), 64, "--row is required");
}

#[test]
fn usage_errors_exit_64_and_explain() {
    let cases: [&[&str]; 7] = [
        &[],
        &["frobnicate"],
        &["exact", "--rule", "agen6"],
        &["exact", "--graph", TREE, "--bogus"],
        &["exact", "--graph", TREE, "--cap", "many"],
        &["exact", "--graph", TREE, "--k"],
        &["exact", "--preset", "nowhere"],
    ];
    for args in cases {
        let o = run(args);
        assert_eq!(code(&o), 64, "{:?}", args);
        assert!(stderr(&o).contains("explore:") && stderr(&o).contains("usage:"), "{:?}: {}", args, stderr(&o));
    }
    // an engine error is reported too
    let o = run(&["exact", "--graph", "3 0 0-1", "--rule", "agen6"]);
    assert_eq!(code(&o), 64, "{}", stdout(&o));
    assert!(stdout(&o).contains("\"status\":\"error\""));
    // help is not an error
    let h = run(&["--help"]);
    assert_eq!(code(&h), 0);
    assert!(stdout(&h).contains("usage:") && stdout(&h).contains("exit status"));
}
