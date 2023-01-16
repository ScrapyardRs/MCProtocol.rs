use serde_derive::{Deserialize, Serialize};
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

fn main() {
    let input = Path::new("./input");
    let output = Path::new("./output");

    block_registry(input.to_path_buf(), output.to_path_buf());
}

#[derive(Serialize, Deserialize)]
struct RegistryItem {
    key: String,
    idx: i32,
}

pub fn block_registry(mut input: PathBuf, mut output: PathBuf) {
    input.push("blocks");
    let input = File::open(input).unwrap();
    let reader = BufReader::new(input);
    let mut items = vec![];
    for (idx, line) in reader.lines().enumerate() {
        let line = line.unwrap();
        if line.is_empty() {
            break;
        }
        let mut chars = line.chars().skip("public static final Block ".len());
        let mut builder = String::new();
        while let Some(next) = chars.next() {
            if next == ' ' {
                break;
            }
            builder.push(next);
        }
        let name = builder;
        let mut chars = chars.skip("= Blocks.register(\"".len());
        let mut builder = String::new();
        while let Some(next) = chars.next() {
            if next == '"' {
                break;
            }
            builder.push(next);
        }
        let key = builder;
        items.push(RegistryItem {
            key,
            idx: idx as i32,
        });
    }
    output.push("blocks.json");

    if output.exists() {
        std::fs::remove_file(output.clone()).unwrap();
    }

    let mut output = File::create(output).unwrap();
    serde_json::to_writer_pretty(&mut output, &items).unwrap();
}
