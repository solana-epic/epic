# Upgrade Safety Guide

Solana programs are upgradable by default. However, modifying a program's state account structures (de-serialization layout) on an active program without migrating existing on-chain account data leads to structural state corruption.

EPIC checks for upgrade safety by validating structural changes between the old and new workspace versions.

---

## Checked Layout Modifications

### 1. Field Removal
Removing a field shifts the deserialization offset for all subsequent fields, causing data truncation or incorrect value loading.
*   **Vulnerable Example**:
    ```rust
    // Old Layout
    #[account]
    pub struct UserConfig {
        pub version: u8,
        pub admin: Pubkey,
        pub paused: bool,
    }

    // New Layout (Vulnerable: admin removed)
    #[account]
    pub struct UserConfig {
        pub version: u8,
        pub paused: bool, // Reads bytes offset originally belonging to admin!
    }
    ```

---

### 2. Field Reordering
Reordering fields of differing types alters the memory offsets. The deserializer will map byte streams into the wrong variables.
*   **Vulnerable Example**:
    ```rust
    // Old Layout
    #[account]
    pub struct Pool {
        pub fee: u64,
        pub authority: Pubkey,
    }

    // New Layout (Vulnerable: fields reordered)
    #[account]
    pub struct Pool {
        pub authority: Pubkey, // Tries to parse the u64 fee bytes as a Pubkey!
        pub fee: u64,
    }
    ```

---

### 3. Type Change
Modifying the type of a field changes its size in bytes, causing offset alignment drift for all fields defined after it.
*   **Vulnerable Example**:
    ```rust
    // Old Layout
    #[account]
    pub struct AccountState {
        pub index: u32,       // 4 bytes
        pub authority: Pubkey, // 32 bytes
    }

    // New Layout (Vulnerable: type expanded to u64)
    #[account]
    pub struct AccountState {
        pub index: u64,       // 8 bytes (Drifts subsequent field offsets by 4 bytes)
        pub authority: Pubkey,
    }
    ```

---

### 4. Account Shrinking
Reducing the size of an account layout is dangerous if existing accounts contain trailing data. Furthermore, Solana account reallocation does not support size decreases cleanly.
*   **Vulnerable Example**:
    ```rust
    // Old Layout (64 bytes)
    #[account]
    pub struct Vault {
        pub metadata: [u8; 64],
    }

    // New Layout (32 bytes - Vulnerable: Shrunk layout)
    #[account]
    pub struct Vault {
        pub metadata: [u8; 32],
    }
    ```

---

### 5. Discriminator Drift
Anchor uses an 8-byte discriminator calculated from the structure name (SHA-256 hash prefix). Renaming an accounts struct changes its discriminator, making all existing on-chain accounts fail discriminator checks on load.
*   **Vulnerable Example**:
    ```rust
    // Old Layout (Discriminator: hash of "UserState")
    #[account]
    pub struct UserState {
        pub user: Pubkey,
    }

    // New Layout (Discriminator: hash of "UserData" - Vulnerable: rename)
    #[account]
    pub struct UserData {
        pub user: Pubkey,
    }
    ```
