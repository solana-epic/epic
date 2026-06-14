use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;
use parser_v2::{Workspace, report::generate_report};

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: epic-parser-v2 <path>");
        std::process::exit(1);
    }

    let root = PathBuf::from(&args[1]);
    let mut workspace = Workspace::new();

    for entry in WalkDir::new(&root).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(entry.path())?;
            // A naive way to get program name from path structure. 
            // If it's a program, it might be in `programs/<program_name>/src/lib.rs`.
            // For now, we'll use the file stem as the module path.
            let file_stem = entry.path().file_stem().unwrap().to_string_lossy().into_owned();
            workspace.add_file("program", &[&file_stem], &content)?;
        }
    }

    let report = generate_report(&workspace.registry)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    
    Ok(())
}
