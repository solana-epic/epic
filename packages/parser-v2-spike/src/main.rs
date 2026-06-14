use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use parser_v2_spike::{parse_source, WorkspaceAnalysis};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: parser-spike <path>");
        std::process::exit(1);
    }

    let root = PathBuf::from(&args[1]);
    let mut final_analysis = WorkspaceAnalysis::default();

    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(entry.path())?;
            if let Ok(analysis) = parse_source(&content) {
                final_analysis.accounts.extend(analysis.accounts);
                final_analysis.structs.extend(analysis.structs);
                final_analysis.aliases.extend(analysis.aliases);
            }
        }
    }

    println!("{}", serde_json::to_string_pretty(&final_analysis)?);
    Ok(())
}
