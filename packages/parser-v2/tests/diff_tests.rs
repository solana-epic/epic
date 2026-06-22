use parser_v2::{
    compare_workspaces, format_diff_results, ChangeType, DiffResult, Severity, Workspace,
};

fn run_diff(old_source: &str, new_source: &str) -> Vec<DiffResult> {
    let mut old_ws = Workspace::new();
    old_ws
        .add_file("program", &["test_mod"], old_source, None)
        .unwrap();

    let mut new_ws = Workspace::new();
    new_ws
        .add_file("program", &["test_mod"], new_source, None)
        .unwrap();

    compare_workspaces(&old_ws, &new_ws)
}

#[test]
fn test_safe_changes() {
    // 1. New optional instruction addition
    let old_program = r#"
        #[program]
        pub mod my_program {
            pub fn initialize(ctx: Context<Init>) -> Result<()> { Ok(()) }
        }
    "#;
    let new_program = r#"
        #[program]
        pub mod my_program {
            pub fn initialize(ctx: Context<Init>) -> Result<()> { Ok(()) }
            pub fn optional_ix(ctx: Context<Opt>) -> Result<()> { Ok(()) }
        }
    "#;
    let diffs = run_diff(old_program, new_program);
    let instr_add = diffs
        .iter()
        .find(|d| d.change_type == ChangeType::InstructionAddition)
        .unwrap();
    assert_eq!(instr_add.severity, Severity::Safe);
    assert!(instr_add.description.contains("Instruction"));
    assert!(instr_add.description.contains("added"));

    // 2. New account type introduced
    let old_accounts = r#"
        #[account]
        pub struct UserPosition {
            pub authority: Pubkey,
        }
    "#;
    let new_accounts = r#"
        #[account]
        pub struct UserPosition {
            pub authority: Pubkey,
        }
        #[account]
        pub struct ProtocolConfig {
            pub admin: Pubkey,
        }
    "#;
    let diffs2 = run_diff(old_accounts, new_accounts);
    let struct_add = diffs2
        .iter()
        .find(|d| d.entity.contains("ProtocolConfig"))
        .unwrap();
    assert_eq!(struct_add.severity, Severity::Safe);
}

#[test]
fn test_minor_changes() {
    // 1. New field appended at end of a non-account struct (safe layout expansion)
    let old_struct = r#"
        pub struct Position {
            pub owner: Pubkey,
        }
    "#;
    let new_struct = r#"
        pub struct Position {
            pub owner: Pubkey,
            pub score: u64,
        }
    "#;
    let diffs = run_diff(old_struct, new_struct);
    let field_add = diffs
        .iter()
        .find(|d| d.change_type == ChangeType::StructFieldAddition)
        .unwrap();
    assert_eq!(field_add.severity, Severity::Minor);
    assert!(field_add.description.contains("appended at end"));

    // 2. New enum variant appended
    let old_enum = r#"
        pub enum Status {
            Active,
            Inactive,
        }
    "#;
    let new_enum = r#"
        pub enum Status {
            Active,
            Inactive,
            Suspended,
        }
    "#;
    let diffs2 = run_diff(old_enum, new_enum);
    let var_add = diffs2
        .iter()
        .find(|d| d.change_type == ChangeType::EnumVariantAddition)
        .unwrap();
    assert_eq!(var_add.severity, Severity::Minor);
    assert!(var_add.description.contains("appended at end"));
}

#[test]
fn test_major_changes() {
    // 1. Account size changed
    let old_account = r#"
        #[account]
        pub struct UserPosition {
            pub owner: Pubkey,
        }
    "#;
    let new_account = r#"
        #[account]
        pub struct UserPosition {
            pub owner: Pubkey,
            pub score: u64,
        }
    "#;
    let diffs = run_diff(old_account, new_account);
    let size_change = diffs
        .iter()
        .find(|d| d.change_type == ChangeType::AccountLayoutChange)
        .unwrap();
    assert_eq!(size_change.severity, Severity::Major);
    assert!(size_change.description.contains("layout changed"));

    // 2. Type width changed
    let old_type = r#"
        pub struct Config {
            pub limit: u32,
        }
    "#;
    let new_type = r#"
        pub struct Config {
            pub limit: u64,
        }
    "#;
    let diffs2 = run_diff(old_type, new_type);
    let type_change = diffs2
        .iter()
        .find(|d| d.change_type == ChangeType::TypeChange)
        .unwrap();
    assert_eq!(type_change.severity, Severity::Major);
    assert!(type_change.description.contains("type width changed"));
}

#[test]
fn test_critical_changes() {
    // 1. Field reordering
    let old_reorder = r#"
        pub struct Position {
            pub owner: Pubkey,
            pub score: u64,
        }
    "#;
    let new_reorder = r#"
        pub struct Position {
            pub score: u64,
            pub owner: Pubkey,
        }
    "#;
    let diffs = run_diff(old_reorder, new_reorder);
    let reorder = diffs
        .iter()
        .find(|d| d.change_type == ChangeType::StructFieldReordering)
        .unwrap();
    assert_eq!(reorder.severity, Severity::Critical);
    assert!(reorder.description.contains("moved from field"));

    // 2. Field removal
    let old_removal = r#"
        pub struct Position {
            pub owner: Pubkey,
            pub score: u64,
        }
    "#;
    let new_removal = r#"
        pub struct Position {
            pub owner: Pubkey,
        }
    "#;
    let diffs2 = run_diff(old_removal, new_removal);
    let removal = diffs2
        .iter()
        .find(|d| d.change_type == ChangeType::StructFieldRemoval)
        .unwrap();
    assert_eq!(removal.severity, Severity::Critical);
    assert!(removal.description.contains("removed"));

    // 3. Enum variant reordering
    let old_enum = r#"
        pub enum Status {
            Active,
            Inactive,
        }
    "#;
    let new_enum = r#"
        pub enum Status {
            Inactive,
            Active,
        }
    "#;
    let diffs3 = run_diff(old_enum, new_enum);
    let enum_reorder = diffs3
        .iter()
        .find(|d| d.change_type == ChangeType::EnumVariantReordering)
        .unwrap();
    assert_eq!(enum_reorder.severity, Severity::Critical);
    assert!(enum_reorder.description.contains("reordered"));

    // 4. Instruction removal
    let old_program = r#"
        #[program]
        pub mod my_program {
            pub fn initialize(ctx: Context<Init>) -> Result<()> { Ok(()) }
            pub fn cancel(ctx: Context<Cancel>) -> Result<()> { Ok(()) }
        }
    "#;
    let new_program = r#"
        #[program]
        pub mod my_program {
            pub fn initialize(ctx: Context<Init>) -> Result<()> { Ok(()) }
        }
    "#;
    let diffs4 = run_diff(old_program, new_program);
    let inst_removal = diffs4
        .iter()
        .find(|d| d.change_type == ChangeType::InstructionRemoval)
        .unwrap();
    assert_eq!(inst_removal.severity, Severity::Critical);
    assert!(inst_removal.description.contains("removed"));

    // 5. PDA/account validation constraint change
    let old_pda = r#"
        #[derive(Accounts)]
        pub struct Initialize<'info> {
            #[account(init, payer = user, space = 8 + 32)]
            pub my_account: Account<'info, MyAccount>,
        }
    "#;
    let new_pda = r#"
        #[derive(Accounts)]
        pub struct Initialize<'info> {
            #[account(init, payer = user, space = 8 + 64)]
            pub my_account: Account<'info, MyAccount>,
        }
    "#;
    let diffs5 = run_diff(old_pda, new_pda);
    let pda_change = diffs5
        .iter()
        .find(|d| d.change_type == ChangeType::PdaAccountDefinitionChange)
        .unwrap();
    assert_eq!(pda_change.severity, Severity::Critical);
    assert!(pda_change.description.contains("constraints changed"));
}

#[test]
fn test_human_readable_output() {
    let old_reorder = r#"
        #[account]
        pub struct UserPosition {
            pub owner: Pubkey,
            pub score: u64,
        }
    "#;
    let new_reorder = r#"
        #[account]
        pub struct UserPosition {
            pub score: u64,
            pub owner: Pubkey,
        }
    "#;
    let diffs = run_diff(old_reorder, new_reorder);
    let output = format_diff_results(&diffs);

    assert!(output.contains("⚠ UserPosition"));
    assert!(output.contains("Severity: Critical"));
    assert!(output.contains("Changes:"));
    assert!(output.contains("field 'owner' moved from field #0 → #1"));
    assert!(output.contains("Impact:"));
    assert!(output.contains("Existing accounts incompatible"));
}
