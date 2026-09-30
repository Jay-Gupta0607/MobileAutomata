//! The shared fixtures: every `testdata/fixtures/NAME.req` is a request of the line protocol and
//! `NAME.json` the answer it must give.  This test runs each through the engine natively;
//! `testdata/check_wasm_parity.mjs` sends the same requests to the WebAssembly build and compares
//! against the same files.

use std::fs;
use std::path::PathBuf;

fn dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("testdata").join("fixtures")
}

fn normalise(text: &str) -> String {
    text.replace("\r\n", "\n").trim_end().to_string()
}

fn names(ext: &str) -> Vec<String> {
    let mut v: Vec<String> = fs::read_dir(dir())
        .unwrap()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().and_then(|n| n.strip_suffix(ext)).map(str::to_string))
        .collect();
    v.sort();
    v
}

#[test]
fn every_request_has_an_expected_answer_and_the_engine_gives_it() {
    let reqs = names(".req");
    assert_eq!(reqs, names(".json"), "each .req needs a .json and the other way round");
    assert!(reqs.len() >= 20, "the fixtures are missing: {:?}", reqs);
    for name in &reqs {
        let request = fs::read_to_string(dir().join(format!("{}.req", name))).unwrap();
        let expected = normalise(&fs::read_to_string(dir().join(format!("{}.json", name))).unwrap());
        let got = game_wasm::handle(&request);
        assert_eq!(got, expected, "fixture {}", name);
    }
}

#[test]
fn the_fixtures_cover_every_outcome_and_both_models() {
    let all: String = names(".json").iter().map(|n| fs::read_to_string(dir().join(format!("{}.json", n))).unwrap()).collect();
    for status in ["explores", "fails", "undefined", "overflow", "unrefuted", "unfinished", "error", "ok"] {
        assert!(all.contains(&format!("\"status\":\"{}\"", status)), "no fixture with status {}", status);
    }
    for reason in ["never_stops", "stopped_early", "stopped_off_start"] {
        assert!(all.contains(&format!("\"reason\":\"{}\"", reason)), "no fixture with reason {}", reason);
    }
    assert!(all.contains("\"model\":\"paper\"") && all.contains("\"model\":\"classic\""));
}
