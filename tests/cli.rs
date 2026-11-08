//! End-to-end tests for `moose-perfgraph`.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

const SAMPLE: &str = "\
Some MOOSE simulation output\n\
\n\
Performance Graph:\n\
------------------------------------------------------------------------------------------------------------\n\
| Section            | Calls | Self      | Avg.  | %    | Memory | Total     | Avg.  | %    | Memory |\n\
------------------------------------------------------------------------------------------------------------\n\
| ROOT               | 1     | 0.001     | 0.001 | 0.0  | 0.0    | 3.100     | 3.100 | 100.0| 1.7    |\n\
|   Solve            | 1     | 1.500     | 1.500 | 50.0 | 1.0    | 2.900     | 2.900 | 96.7 | 1.6    |\n\
|     Compute Jacobian | 10  | 0.800     | 0.080 | 26.7 | 0.5    | 0.800     | 0.080 | 26.7 | 0.5    |\n\
|     Compute Residual | 10  | 0.400     | 0.040 | 13.3 | 0.2    | 0.400     | 0.040 | 13.3 | 0.2    |\n\
------------------------------------------------------------------------------------------------------------\n\
\n\
Heaviest Sections:\n\
--------------------------------------------------------------------------------\n\
| Section            | Calls | Self(s)   | Avg.  | %    | Mem(MB) |\n\
--------------------------------------------------------------------------------\n\
| Solve              | 1     | 1.500     | 1.500 | 50.0 | 1.0     |\n\
| Compute Jacobian   | 10    | 0.800     | 0.080 | 26.7 | 0.5     |\n\
| Compute Residual   | 10    | 0.400     | 0.040 | 13.3 | 0.2     |\n\
--------------------------------------------------------------------------------\n\
";

fn fixture(name: &str) -> PathBuf {
    let path = std::env::temp_dir().join(format!("perfgraph-{name}-{}.log", std::process::id()));
    fs::write(&path, SAMPLE).expect("write fixture");
    path
}

fn run(path: &PathBuf, extra: &[&str]) -> (bool, String) {
    let output = Command::new(env!("CARGO_BIN_EXE_moose-perfgraph"))
        .arg(path)
        .args(extra)
        .output()
        .expect("run binary");
    (
        output.status.success(),
        String::from_utf8(output.stdout).expect("utf8"),
    )
}

#[test]
fn ranks_sections_by_self_and_total_time() {
    let path = fixture("rank");
    let (ok, stdout) = run(&path, &[]);
    assert!(ok, "{stdout}");
    let self_pos = stdout.find("top by self time:").expect("self ranking");
    let total_pos = stdout.find("top by total time:").expect("total ranking");
    let self_section = &stdout[self_pos..total_pos];
    assert!(
        self_section.find("Solve:").expect("solve")
            < self_section.find("Compute Jacobian:").expect("jac")
    );
    let total_section = &stdout[total_pos..];
    assert!(total_section.contains("Solve: self=1.500s"), "{stdout}");
    assert!(stdout.contains("Compute Residual: self=0.400s"), "{stdout}");
    fs::remove_file(&path).ok();
}

#[test]
fn name_and_min_self_filters_apply() {
    let path = fixture("filter");
    let (ok, stdout) = run(&path, &["--name", "jacobian"]);
    assert!(ok);
    assert!(stdout.contains("Compute Jacobian"), "{stdout}");
    assert!(!stdout.contains("Compute Residual"), "{stdout}");

    let (_, stdout) = run(&path, &["--min-self", "1.0"]);
    assert!(stdout.contains("Solve"), "{stdout}");
    assert!(!stdout.contains("Compute Jacobian"), "{stdout}");
    fs::remove_file(&path).ok();
}

#[test]
fn no_perf_data_exits_one() {
    let path = std::env::temp_dir().join(format!("perfgraph-empty-{}.log", std::process::id()));
    fs::write(&path, "just an ordinary log\n").expect("write");
    let output = Command::new(env!("CARGO_BIN_EXE_moose-perfgraph"))
        .arg(&path)
        .output()
        .expect("run binary");
    assert_eq!(output.status.code(), Some(1));
    fs::remove_file(&path).ok();
}
