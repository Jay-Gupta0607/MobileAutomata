//! Command-line front end of the colour-exploration engine.
//!
//! A thin wrapper: it builds the same request text the browser page sends and calls
//! `game_wasm::handle`, so a native run and a run in the WebAssembly build give the same
//! answer for the same request.  No dependencies.

use std::io::Write;
use std::process::ExitCode;

const USAGE: &str = "\
usage: explore <command> --graph \"n s u-v,u-v,...\" [options]

commands
  exact     search every adversarial execution (the exact game)
  walks     try deterministic and random adversary walks (a failure is proof, success is not)
  play      one execution under an adversary that prefers the vertices given by --prefer
  answer    the rule's action for one row (--row own.bag)

graph
  --graph TEXT        `n s u-v,u-v,...`: n vertices, start s, edges (0-based)
  --graph-file FILE   the same text, read from a file
  --preset NAME       sketch1

rule
  --rule NAME         default rule: sigma, chasewhite, chase3, eat3, flipsweep4, flipsweep4b, flipsweep5,
                      flipsweep5d, sweep(c,t,p), none  (default: none), or an algorithm of the papers:
                        agen6  any graph, 6 colours (Takahashi et al., arXiv 2505.02789)
                        atc3   trees and simple cycles, 3 colours (Hiraoka et al., arXiv 2609.14356)
                        ac4    blocks that are cycles or complete bipartite (cacti), 4 colours (same paper)
                        aus5   blocks that are cliques or triangle-free, 5 colours (same paper)
                      and, with --agents 2 only (the default there):
                        a2c5   any graph, TWO agents, 5 colours (see \"Two agents\" in workbench/README.md)
  --table FILE        rows `own.bag paint>target`, one per line; # starts a comment; target is a
                      colour, stay or stop.  Rows in the table win over --rule
  --agents N          1 (default) or 2.  Two agents see each other and run the built-in rule a2c5 (5 colours, the
                      paper model); tables, --model classic and the walks command are for one agent.  In the
                      answer for two agents a row reads own.bag@p: p is h (the other agent is on this vertex),
                      the colour digit of the neighbour holding it, or - (not next door); e.g. 4.20100@h
  --k N               number of colours, 2 to 6 (default 5; the algorithms of the papers set their own
                      number, which is also the least they accept)
  --model MODEL       classic or paper (default: paper when the rule can stop, else classic)

search
  --cap N             exact only: most positions to enumerate, 1000 to 12000000 (default 1000000)
  --walks N           walks only: random walks after the three deterministic ones, 0 to 100000 (default 64)
  --budget N          walks and play only: most steps, 10 to 5000000 (default 200000)
  --prefer LIST       play only: adversary preference order, e.g. \"1 2 3 0\" (a vertex not listed comes last)
  --row own.bag       answer only: the row to look up, e.g. 3.100100 (two agents: own.bag@p, e.g. 4.11001@h)

An option that does not belong to the command is an error, and so is a value outside its range
(the engine would otherwise adjust it without saying so).

output
  --summary           one line instead of the JSON
  --request           print the request that would be sent and stop
  --request-file FILE send a request of the line protocol exactly as written (no command or other
                      option needed); the fixtures of testdata/fixtures are such files

exit status
  0 explores    1 fails    2 undefined row    3 overflow (raise --cap)
  4 unrefuted (no failure found by the walks: not a proof)    5 unfinished (budget ran out)
  64 usage error or an error from the engine
";

const SHORT: &str = "usage: explore <exact|walks|play|answer> --graph \"n s u-v,u-v,...\" [options]   (explore --help lists the options)";

/// `n s edges`, with the edge pieces joined: the engine reads only the third whitespace-separated
/// token as the edge list and would silently drop the rest of `0-1, 1-2, 0-2`.
fn normalise_graph(text: &str) -> Result<String, String> {
    let mut it = text.split_whitespace();
    let n = it.next().ok_or("the graph is empty (expected `n s u-v,u-v,...`)")?;
    let s = it.next().ok_or("the graph needs a start vertex (expected `n s u-v,u-v,...`)")?;
    let edges: String = it.collect::<Vec<_>>().concat();
    Ok(if edges.is_empty() { format!("{} {}", n, s) } else { format!("{} {} {}", n, s, edges) })
}

/// A text file the way Windows tools write it: a UTF-8 byte order mark is dropped, and UTF-16 (what
/// Windows PowerShell's `>` writes) is named as the problem instead of failing as "not UTF-8".
fn read_text(path: &str) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {}", path, e))?;
    if bytes.starts_with(&[0xFF, 0xFE]) || bytes.starts_with(&[0xFE, 0xFF]) {
        return Err(format!("{} is UTF-16 text (Windows PowerShell's `>` writes that); save it as UTF-8, for example with `Out-File -Encoding utf8`", path));
    }
    let body = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(&bytes);
    String::from_utf8(body.to_vec()).map_err(|_| format!("{} is not valid UTF-8 text", path))
}

/// The preset graphs (same texts as the browser page).
fn preset(name: &str) -> Option<&'static str> {
    match name {
        "sketch1" => Some("17 0 0-1,1-2,0-2,0-3,2-3,0-4,4-5,3-6,4-6,5-6,5-7,6-8,7-8,5-9,9-14,9-15,9-10,10-16,5-10,9-11,10-11,5-11,7-11,11-12,11-13,12-13"),
        _ => None,
    }
}

struct Args {
    command: String,
    graph: Option<String>,
    rule: Option<String>,
    table: Option<String>,
    k: Option<String>,
    model: Option<String>,
    cap: Option<String>,
    walks: Option<String>,
    budget: Option<String>,
    prefer: Option<String>,
    row: Option<String>,
    agents: Option<String>,
    summary: bool,
    request: bool,
    request_file: Option<String>,
}

fn parse_args(argv: &[String]) -> Result<Args, String> {
    let mut a = Args { command: String::new(), graph: None, rule: None, table: None, k: None, model: None, cap: None, walks: None, budget: None, prefer: None, row: None, agents: None, summary: false, request: false, request_file: None };
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        let mut value = |name: &str| -> Result<String, String> {
            i += 1;
            argv.get(i).cloned().ok_or_else(|| format!("{} needs a value", name))
        };
        match arg {
            "--graph" => a.graph = Some(normalise_graph(&value(arg)?)?),
            "--graph-file" => {
                let path = value(arg)?;
                a.graph = Some(normalise_graph(&read_text(&path)?)?);
            }
            "--preset" => {
                let name = value(arg)?;
                a.graph = Some(preset(&name).ok_or_else(|| format!("unknown preset {} (sketch1)", name))?.to_string());
            }
            "--rule" => a.rule = Some(value(arg)?),
            "--table" => {
                let path = value(arg)?;
                a.table = Some(read_text(&path)?);
            }
            "--k" => a.k = Some(value(arg)?),
            "--model" => a.model = Some(value(arg)?),
            "--cap" => a.cap = Some(value(arg)?),
            "--walks" => a.walks = Some(value(arg)?),
            "--budget" => a.budget = Some(value(arg)?),
            "--prefer" => a.prefer = Some(value(arg)?),
            "--row" => a.row = Some(value(arg)?),
            "--agents" => a.agents = Some(value(arg)?),
            "--summary" => a.summary = true,
            "--request" => a.request = true,
            "--request-file" => a.request_file = Some(value(arg)?),
            "-h" | "--help" => return Err(String::new()),
            _ if arg.starts_with("--") => return Err(format!("unknown option {}", arg)),
            _ if a.command.is_empty() => a.command = arg.to_string(),
            _ => return Err(format!("unexpected argument {}", arg)),
        }
        i += 1;
    }
    if a.request_file.is_some() {
        // the file is sent as written, so an option that would change the request would be ignored without a word
        let others = a.graph.is_some() || a.rule.is_some() || a.table.is_some() || a.k.is_some() || a.model.is_some() || a.cap.is_some() || a.walks.is_some() || a.budget.is_some() || a.prefer.is_some() || a.row.is_some() || a.agents.is_some();
        return if a.command.is_empty() && !others { Ok(a) } else { Err("--request-file is used alone: the file holds the whole request (only --summary and --request go with it)".into()) };
    }
    match a.command.as_str() {
        "exact" | "walks" | "play" | "answer" => Ok(a),
        "" => Err("no command given".into()),
        other => Err(format!("unknown command {} (exact, walks, play, answer)", other)),
    }
}

/// A number option must be digits only and inside `lo..=hi`: anything else would silently fall back
/// to the engine's default or be clamped to its limits.
fn number(name: &str, v: &str, lo: u64, hi: u64) -> Result<String, String> {
    if v.is_empty() || !v.bytes().all(|b| b.is_ascii_digit()) {
        return Err(format!("{} must be a whole number, got {:?}", name, v));
    }
    match v.parse::<u64>() {
        Ok(n) if (lo..=hi).contains(&n) => Ok(n.to_string()),
        _ => Err(format!("{} must be between {} and {}, got {}", name, lo, hi, v)),
    }
}

/// A value that goes into the request as it is (rule, model, row) must be one line: a line break
/// would add protocol lines of its own (`sigma\nmodel classic`).
fn one_line(name: &str, v: &str) -> Result<(), String> {
    if v.chars().any(|c| c.is_control()) {
        return Err(format!("{} must be a single line without control characters", name));
    }
    Ok(())
}

/// An option given for a command that does not read it would have no effect, and no warning.
fn only_for(a: &Args, option: &str, present: bool, commands: &[&str]) -> Result<(), String> {
    if present && !commands.contains(&a.command.as_str()) {
        return Err(format!("{} does not apply to `{}` (it is for {})", option, a.command, commands.join(" and ")));
    }
    Ok(())
}

/// The request text of the engine's line protocol.
fn build_request(a: &Args) -> Result<String, String> {
    only_for(a, "--cap", a.cap.is_some(), &["exact"])?;
    only_for(a, "--walks", a.walks.is_some(), &["walks"])?;
    only_for(a, "--budget", a.budget.is_some(), &["walks", "play"])?;
    only_for(a, "--prefer", a.prefer.is_some(), &["play"])?;
    only_for(a, "--row", a.row.is_some(), &["answer"])?;
    if a.command == "answer" && a.graph.is_some() {
        return Err("`answer` looks up one row and takes no graph".into());
    }
    let agents: u8 = match &a.agents {
        Some(v) => number("--agents", v, 1, 2)?.parse().unwrap(),
        None => 1,
    };
    if agents == 2 {
        if a.table.is_some() {
            return Err("--table is for one agent: two agents run the built-in rule a2c5".into());
        }
        if a.command == "walks" {
            return Err("`walks` is for one agent: with two agents use `exact` or `play`".into());
        }
        if let Some(rule) = &a.rule {
            if rule != "a2c5" {
                return Err(format!("two agents run the rule a2c5, not {}", rule));
            }
        }
    } else if a.rule.as_deref() == Some("a2c5") {
        return Err("a2c5 is a two-agent rule: add --agents 2".into());
    }
    let mut r = format!("cmd {}\n", a.command);
    if agents == 2 {
        r += "agents 2\ndefault a2c5\n";
        if a.k.is_none() {
            r += "k 5\n";
        }
    }
    if let Some(k) = &a.k {
        r += &format!("k {}\n", number("--k", k, 2, 6)?);
    } else if let Some((_, k)) = a.rule.as_deref().and_then(game_wasm::Formula::parse).and_then(|f| f.paper_algorithm()) {
        r += &format!("k {}\n", k); // an algorithm of the papers brings its own number of colours
    }
    if a.command != "answer" {
        let g = a.graph.as_deref().ok_or("--graph (or --graph-file, --preset) is required")?;
        r += &format!("graph {}\n", g);
    }
    if let (Some(rule), 1) = (&a.rule, agents) {
        one_line("--rule", rule)?;
        r += &format!("default {}\n", rule);
    }
    if let Some(m) = &a.model {
        one_line("--model", m)?;
        r += &format!("model {}\n", m);
    }
    if let Some(v) = &a.cap {
        r += &format!("cap {}\n", number("--cap", v, 1000, 12_000_000)?);
    }
    if let Some(v) = &a.walks {
        r += &format!("walks {}\n", number("--walks", v, 0, 100_000)?);
    }
    if let Some(v) = &a.budget {
        r += &format!("budget {}\n", number("--budget", v, 10, 5_000_000)?);
    }
    if let Some(p) = &a.prefer {
        r += &format!("prefer {}\n", p.split(|c: char| c == ',' || c.is_whitespace()).filter(|x| !x.is_empty()).collect::<Vec<_>>().join(" "));
    }
    if a.command == "answer" {
        let row = a.row.as_deref().ok_or("answer needs --row own.bag")?;
        one_line("--row", row)?;
        r += &format!("row {}\n", row);
    }
    if let Some(t) = &a.table {
        r += "table\n";
        for line in t.lines() {
            let line = line.split('#').next().unwrap_or("").trim();
            if !line.is_empty() {
                r += line;
                r.push('\n');
            }
        }
        r += "end\n";
    }
    Ok(r)
}

/// The string value of `"key":"value"` in the engine's flat JSON (first occurrence).
fn json_str<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\":\"", key);
    let start = json.find(&pat)? + pat.len();
    Some(&json[start..start + json[start..].find('"')?])
}

fn json_raw<'a>(json: &'a str, key: &str) -> Option<&'a str> {
    let pat = format!("\"{}\":", key);
    let start = json.find(&pat)? + pat.len();
    let rest = &json[start..];
    Some(&rest[..rest.find(|c| c == ',' || c == '}').unwrap_or(rest.len())])
}

fn exit_code(status: &str) -> u8 {
    match status {
        "explores" | "ok" => 0,
        "fails" => 1,
        "undefined" => 2,
        "overflow" => 3,
        "unrefuted" => 4,
        "unfinished" => 5,
        _ => 64,
    }
}

fn summary(json: &str) -> String {
    let status = json_str(json, "status").unwrap_or("?");
    if status == "error" {
        return format!("error: {}", json_str(json, "message").unwrap_or("?"));
    }
    let mut s = status.to_string();
    if let Some(m) = json_str(json, "model") {
        s += &format!(" ({} model{})", m, if json_raw(json, "agents") == Some("2") { ", 2 agents" } else { "" });
    }
    if let Some(reason) = json_str(json, "reason") {
        s += &format!(", {}", reason.replace('_', " "));
    }
    if let Some(v) = json_raw(json, "positions") {
        if v != "0" {
            s += &format!(", {} positions", v);
        }
    }
    if let Some(v) = json_raw(json, "walks") {
        s += &format!(", {} walks", v);
    }
    if let Some(v) = json_raw(json, "stopped_at") {
        if v != "null" {
            s += &format!(", stopped on vertex {}", v);
        }
    }
    if let Some(row) = json_str(json, "row") {
        if status == "undefined" {
            s += &format!(", no action for row {}", row);
        }
    }
    s
}

/// Writes to stdout without panicking: a reader that has gone away (`explore ... | head`) is not an
/// error worth hiding the outcome's exit status for; any other write failure is reported.
fn emit(text: &str) {
    let mut out = std::io::stdout().lock();
    if let Err(e) = out.write_all(text.as_bytes()).and_then(|_| out.flush()) {
        if e.kind() != std::io::ErrorKind::BrokenPipe {
            eprintln!("explore: cannot write the answer: {}", e);
        }
    }
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = match parse_args(&argv) {
        Ok(a) => a,
        Err(msg) => {
            if msg.is_empty() {
                emit(USAGE);
                return ExitCode::SUCCESS;
            }
            eprintln!("explore: {}\n{}", msg, SHORT);
            return ExitCode::from(64);
        }
    };
    let built = match &args.request_file {
        Some(path) => read_text(path),
        None => build_request(&args),
    };
    let request = match built {
        Ok(r) => r,
        Err(msg) => {
            eprintln!("explore: {}\n{}", msg, SHORT);
            return ExitCode::from(64);
        }
    };
    if args.request {
        emit(&request);
        return ExitCode::SUCCESS;
    }
    let json = game_wasm::handle(&request);
    if args.summary {
        emit(&format!("{}\n", summary(&json)));
    } else {
        emit(&format!("{}\n", json));
    }
    ExitCode::from(exit_code(json_str(&json, "status").unwrap_or("error")))
}
