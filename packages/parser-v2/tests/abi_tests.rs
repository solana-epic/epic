use parser_v2::{abi::AbiEngine, Workspace};

fn get_fingerprint(source: &str, struct_name: &str) -> String {
    let mut workspace = Workspace::new();
    workspace
        .add_file("program", &["test_mod"], source, None)
        .unwrap();

    let mut abi_engine = AbiEngine::new(&workspace.registry);
    abi_engine
        .hash_of_absolute_path(&format!("program::test_mod::{}", struct_name))
        .unwrap()
}

#[test]
fn test_field_reorder_changes_fingerprint() {
    let source1 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
            pub b: u32,
        }
    "#;

    let source2 = r#"
        #[account]
        pub struct MyAccount {
            pub b: u32,
            pub a: u64,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(hash1, hash2, "Field reorder should change fingerprint");
}

#[test]
fn test_field_added_changes_fingerprint() {
    let source1 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
        }
    "#;

    let source2 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
            pub b: u32,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(hash1, hash2, "Field addition should change fingerprint");
}

#[test]
fn test_field_removed_changes_fingerprint() {
    let source1 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
            pub b: u32,
        }
    "#;

    let source2 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(hash1, hash2, "Field removal should change fingerprint");
}

#[test]
fn test_type_changes_changes_fingerprint() {
    let source1 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u64,
        }
    "#;

    let source2 = r#"
        #[account]
        pub struct MyAccount {
            pub a: u32,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(hash1, hash2, "Type change should change fingerprint");
}

#[test]
fn test_nested_struct_changes_fingerprint() {
    let source1 = r#"
        pub struct Nested {
            pub inner: u64,
        }
        #[account]
        pub struct MyAccount {
            pub n: Nested,
        }
    "#;

    let source2 = r#"
        pub struct Nested {
            pub inner: u32,
        }
        #[account]
        pub struct MyAccount {
            pub n: Nested,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(
        hash1, hash2,
        "Nested struct change should cascade to parent fingerprint"
    );
}

#[test]
fn test_enum_changes_fingerprint() {
    let source1 = r#"
        pub enum MyEnum {
            A,
            B,
        }
        #[account]
        pub struct MyAccount {
            pub e: MyEnum,
        }
    "#;

    let source2 = r#"
        pub enum MyEnum {
            A,
            B,
            C,
        }
        #[account]
        pub struct MyAccount {
            pub e: MyEnum,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(
        hash1, hash2,
        "Enum change should cascade to parent fingerprint"
    );
}

#[test]
fn test_alias_target_changes_fingerprint() {
    let source1 = r#"
        pub type MyAlias = u64;
        #[account]
        pub struct MyAccount {
            pub a: MyAlias,
        }
    "#;

    let source2 = r#"
        pub type MyAlias = u32;
        #[account]
        pub struct MyAccount {
            pub a: MyAlias,
        }
    "#;

    let hash1 = get_fingerprint(source1, "MyAccount");
    let hash2 = get_fingerprint(source2, "MyAccount");
    assert_ne!(
        hash1, hash2,
        "Alias target change should cascade to parent fingerprint"
    );
}
