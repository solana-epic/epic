use parser_v2::cfg::{
    extract_guards_from_accounts_struct, FactExpression, GuardFact, GuardTarget, SSAVersionId,
    SolanaProperty,
};
use parser_v2::types::{FieldDef, StructDef, TypeRef};
use std::collections::HashMap;

#[test]
fn test_extract_guards_from_anchor_accounts() {
    // Programmatic mockup of the parsed derive(Accounts) struct:
    // #[derive(Accounts)]
    // pub struct Initialize<'info> {
    //     #[account(init, payer = user, space = 8 + 64)]
    //     pub vault: Account<'info, VaultState>,
    //
    //     #[account(mut, signer)]
    //     pub user: Signer<'info>,
    //
    //     #[account(mut, seeds = [b"vault", user.key().as_ref()], bump)]
    //     pub pda_vault: Account<'info, VaultState>,
    //
    //     #[account(close = user)]
    //     pub temporary: Account<'info, TempState>,
    //
    //     #[account(realloc = 8 + 128, payer = user, zero = true)]
    //     pub resized_acc: Account<'info, ResizedState>,
    //
    //     #[account(has_one = user)]
    //     pub vault_authority: Account<'info, AuthState>,
    // }
    let accounts_struct = StructDef {
        name: "Initialize".to_string(),
        is_account: false,
        fields: vec![
            FieldDef {
                name: "vault".to_string(),
                type_ref: TypeRef::Custom("Account<'info, VaultState>".to_string()),
                attrs: vec!["#[account(init, payer = user, space = 8 + 64)]".to_string()],
            },
            FieldDef {
                name: "user".to_string(),
                type_ref: TypeRef::Custom("Signer<'info>".to_string()),
                attrs: vec!["#[account(mut, signer)]".to_string()],
            },
            FieldDef {
                name: "pda_vault".to_string(),
                type_ref: TypeRef::Custom("Account<'info, VaultState>".to_string()),
                attrs: vec![
                    "#[account(mut, seeds = [b\"vault\", user.key().as_ref()], bump)]".to_string(),
                ],
            },
            FieldDef {
                name: "temporary".to_string(),
                type_ref: TypeRef::Custom("Account<'info, TempState>".to_string()),
                attrs: vec!["#[account(close = user)]".to_string()],
            },
            FieldDef {
                name: "resized_acc".to_string(),
                type_ref: TypeRef::Custom("Account<'info, ResizedState>".to_string()),
                attrs: vec!["#[account(realloc = 8 + 128, payer = user, zero = true)]".to_string()],
            },
            FieldDef {
                name: "vault_authority".to_string(),
                type_ref: TypeRef::Custom("Account<'info, AuthState>".to_string()),
                attrs: vec!["#[account(has_one = user)]".to_string()],
            },
        ],
        attrs: vec!["#[derive(Accounts)]".to_string()],
    };

    let mut symbol_table = HashMap::new();
    let mut next_symbol_id = 1;

    let facts_provenance = extract_guards_from_accounts_struct(
        &accounts_struct,
        &mut symbol_table,
        &mut next_symbol_id,
    );

    let facts: Vec<GuardFact> = facts_provenance.into_iter().map(|(f, _)| f).collect();
    println!("PARSED FACTS: {:#?}", facts);

    // Verify Symbol ID mappings
    let user_symbol = *symbol_table.get("user").unwrap();
    let vault_symbol = *symbol_table.get("vault").unwrap();
    let pda_vault_symbol = *symbol_table.get("pda_vault").unwrap();
    let temporary_symbol = *symbol_table.get("temporary").unwrap();
    let resized_acc_symbol = *symbol_table.get("resized_acc").unwrap();
    let vault_authority_symbol = *symbol_table.get("vault_authority").unwrap();

    // 1. Check vault implicit ownership and explicit initialization
    assert!(facts.contains(&GuardFact::Owner {
        account: GuardTarget::Account(vault_symbol),
        expected_owner: FactExpression::Literal("program_id".to_string()),
    }));
    assert!(facts.contains(&GuardFact::Initialized {
        account: GuardTarget::Account(vault_symbol),
        payer: GuardTarget::Variable(SSAVersionId {
            symbol_id: user_symbol,
            version: 1
        }),
        space: Some(FactExpression::BinaryOp {
            op: "+".to_string(),
            lhs: Box::new(FactExpression::Literal("8".to_string())),
            rhs: Box::new(FactExpression::Literal("64".to_string())),
        }),
    }));

    // 2. Check user signer constraint
    assert!(facts.contains(&GuardFact::Signer(GuardTarget::Account(user_symbol))));

    // 3. Check pda_vault seeds derivation
    let expected_seeds = FactExpression::Literal("[b\"vault\",user.key().as_ref()]".to_string());
    assert!(facts.iter().any(|f| match f {
        GuardFact::PDA {
            account,
            seeds,
            bump,
        } => {
            account == &GuardTarget::Account(pda_vault_symbol)
                && seeds.contains(&expected_seeds)
                && bump.is_none() // bump is parsed and matched empty Option or resolved expression
        }
        _ => false,
    }));

    // 4. Check temporary closed / deallocated constraint
    assert!(facts.contains(&GuardFact::Deallocated {
        account: GuardTarget::Account(temporary_symbol),
        destination: GuardTarget::Variable(SSAVersionId {
            symbol_id: user_symbol,
            version: 1
        }),
    }));

    // 5. Check resized_acc reallocated / resized constraint
    assert!(facts.contains(&GuardFact::Resized {
        account: GuardTarget::Account(resized_acc_symbol),
        new_size: FactExpression::BinaryOp {
            op: "+".to_string(),
            lhs: Box::new(FactExpression::Literal("8".to_string())),
            rhs: Box::new(FactExpression::Literal("128".to_string())),
        },
        payer: GuardTarget::Literal("payer".to_string()),
    }));

    // 6. Check vault_authority has_one KeyRelation constraint
    assert!(facts.contains(&GuardFact::KeyRelation {
        account: GuardTarget::Account(vault_authority_symbol),
        field: SolanaProperty::Address,
        target: GuardTarget::Variable(SSAVersionId {
            symbol_id: user_symbol,
            version: 1
        }),
    }));
}
