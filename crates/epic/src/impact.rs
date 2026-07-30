use crate::abi::{ChangeType, DiffResult, Severity};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ImpactAnalysis {
    pub severity: Severity,
    pub risk_category: String,
    pub impact: Vec<String>,
    pub recommendations: Vec<String>,
    pub migration_required: bool,
}

pub fn analyze_impact(diff: &DiffResult) -> ImpactAnalysis {
    let mut impact = Vec::new();
    let mut recommendations = Vec::new();
    let risk_category;
    let migration_required;

    match diff.change_type {
        ChangeType::StructFieldRemoval => {
            risk_category = "Account Deserialization Break".to_string();
            impact.push("Existing PDA accounts incompatible".to_string());
            impact.push("Existing state may become unreadable".to_string());
            recommendations.push("Create migration instruction".to_string());
            recommendations.push("Snapshot affected accounts before upgrade".to_string());
            migration_required = true;
        }
        ChangeType::StructFieldReordering => {
            risk_category = "State Corruption".to_string();
            impact.push("Existing serialized data no longer maps correctly".to_string());
            impact.push(
                "Reading corrupted data will lead to undefined behavior or state locks".to_string(),
            );
            recommendations.push("Do not deploy without migration".to_string());
            recommendations.push("Create manual state migration scripts".to_string());
            migration_required = true;
        }
        ChangeType::StructFieldAddition => {
            if diff.severity == Severity::Critical {
                risk_category = "Layout Shift".to_string();
                impact.push(
                    "Field inserted in middle/front shifts offsets of all subsequent fields"
                        .to_string(),
                );
                impact.push(
                    "Deserializing existing accounts will result in corrupted state".to_string(),
                );
                recommendations
                    .push("Do not insert fields in the middle/front of state structs".to_string());
                recommendations.push("If necessary, append fields at the end or use a new versioned account structure".to_string());
                migration_required = true;
            } else {
                risk_category = "Account Expansion".to_string();
                impact.push("Existing accounts require realloc".to_string());
                impact.push("New serialized layout is longer than previous version".to_string());
                recommendations.push("Use Anchor realloc to expand existing accounts".to_string());
                recommendations.push("Calculate additional rent exemption costs".to_string());
                migration_required = false;
            }
        }
        ChangeType::AccountLayoutChange => {
            risk_category = "Account Size Shift".to_string();
            impact.push("Total byte size of the state account changed".to_string());
            impact.push("Existing accounts must be resized to fit the new layout".to_string());
            recommendations
                .push("Ensure proper realloc constraints are added to instructions".to_string());
            recommendations
                .push("Fund additional rent exemption fees for existing accounts".to_string());
            migration_required = true;
        }
        ChangeType::TypeChange => {
            if diff.severity == Severity::Critical {
                risk_category = "Semantic Type Mismatch".to_string();
                impact.push("Field type changed without size change (e.g. i64 to u64)".to_string());
                impact.push(
                    "Existing data may decode successfully but with incorrect semantic values"
                        .to_string(),
                );
                recommendations.push(
                    "Perform a thorough audit of the affected field's mathematical bounds"
                        .to_string(),
                );
                recommendations.push(
                    "Implement data transformation scripts to validate and convert existing values"
                        .to_string(),
                );
                migration_required = true;
            } else {
                risk_category = "Type Width Mismatch".to_string();
                impact
                    .push("Field type size changed, shifting subsequent field offsets".to_string());
                impact.push("Borsh deserialization will fail on old accounts".to_string());
                recommendations.push(
                    "Write custom migration to adjust account space and transform values"
                        .to_string(),
                );
                recommendations.push("Validate type size alignment before deployment".to_string());
                migration_required = true;
            }
        }
        ChangeType::EnumVariantReordering => {
            risk_category = "Discriminator Corruption".to_string();
            impact.push("Existing enum values may map to different variants".to_string());
            impact.push(
                "On-chain state logic will behave incorrectly due to variant index shifts"
                    .to_string(),
            );
            recommendations.push("Breaking upgrade".to_string());
            recommendations.push("Migration mandatory".to_string());
            migration_required = true;
        }
        ChangeType::EnumVariantRemoval => {
            risk_category = "Enum Variant Deprecation".to_string();
            impact.push(
                "Removing a variant shifts subsequent variant indices in Borsh serialization"
                    .to_string(),
            );
            impact.push("Existing accounts containing the removed variant or subsequent variants will fail to deserialize".to_string());
            recommendations.push("Do not remove enum variants".to_string());
            recommendations.push(
                "Use an 'Unused' or 'Deprecated' placeholder variant to preserve ordering"
                    .to_string(),
            );
            migration_required = true;
        }
        ChangeType::EnumVariantAddition => {
            if diff.severity == Severity::Critical {
                risk_category = "Discriminator Shift".to_string();
                impact.push(
                    "Inserting variant in the middle shifts subsequent variant discriminators"
                        .to_string(),
                );
                impact.push("Existing accounts will map to incorrect variants".to_string());
                recommendations.push("Do not insert enum variants in the middle".to_string());
                recommendations.push(
                    "Only append enum variants at the end of the enum definition".to_string(),
                );
                migration_required = true;
            } else {
                risk_category = "Enum Expansion".to_string();
                impact.push("New enum variant introduced".to_string());
                impact.push("No impact on existing serialized variant indices".to_string());
                recommendations.push("Rebuild clients to support the new variant".to_string());
                recommendations
                    .push("Update matching patterns in program instructions".to_string());
                migration_required = false;
            }
        }
        ChangeType::InstructionRemoval => {
            risk_category = "Client Compatibility Break".to_string();
            impact.push("Existing SDK integrations may fail".to_string());
            impact.push("Clients attempting to call the removed instruction will fail".to_string());
            recommendations.push("Deprecate before removal".to_string());
            recommendations.push(
                "Ensure all client applications have migrated to the new instructions".to_string(),
            );
            migration_required = false;
        }
        ChangeType::InstructionAddition => {
            risk_category = "API Expansion".to_string();
            impact.push("New entry point added to the program".to_string());
            impact.push("No impact on existing instructions or layouts".to_string());
            recommendations.push("Update client SDKs to expose the new instruction".to_string());
            recommendations
                .push("Verify access control constraints on the new instruction".to_string());
            migration_required = false;
        }
        ChangeType::PdaAccountDefinitionChange => {
            risk_category = "PDA Derivation Break".to_string();
            impact.push(
                "Changing validation constraints or seeds changes the derived address".to_string(),
            );
            impact.push("Existing PDA accounts become inaccessible or unreachable".to_string());
            recommendations
                .push("Validate that seed alterations are backwards-compatible".to_string());
            recommendations
                .push("Create fallback address resolution if changing seeds".to_string());
            migration_required = true;
        }
        ChangeType::IdlBreakingChange => {
            risk_category = "IDL Interface Break".to_string();
            impact.push("Serialized interface structure has changed".to_string());
            impact.push("Deserialization of accounts or instructions will fail on clients using the old IDL".to_string());
            recommendations.push("Regenerate and deploy the updated IDL".to_string());
            recommendations
                .push("Rebuild and redeploy all dependent client applications".to_string());
            migration_required = true;
        }
    }

    ImpactAnalysis {
        severity: diff.severity,
        risk_category,
        impact,
        recommendations,
        migration_required,
    }
}

pub fn generate_aggregated_impact(diffs: &[DiffResult]) -> ImpactAnalysis {
    if diffs.is_empty() {
        return ImpactAnalysis {
            severity: Severity::Safe,
            risk_category: "None".to_string(),
            impact: vec![],
            recommendations: vec![],
            migration_required: false,
        };
    }

    let severity = diffs
        .iter()
        .map(|d| d.severity)
        .max()
        .unwrap_or(Severity::Safe);
    let mut risk_cats = Vec::new();
    let mut impact = Vec::new();
    let mut recommendations = Vec::new();
    let mut migration_required = false;

    for d in diffs {
        let single = analyze_impact(d);
        if !risk_cats.contains(&single.risk_category)
            && single.risk_category != "Unknown"
            && single.risk_category != "None"
        {
            risk_cats.push(single.risk_category);
        }
        for imp in single.impact {
            if !impact.contains(&imp) {
                impact.push(imp);
            }
        }
        for rec in single.recommendations {
            if !recommendations.contains(&rec) {
                recommendations.push(rec);
            }
        }
        if single.migration_required {
            migration_required = true;
        }
    }

    let risk_category = if risk_cats.is_empty() {
        "None".to_string()
    } else {
        risk_cats.join(", ")
    };

    ImpactAnalysis {
        severity,
        risk_category,
        impact,
        recommendations,
        migration_required,
    }
}

pub fn format_impact_terminal(program_name: &str, analysis: &ImpactAnalysis) -> String {
    let mut output = Vec::new();
    let header = match analysis.severity {
        Severity::Critical => "CRITICAL UPGRADE WARNING",
        Severity::Major => "MAJOR UPGRADE WARNING",
        Severity::Minor => "MINOR UPGRADE ALERT",
        Severity::Safe => "SAFE UPGRADE CONFIRMED",
    };

    output.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    output.push(header.to_string());
    output.push("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━".to_string());
    output.push(String::new());
    output.push(format!("Program: {}", program_name));
    output.push(String::new());
    output.push("Risk:".to_string());
    output.push(analysis.risk_category.clone());
    output.push(String::new());
    output.push("Impact:".to_string());
    for imp in &analysis.impact {
        output.push(format!("• {}", imp));
    }
    output.push(String::new());
    output.push("Recommended Actions:".to_string());
    for rec in &analysis.recommendations {
        output.push(format!("• {}", rec));
    }
    output.push(String::new());

    output.join("\n").trim().to_string()
}
