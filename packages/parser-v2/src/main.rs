use parser_v2::{
    compare_workspaces, format_impact_terminal, generate_aggregated_impact,
    report::generate_report, Severity, Workspace,
};
use std::fs;
use std::path::PathBuf;
use walkdir::WalkDir;

fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage:");
        eprintln!("  epic-parser-v2 audit <path>                (Audit Solana program/workspace)");
        eprintln!("  epic-parser-v2 <path>                      (Analyze path)");
        eprintln!("  epic-parser-v2 <old_path> <new_path>       (Diff and analyze impact)");
        std::process::exit(1);
    }

    if args[1] == "audit" {
        if args.len() < 3 {
            eprintln!("Usage: epic-parser-v2 audit <path>");
            std::process::exit(1);
        }
        let root = &args[2];
        let diagnostics = parser_v2::audit::run_audit(root)?;
        println!("{}", serde_json::to_string_pretty(&diagnostics)?);
        return Ok(());
    }

    if args.len() >= 3 {
        let old_path = &args[1];
        let new_path = &args[2];
        run_diff_cli(old_path, new_path)?;
    } else {
        let root = PathBuf::from(&args[1]);
        let workspace = build_workspace_from_path(root.to_str().unwrap())?;
        let report = generate_report(&workspace.registry)?;
        println!("{}", serde_json::to_string_pretty(&report)?);
    }

    Ok(())
}

fn build_workspace_from_path(root: &str) -> anyhow::Result<Workspace> {
    let mut workspace = Workspace::new();
    for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
        if entry.path().extension().map_or(false, |ext| ext == "rs") {
            let content = fs::read_to_string(entry.path())?;
            let file_stem = entry
                .path()
                .file_stem()
                .unwrap()
                .to_string_lossy()
                .into_owned();
            workspace.add_file("program", &[&file_stem], &content, Some(&entry.path().to_string_lossy()))?;
        }
    }
    Ok(workspace)
}

fn run_diff_cli(old_path: &str, new_path: &str) -> anyhow::Result<()> {
    let old_ws = build_workspace_from_path(old_path)?;
    let new_ws = build_workspace_from_path(new_path)?;

    let diffs = compare_workspaces(&old_ws, &new_ws);
    let aggregated_impact = generate_aggregated_impact(&diffs);

    // 1. Console report
    let terminal_report = format_impact_terminal("Solana Workspace", &aggregated_impact);
    println!("{}", terminal_report);

    // 2. JSON artifact
    let json_content = serde_json::to_string_pretty(&aggregated_impact)?;
    fs::write("epic-report.json", json_content)?;

    // 3. Pull Request summary markdown
    let mut md_content = String::new();
    md_content.push_str("## EPIC Upgrade Report\n\n");
    md_content.push_str(&format!("Severity: **{}**\n\n", aggregated_impact.severity));
    md_content.push_str("### Risk:\n");
    md_content.push_str(&format!("{}\n\n", aggregated_impact.risk_category));
    md_content.push_str("### Impact:\n");
    for imp in &aggregated_impact.impact {
        md_content.push_str(&format!("- {}\n", imp));
    }
    md_content.push_str("\n### Recommended Actions:\n");
    for rec in &aggregated_impact.recommendations {
        md_content.push_str(&format!("- {}\n", rec));
    }
    md_content.push_str("\n### Deployment Recommendation:\n");
    let recommendation = match aggregated_impact.severity {
        Severity::Critical => "**BLOCK**",
        Severity::Major => "**WARNING** (Major layout changes, verify reallocation/rent)",
        Severity::Minor | Severity::Safe => "**APPROVE**",
    };
    md_content.push_str(&format!("{}\n", recommendation));

    fs::write("epic-report.md", md_content)?;

    // Exit code 1 on Critical findings, exit code 0 otherwise
    if aggregated_impact.severity == Severity::Critical {
        std::process::exit(1);
    } else {
        std::process::exit(0);
    }
}
