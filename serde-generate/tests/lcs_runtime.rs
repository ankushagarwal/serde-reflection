// Copyright (c) Facebook, Inc. and its affiliates
// SPDX-License-Identifier: MIT OR Apache-2.0

use libra_canonical_serialization as lcs;
use serde::{Deserialize, Serialize};
use serde_generate::{python3, rust, test_utils};
use serde_reflection::{Registry, Result, Samples, Tracer, TracerConfig};
use std::fs::File;
use std::io::Write;
use std::process::Command;
use tempfile::tempdir;

#[derive(Serialize, Deserialize)]
struct Test {
    a: Vec<u64>,
    b: (u32, u32),
    c: Choice,
}

#[derive(Serialize, Deserialize)]
enum Choice {
    A,
    B(u64),
    C { x: u8 },
}

fn get_local_registry() -> Result<Registry> {
    let mut tracer = Tracer::new(TracerConfig::default());
    let samples = Samples::new();
    tracer.trace_type::<Test>(&samples)?;
    tracer.trace_type::<Choice>(&samples)?;
    Ok(tracer.registry()?)
}

#[test]
fn test_python_lcs_runtime_on_simple_data() {
    let registry = get_local_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();
    python3::output(&mut source, &registry).unwrap();

    let reference = lcs::to_bytes(&Test {
        a: vec![4, 6],
        b: (3, 5),
        c: Choice::C { x: 7 },
    })
    .unwrap();
    writeln!(
        source,
        r#"
import lcs

value = Test([4, 6], (3, 5), Choice.C(7))

s = lcs.serialize(value, Test)
assert s == bytes.fromhex("{}")

v, buffer = lcs.deserialize(s, Test)
assert len(buffer) == 0
assert v == value
assert v.c.x == 7

v.b = (3, 0)
t = lcs.serialize(v, Test)
assert len(t) == len(s)
assert t != s
"#,
        hex::encode(&reference),
    )
    .unwrap();

    let python_path = std::env::var("PYTHONPATH").unwrap_or_default() + ":runtime/python";
    let output = Command::new("python3")
        .arg(source_path)
        .env("PYTHONPATH", python_path)
        .output()
        .unwrap();

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();
    assert_eq!(String::new(), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
}

#[test]
fn test_python_lcs_runtime_on_all_supported_types() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    let source_path = dir.path().join("test.py");
    let mut source = File::create(&source_path).unwrap();
    python3::output(&mut source, &registry).unwrap();

    let values = test_utils::get_sample_values();
    let hex_encodings: Vec<_> = values
        .iter()
        .map(|v| format!("'{}'", hex::encode(&lcs::to_bytes(&v).unwrap())))
        .collect();

    writeln!(
        source,
        r#"
import lcs

encodings = [bytes.fromhex(s) for s in [{}]]

for encoding in encodings:
    v, buffer = lcs.deserialize(encoding, SerdeData)
    assert len(buffer) == 0

    s = lcs.serialize(v, SerdeData)
    assert s == encoding
"#,
        hex_encodings.join(", ")
    )
    .unwrap();

    let python_path = format!(
        "{}:runtime/python",
        std::env::var("PYTHONPATH").unwrap_or_default()
    );
    let output = Command::new("python3")
        .arg(source_path)
        .env("PYTHONPATH", python_path)
        .output()
        .unwrap();

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();
    assert_eq!(String::new(), String::from_utf8_lossy(&output.stderr));
    assert!(output.status.success());
}

// Full test using cargo. This may take a while.
#[test]
fn test_rust_lcs_runtime() {
    let registry = test_utils::get_registry().unwrap();
    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "testing2"
version = "0.1.0"
edition = "2018"

[dependencies]
hex = "0.4.2"
serde = { version = "1.0.112", features = ["derive"] }
serde_bytes = "0.11"
libra-canonical-serialization = { git = "https://github.com/libra/libra.git", branch = "testnet" }

[workspace]
"#,
    )
    .unwrap();
    std::fs::create_dir(dir.path().join("src")).unwrap();
    let source_path = dir.path().join("src/main.rs");
    let mut source = File::create(&source_path).unwrap();
    rust::output(&mut source, /* with_derive_macros */ true, &registry).unwrap();

    let values = test_utils::get_sample_values();
    let hex_encodings: Vec<_> = values
        .iter()
        .map(|v| format!("\"{}\"", hex::encode(&lcs::to_bytes(&v).unwrap())))
        .collect();

    writeln!(
        source,
        r#"
use libra_canonical_serialization as lcs;

fn main() {{
    let hex_encodings = vec![{}];

    for hex_encoding in hex_encodings {{
        let encoding = hex::decode(hex_encoding).unwrap();
        let value = lcs::from_bytes::<SerdeData>(&encoding).unwrap();

        let s = lcs::to_bytes(&value).unwrap();
        assert_eq!(s, encoding);
    }}
}}
"#,
        hex_encodings.join(", ")
    )
    .unwrap();

    // Use a stable `target` dir to avoid downloading and recompiling crates everytime.
    let target_dir = std::env::current_dir().unwrap().join("../target");
    let output = Command::new("cargo")
        .current_dir(dir.path())
        .arg("run")
        .arg("--target-dir")
        .arg(target_dir)
        .output()
        .unwrap();

    std::io::stdout().write_all(&output.stdout).unwrap();
    std::io::stderr().write_all(&output.stderr).unwrap();
    assert!(output.status.success());
}
