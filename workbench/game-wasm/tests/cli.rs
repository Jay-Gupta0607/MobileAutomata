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
    fn bytes(name: &str, bytes: &[u8]) -> Temp {
        let path = std::env::temp_dir().join(format!("explore-test-{}-{}", std::process::id(), name));
        std::fs::write(&path, bytes).unwrap();
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

#[test]
fn a_request_file_is_sent_exactly_as_written() {
    let req = "cmd play\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\nprefer 0 1 2\n";
    let f = Temp::new("request.req", req);
    let via_file = run(&["--request-file", f.path()]);
    let via_args = run(&["play", "--graph", "3 0 0-1,1-2", "--rule", "agen6", "--prefer", "0 1 2"]);
    assert_eq!(code(&via_file), 0, "{}", stderr(&via_file));
    assert_eq!(stdout(&via_file), stdout(&via_args), "the same request, the same answer");
    // it takes no command, and a missing file is a usage error
    assert_eq!(code(&run(&["play", "--request-file", f.path()])), 64);
    assert_eq!(code(&run(&["--request-file", "no-such-file.req"])), 64);
}

#[test]
fn prefer_must_be_vertex_numbers() {
    let o = run(&["play", "--graph", TREE, "--rule", "agen6", "--prefer", "1 two 0"]);
    assert_eq!(code(&o), 64, "{}", stdout(&o));
    assert!(stdout(&o).contains("\"two\"") || stdout(&o).contains("two is not a vertex number"), "{}", stdout(&o));
    // commas work as separators, a vertex outside the graph is still an error
    assert_eq!(code(&run(&["play", "--graph", TREE, "--rule", "agen6", "--prefer", "1,2,3,0"])), 0);
    assert_eq!(code(&run(&["play", "--graph", TREE, "--rule", "agen6", "--prefer", "1,9"])), 64);
}

#[test]
fn files_written_by_windows_tools_are_read_or_explained() {
    let bom = |text: &str| -> Vec<u8> { [&[0xEF, 0xBB, 0xBF][..], text.as_bytes()].concat() };
    // UTF-8 with a byte order mark (Notepad, `Out-File -Encoding utf8`) works for all three file options
    let g = Temp::bytes("bom-graph.txt", &bom("4 0 0-1,0-2,1-3"));
    assert_eq!(code(&run(&["exact", "--graph-file", g.path(), "--rule", "agen6"])), 0);
    let t = Temp::bytes("bom-table.txt", &bom("0.10 1>0\n0.01 0>1\n1.10 1>stop\n"));
    assert_eq!(code(&run(&["exact", "--graph", "2 0 0-1", "--k", "2", "--table", t.path()])), 0);
    let r = Temp::bytes("bom.req", &bom("cmd exact\nk 6\ngraph 3 0 0-1,1-2\ndefault agen6\n"));
    assert_eq!(code(&run(&["--request-file", r.path()])), 0);
    // UTF-16 (Windows PowerShell's `>`) is named, with the way out
    let utf16: Vec<u8> = [&[0xFF, 0xFE][..], &"4 0 0-1".encode_utf16().flat_map(|u| u.to_le_bytes()).collect::<Vec<u8>>()].concat();
    let u = Temp::bytes("utf16.txt", &utf16);
    let o = run(&["exact", "--graph-file", u.path(), "--rule", "agen6"]);
    assert_eq!(code(&o), 64);
    assert!(stderr(&o).contains("UTF-16") && stderr(&o).contains("Out-File -Encoding utf8"), "{}", stderr(&o));
    // anything else that is not UTF-8 is said so
    let bad = Temp::bytes("latin1.txt", &[0x34, 0x20, 0xC3, 0x28]);
    let o = run(&["exact", "--graph-file", bad.path(), "--rule", "agen6"]);
    assert_eq!(code(&o), 64);
    assert!(stderr(&o).contains("not valid UTF-8"), "{}", stderr(&o));
}

#[test]
fn options_that_do_not_belong_to_the_command_are_errors() {
    let cases: [&[&str]; 6] = [
        &["exact", "--graph", TREE, "--rule", "agen6", "--walks", "5"],
        &["exact", "--graph", TREE, "--rule", "agen6", "--budget", "100"],
        &["play", "--graph", TREE, "--rule", "agen6", "--cap", "2000"],
        &["walks", "--graph", TREE, "--rule", "agen6", "--prefer", "1"],
        &["exact", "--graph", TREE, "--rule", "agen6", "--row", "4.001000"],
        &["answer", "--graph", TREE, "--rule", "agen6", "--row", "4.001000"],
    ];
    for args in cases {
        let o = run(args);
        assert_eq!(code(&o), 64, "{:?}", args);
        assert!(stderr(&o).contains("does not apply") || stderr(&o).contains("takes no graph"), "{:?}: {}", args, stderr(&o));
    }
    // each option is still accepted by the commands that read it
    assert_eq!(code(&run(&["exact", "--graph", TREE, "--rule", "agen6", "--cap", "5000"])), 0);
    assert_eq!(code(&run(&["walks", "--graph", TREE, "--rule", "agen6", "--walks", "3", "--budget", "500"])), 4);
    assert_eq!(code(&run(&["play", "--graph", TREE, "--rule", "agen6", "--budget", "500"])), 0);
}

#[test]
fn numbers_outside_the_engines_limits_are_errors_not_adjusted() {
    let bad: [&[&str]; 6] = [
        &["exact", "--graph", TREE, "--rule", "agen6", "--cap", "999"],
        &["exact", "--graph", TREE, "--rule", "agen6", "--cap", "12000001"],
        &["walks", "--graph", TREE, "--rule", "agen6", "--walks", "100001"],
        &["play", "--graph", TREE, "--rule", "agen6", "--budget", "9"],
        &["walks", "--graph", TREE, "--rule", "agen6", "--budget", "5000001"],
        &["exact", "--graph", TREE, "--rule", "agen6", "--k", "7"],
    ];
    for args in bad {
        let o = run(args);
        assert_eq!(code(&o), 64, "{:?}", args);
        assert!(stderr(&o).contains("must be between"), "{:?}: {}", args, stderr(&o));
    }
    // the limits themselves are fine
    assert_eq!(code(&run(&["exact", "--graph", TREE, "--rule", "agen6", "--cap", "1000"])), 0);
    assert_eq!(code(&run(&["play", "--graph", TREE, "--rule", "agen6", "--budget", "10"])), 5);
    assert_eq!(code(&run(&["walks", "--graph", TREE, "--rule", "agen6", "--walks", "0"])), 4);
}

#[test]
fn a_closed_pipe_does_not_hide_the_outcome() {
    // `explore ... | head -c 0`: the reader goes away before the answer is written.  The exit status is
    // still the outcome's (0, explores), not a panic (101) from a failed print.
    let mut child = Command::new(env!("CARGO_BIN_EXE_explore"))
        .args(["exact", "--preset", "sketch1", "--rule", "sigma", "--cap", "3000000"])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("the explore binary runs");
    drop(child.stdout.take());
    let out = child.wait_with_output().unwrap();
    assert_eq!(out.status.code(), Some(0), "stderr: {}", String::from_utf8_lossy(&out.stderr));
    assert!(!String::from_utf8_lossy(&out.stderr).contains("panicked"), "{}", String::from_utf8_lossy(&out.stderr));
}

#[test]
fn a_line_break_in_an_option_cannot_add_request_lines() {
    // without the check `--rule "sigma\nmodel paper"` sends two protocol lines
    let o = run(&["exact", "--graph", PATH4, "--rule", "sigma\nmodel paper", "--request"]);
    assert_eq!(code(&o), 64, "it printed the request: {}", stdout(&o));
    assert!(stderr(&o).contains("--rule") && stderr(&o).contains("single line"), "{}", stderr(&o));
    for args in [
        &["exact", "--graph", PATH4, "--rule", "sigma", "--model", "classic\ncap 5"][..],
        &["answer", "--rule", "agen6", "--row", "4.001000\ncmd exact"][..],
        &["exact", "--graph", PATH4, "--rule", "sigma\r\ncap 5"][..],
        &["exact", "--graph", PATH4, "--rule", "sig\tma"][..],
    ] {
        let o = run(args);
        assert_eq!(code(&o), 64, "{:?}", args);
        assert!(stderr(&o).contains("single line"), "{:?}: {}", args, stderr(&o));
    }
    // ordinary values, including a rule with parentheses and commas, are untouched
    let ok = run(&["exact", "--graph", PATH4, "--rule", "sweep(0,4,0)", "--request"]);
    assert_eq!(code(&ok), 0);
    assert!(stdout(&ok).lines().any(|l| l == "default sweep(0,4,0)"), "{}", stdout(&ok));
}
