#![allow(clippy::absurd_extreme_comparisons)]

use std::fs;
use std::path::Path;

const MAX_UNWRAP: usize = 0;
const MAX_EXPECT: usize = 0;
const MAX_PANIC: usize = 0;
const MAX_UNREACHABLE: usize = 0;
const MAX_TODO: usize = 0;
const MAX_UNIMPLEMENTED: usize = 0;

struct SourceFile {
    content: String,
}

fn source_files() -> Vec<SourceFile> {
    let mut files = Vec::new();
    collect_rs_files(Path::new("src"), &mut files);
    files
}

fn collect_rs_files(dir: &Path, out: &mut Vec<SourceFile>) {
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_rs_files(&path, out);
            continue;
        }

        let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
            continue;
        };

        if !name.ends_with(".rs")
            || name.ends_with("_test.rs")
            || name.ends_with(".test.rs")
            || name.ends_with("_tests.rs")
            || name == "openapi_contract_tests.rs"
        {
            continue;
        }

        if let Ok(content) = fs::read_to_string(&path) {
            out.push(SourceFile { content });
        }
    }
}

fn count_in_source(files: &[SourceFile], pattern: &str) -> usize {
    files
        .iter()
        .map(|file| {
            file.content
                .lines()
                .filter(|line| line.contains(pattern))
                .count()
        })
        .sum()
}

#[test]
fn unwrap_budget() {
    let files = source_files();
    let count = count_in_source(&files, ".unwrap()");
    assert!(
        count <= MAX_UNWRAP,
        ".unwrap() budget exceeded: found {count}, max {MAX_UNWRAP}."
    );
}

#[test]
fn expect_budget() {
    let files = source_files();
    let count = count_in_source(&files, ".expect(");
    assert!(
        count <= MAX_EXPECT,
        ".expect( budget exceeded: found {count}, max {MAX_EXPECT}."
    );
}

#[test]
fn panic_budget() {
    let files = source_files();
    let count = count_in_source(&files, "panic!(");
    assert!(
        count <= MAX_PANIC,
        "panic!( budget exceeded: found {count}, max {MAX_PANIC}."
    );
}

#[test]
fn unreachable_budget() {
    let files = source_files();
    let count = count_in_source(&files, "unreachable!(");
    assert!(
        count <= MAX_UNREACHABLE,
        "unreachable!( budget exceeded: found {count}, max {MAX_UNREACHABLE}."
    );
}

#[test]
fn todo_budget() {
    let files = source_files();
    let count = count_in_source(&files, "todo!(");
    assert!(
        count <= MAX_TODO,
        "todo!( budget exceeded: found {count}, max {MAX_TODO}."
    );
}

#[test]
fn unimplemented_budget() {
    let files = source_files();
    let count = count_in_source(&files, "unimplemented!(");
    assert!(
        count <= MAX_UNIMPLEMENTED,
        "unimplemented!( budget exceeded: found {count}, max {MAX_UNIMPLEMENTED}."
    );
}
