# EPIC-SEC-004: PDA Cryptographic Seed Collision — Hostile Design Review

## 1. What exactly is a PDA seed collision?
A Program Derived Address (PDA) seed collision occurs when two distinct sets of logical seeds produce the exact same concatenated byte array before hashing. Since Solana's runtime derives PDAs by concatenating all seed byte arrays without adding delimiters, boundary ambiguity between adjacent variable-length seeds allows one input to "overflow" into another, resulting in the same derived public key.

---

## 2. How does Solana derive PDAs?
Solana derives PDAs using `Pubkey::find_program_address(seeds, program_id)` or `Pubkey::create_program_address(seeds, program_id)`. The runtime:
1. Concatenates all input byte arrays in the `seeds` slice sequentially.
2. Concatenates the target `program_id`.
3. Appends a single-byte `bump` value (starting at 255).
4. Hashes the entire byte stream using SHA-256.
5. Verifies if the resulting 32-byte hash lies on the Ed25519 elliptic curve. If it does not, it represents a valid PDA (address with no corresponding private key) and is returned. If it does, the bump is decremented, and the loop repeats.

---

## 3. Why do adjacent variable-length seeds create ambiguity?
When Solana concatenates seeds, the boundaries between individual seeds are completely erased.
For example, if seeds are `[a, b]`:
*   If `a = "abc"`, `b = "def"` $\rightarrow$ concatenated stream is `"abcdef"`.
*   If `a = "abcd"`, `b = "ef"` $\rightarrow$ concatenated stream is `"abcdef"`.

If both `a` and `b` are variable-length (such as user-provided strings or symbols), an attacker can choose values that collide with another user's logical account derivation, hijacking the associated PDA balance or state.

---

## 4. Which real-world exploits or bug classes relate to seed collisions?
Common bug classes include:
*   **Multi-Tenant / Sub-Account Hijacks**: Vaults derived from `[username, vault_name]`. A malicious user can register `username = "alice_foo"`, `vault_name = "bar"` to collide with `username = "alice"`, `vault_name = "foo_bar"`.
*   **Dynamic Pools / Token Pair Hijacks**: Liquidity pools derived from `[token_a_symbol, token_b_symbol]`. If symbols are variable-length (like `"USDT"`, `"BTC"`), a collision can link two different token pairs to the same state account, corrupting pool accounting.

---

## 5. How does Sentio SW021 detect them?
Sentio SW021 uses AST-level pattern matching to inspect arguments of `find_program_address` or similar functions:
1. It matches expressions like `name.as_bytes()` or `.as_ref()`.
2. It flags any instance where two dynamic variable-like expressions are adjacent in the seeds array.
3. It does not perform semantic type resolution or alias tracing, relying primarily on syntactic matching.

---

## 6. What false positives does Sentio create?
Sentio's lack of deep type information leads to major false positives:
*   **Resolved Fixed-Size Arrays**: If a variable is a hash function return, e.g. `let fixed_hash: [u8; 32] = hash(name);`, passing `[name.as_bytes(), &fixed_hash]` will be flagged because Sentio cannot verify that `fixed_hash` is a fixed-length type.
*   **Aliases to Constants**: If a constant separator like `const DELIM: &[u8] = b"|";` is assigned to a local variable `let sep = DELIM;` and placed between two variable-length seeds, Sentio flags it as unsafe because it views `sep` as a dynamic variable rather than a constant delimiter.

---

## 7. How can EPIC use type information to outperform it?
EPIC leverages its unified **Type Registry** and **Symbol Resolver** to determine semantic width:
*   It traces each seed expression's type using the `TypeInferenceEngine`.
*   If a variable resolves to `[u8; N]`, `Pubkey`, `u64`, or other fixed-width types, it is classified as `FIXED_LENGTH`, preventing false positives.
*   It performs **Alias Tracing** to identify constant values and delimiters mapped through local variables.

---

## 8. Which seed types are fixed-length?
*   `Pubkey` (32 bytes)
*   Integers: `u8`, `u16`, `u32`, `u64`, `u128`, `i8`, `i16`, `i32`, `i64`, `i128` (converted via `.to_le_bytes()`, `.to_be_bytes()`)
*   Fixed-size byte arrays: `[u8; N]`
*   Static byte string literals: `b"vault"`, `b"|"` (known compile-time length)
*   References to the above (e.g., `&Pubkey`, `&[u8; 32]`)

---

## 9. Which seed types are variable-length?
*   `String`, `&str`
*   `Vec<u8>`
*   Dynamically sliced slices: `&[u8]` (unless inferred to be derived from a fixed-length parent symbol)

---

## 10. Which cases should be considered safe?
A seeds list is **SAFE** if:
1. No two `VARIABLE_LENGTH` seeds are adjacent.
2. Every `VARIABLE_LENGTH` seed is separated from another `VARIABLE_LENGTH` seed by at least one `FIXED_LENGTH` seed or constant delimiter (like `b"|"`).
