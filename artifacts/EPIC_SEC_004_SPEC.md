# EPIC-SEC-004: PDA Cryptographic Seed Collision Risk — Specification

## 1. Rule Metadata
*   **Rule ID**: `EPIC-SEC-004`
*   **Title**: PDA Cryptographic Seed Collision Risk
*   **Severity**: `HIGH`
*   **Target Vulnerability**: PDA derivation hijacking via ambiguous adjacent variable-length seeds.

---

## 2. Threat Model
Solana's runtime derives Program Derived Addresses (PDAs) by hashing a concatenated stream of bytes provided in the `seeds` slice without any runtime length prefixing or boundary delimiters. When two variable-length seeds (such as dynamic strings or raw vectors) are placed directly next to each other, an attacker can shift bytes from the end of the first seed to the beginning of the second seed while keeping the concatenated byte stream identical. This allows different logical accounts to map to the same PDA, leading to unauthorized state overwrite or access.

---

## 3. Code Patterns

### Vulnerable Examples (Native & Anchor)

#### 1. Adjacent Dynamic Strings
```rust
// Vulnerable: Adjacent variable-length string seeds
Pubkey::find_program_address(
    &[
        user_name.as_bytes(),
        folder_name.as_bytes()
    ],
    program_id
)
```

#### 2. Adjacent Vector Seeds
```rust
// Vulnerable: Adjacent raw vector slices
Pubkey::find_program_address(
    &[
        vec_a.as_slice(),
        vec_b.as_slice()
    ],
    program_id
)
```

---

### Safe Examples

#### 1. Separated by Fixed-Length Key
```rust
// Safe: Variable-length name separated by a fixed-length public key
Pubkey::find_program_address(
    &[
        user_name.as_bytes(),
        user_key.as_ref(), // Fixed-length (32 bytes)
        folder_name.as_bytes()
    ],
    program_id
)
```

#### 2. Separated by String Literal Delimiter
```rust
// Safe: Variable-length name separated by a static literal string delimiter
Pubkey::find_program_address(
    &[
        user_name.as_bytes(),
        b"|",              // Static delimiter (1 byte)
        folder_name.as_bytes()
    ],
    program_id
)
```

#### 3. Anchor Attribute Safe Delimiting
```rust
// Safe: Anchor constraint uses static constant seeds
#[account(
    init,
    payer = user,
    space = 8 + 64,
    seeds = [user.key().as_ref(), b"vault"], // boundaries clear
    bump
)]
pub vault: Account<'info, VaultAccount>,
```

---

### False Positive Examples (EPIC Advantage)

#### 1. Adjacent to Fixed-Length Inferred Hash Array
Sentio flags this pattern because it syntactically views `fixed_hash` as a dynamic slice argument. EPIC resolves its type to `[u8; 32]` and classifies it as `SAFE`.
```rust
let fixed_hash: [u8; 32] = hash(name.as_bytes());

// Safe: fixed_hash is exactly 32 bytes
Pubkey::find_program_address(
    &[
        name.as_bytes(),
        &fixed_hash
    ],
    program_id
)
```

#### 2. Constant Identifier Aliasing
Sentio flags this because it doesn't trace local variable initialization to constant string declarations.
```rust
const SEP: &[u8] = b"-";
let delimiter = SEP;

// Safe: delimiter resolves to constant b"-"
Pubkey::find_program_address(
    &[
        name.as_bytes(),
        delimiter,
        symbol.as_bytes()
    ],
    program_id
)
```

---

## 4. Expected Diagnostic Output

```json
{
  "rule_id": "EPIC-SEC-004",
  "severity": "High",
  "message": "Potential PDA cryptographic seed collision risk. Adjacent variable-length seeds 'user_name' and 'folder_name' can merge ambiguously. Insert a fixed-length seed or literal delimiter between them.",
  "location": {
    "file": "programs/example/src/lib.rs",
    "line": 45,
    "column": 0
  }
}
```
