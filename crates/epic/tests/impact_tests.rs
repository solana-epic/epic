use epic::{
    analyze_impact, format_impact_terminal, generate_aggregated_impact, ChangeType, DiffResult,
    Severity,
};

#[test]
fn test_all_change_type_mappings() {
    let change_types = vec![
        (
            ChangeType::StructFieldRemoval,
            Severity::Critical,
            "Account Deserialization Break",
            true,
        ),
        (
            ChangeType::StructFieldReordering,
            Severity::Critical,
            "State Corruption",
            true,
        ),
        (
            ChangeType::StructFieldAddition,
            Severity::Critical,
            "Layout Shift",
            true,
        ),
        (
            ChangeType::StructFieldAddition,
            Severity::Minor,
            "Account Expansion",
            false,
        ),
        (
            ChangeType::AccountLayoutChange,
            Severity::Major,
            "Account Size Shift",
            true,
        ),
        (
            ChangeType::TypeChange,
            Severity::Critical,
            "Semantic Type Mismatch",
            true,
        ),
        (
            ChangeType::TypeChange,
            Severity::Major,
            "Type Width Mismatch",
            true,
        ),
        (
            ChangeType::EnumVariantReordering,
            Severity::Critical,
            "Discriminator Corruption",
            true,
        ),
        (
            ChangeType::EnumVariantRemoval,
            Severity::Critical,
            "Enum Variant Deprecation",
            true,
        ),
        (
            ChangeType::EnumVariantAddition,
            Severity::Critical,
            "Discriminator Shift",
            true,
        ),
        (
            ChangeType::EnumVariantAddition,
            Severity::Minor,
            "Enum Expansion",
            false,
        ),
        (
            ChangeType::InstructionRemoval,
            Severity::Critical,
            "Client Compatibility Break",
            false,
        ),
        (
            ChangeType::InstructionAddition,
            Severity::Safe,
            "API Expansion",
            false,
        ),
        (
            ChangeType::PdaAccountDefinitionChange,
            Severity::Critical,
            "PDA Derivation Break",
            true,
        ),
        (
            ChangeType::IdlBreakingChange,
            Severity::Critical,
            "IDL Interface Break",
            true,
        ),
    ];

    for (change_type, severity, expected_risk, expected_migration) in change_types {
        let diff = DiffResult {
            entity: "TestEntity".to_string(),
            change_type,
            severity,
            description: "Test description".to_string(),
        };
        let analysis = analyze_impact(&diff);
        assert_eq!(analysis.severity, severity);
        assert_eq!(analysis.risk_category, expected_risk);
        assert_eq!(analysis.migration_required, expected_migration);
        assert!(
            !analysis.impact.is_empty(),
            "Impact list should not be empty for {:?}",
            diff.change_type
        );
        assert!(
            !analysis.recommendations.is_empty(),
            "Recommendations list should not be empty for {:?}",
            diff.change_type
        );
    }
}

#[test]
fn test_terminal_formatter() {
    let diff = DiffResult {
        entity: "UserConfig".to_string(),
        change_type: ChangeType::StructFieldReordering,
        severity: Severity::Critical,
        description: "Field 'admin' reordered".to_string(),
    };
    let analysis = analyze_impact(&diff);
    let terminal_report = format_impact_terminal("SquadsMultisig", &analysis);

    assert!(terminal_report.contains("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"));
    assert!(terminal_report.contains("CRITICAL UPGRADE WARNING"));
    assert!(terminal_report.contains("Program: SquadsMultisig"));
    assert!(terminal_report.contains("Risk:\nState Corruption"));
    assert!(terminal_report.contains("Impact:"));
    assert!(terminal_report.contains("• Existing serialized data no longer maps correctly"));
}

#[test]
fn test_aggregated_impact_analysis() {
    let diffs = vec![
        DiffResult {
            entity: "User".to_string(),
            change_type: ChangeType::InstructionAddition,
            severity: Severity::Safe,
            description: "Added optional instruction".to_string(),
        },
        DiffResult {
            entity: "User".to_string(),
            change_type: ChangeType::StructFieldAddition,
            severity: Severity::Minor,
            description: "Appended score field".to_string(),
        },
        DiffResult {
            entity: "User".to_string(),
            change_type: ChangeType::StructFieldRemoval,
            severity: Severity::Critical,
            description: "Removed admin field".to_string(),
        },
    ];

    let aggregated = generate_aggregated_impact(&diffs);

    assert_eq!(aggregated.severity, Severity::Critical);
    assert!(aggregated.migration_required);
    assert!(aggregated.risk_category.contains("API Expansion"));
    assert!(aggregated.risk_category.contains("Account Expansion"));
    assert!(aggregated
        .risk_category
        .contains("Account Deserialization Break"));

    // Ensure all impacts are collected
    assert!(aggregated
        .impact
        .contains(&"Existing PDA accounts incompatible".to_string()));
    assert!(aggregated
        .impact
        .contains(&"Existing accounts require realloc".to_string()));
    assert!(aggregated
        .impact
        .contains(&"New entry point added to the program".to_string()));
}
