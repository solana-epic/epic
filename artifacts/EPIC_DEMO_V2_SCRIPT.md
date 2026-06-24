# EPIC Demo Walkthrough Script (v2)

A fast-paced, 3-minute technical product walk-through designed for developers and founders.

---

## Timeline & Script

### 0:00 - 0:30: Hook & Installation
*   **Visual**: Open a clean shell terminal.
*   **Action**: Run install command:
    ```bash
    npm install -g @solana-epic/cli
    ```
*   **Voiceover**: *"Hey everyone, this is Aksh. Today, I am going to show you how to protect your Solana protocols from layout drift and security vulnerabilities in under 3 minutes using EPIC. First, we install the CLI globally."*

### 0:30 - 1:00: Understanding Rules
*   **Visual**: Terminal output showing rules.
*   **Action**: Run:
    ```bash
    epic rules
    ```
    Then run:
    ```bash
    epic explain EPIC-SEC-005
    ```
*   **Voiceover**: *"EPIC registers 5 core compiler-level rules covering owner validation, signer validation, and arbitrary CPI targets. Let us explain SEC-005. It parses our Rust AST to find instruction calls passing target programs dynamically without checks. Let us run it against some actual code."*

### 1:00 - 2:00: Security Audit Execution
*   **Visual**: Code editor showing a vulnerable Anchor program, then running the audit in terminal.
*   **Action**: Run:
    ```bash
    epic audit demo/security_vulnerable
    ```
*   **Voiceover**: *"Here is our vulnerable demo. Notice we mutations without signer validations, and a CPI target passed blindly. Running `epic audit` immediately catches owner validation breaks, signer validation gaps, and the arbitrary CPI target. Zero configurations, sub-second execution."*

### 2:00 - 3:00: Upgrade Safety Engine
*   **Visual**: Show diffing layout changes between two programs.
*   **Action**: Run:
    ```bash
    epic check demo/upgrade_old demo/upgrade_new_critical
    ```
*   **Voiceover**: *"Now, what about upgrades? Upgrades are notorious for layout shifts. If we change field types, shrink account sizes, or swap ordering, we break mainnet state. Running `epic check` compares the old program AST against the new program AST, instantly flagging layout sizes shrinking and field width shifts as critical upgrade risks. Stop layout bugs before compilation. Install today."*
