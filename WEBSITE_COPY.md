# EPIC Website Copy

This document outlines the content architecture, copy, and layout structure for the EPIC platform website.

---

## 1. Hero Section

### Headline
EPIC

### Subheadline
Upgrade Intelligence for Solana Programs.  
Know what changes. Know what breaks. Ship with confidence.

### Copy
EPIC is the deployment readiness and upgrade safety infrastructure for Solana developers. Positioned between your compiler and your deployment keys, EPIC analyzes account layouts, tracks state evolution, and audits safety invariants before code reaches mainnet.

### Primary CTA
`npm install -g @solana-epic/cli` (with clipboard copy icon)

### Secondary CTA
[View Core Repository](https://github.com/solana-epic/epic)

---

## 2. Problem Section

### Header
The Upgrade Risk in Solana

### Copy
Every Solana program upgrade is a high-risk mutation of live state. Standard compilers only verify that your new code compiles—they cannot verify that your changes are compatible with your existing mainnet state.

A seemingly minor modification, such as changing a field type or shifting field order, corrupts existing account data. Missing an authority check or failing to reload a cached account state after a Cross-Program Invocation (CPI) exposes live assets to critical exploits.

Most teams discover these layout drifts and security regressions during audits or mainnet post-mortems. **EPIC shifts layout safety verification from mainnet incidents to compile-time guarantees.**

---

## 3. Features Section

### Feature 1: Upgrade Compatibility Analysis
*   **Title**: Diffs State Schemas, Instantly
*   **Copy**: EPIC compares the AST of your old program version against your new changes to track layout drifts, offset alignments, and discriminator changes.
*   **Visual**: A side-by-side AST schema diff highlighting added, reordered, or shrank fields.

### Feature 2: Invariant Verification
*   **Title**: Prevent Security Regressions
*   **Copy**: Automatically verifies compile-time invariants—such as ownership checks, privileged signer requirements, and PDA seed boundaries—across modified instruction paths.
*   **Visual**: Alert snippet highlighting a missing `reload()` call after a CPI instruction.

### Feature 3: Account Evolution Tracking
*   **Title**: Understand Your Memory Footprint
*   **Copy**: Analyzes serialized account sizes, offset structures, and memory growth projections to assist in managing state rent and realloc scaling.
*   **Visual**: Bar chart showing state scaling projection and byte offset alignment layout.

---

## 4. How It Works Section

### Header
Static Verification in Four Steps

### Step 1: Rust AST Parsing
EPIC parses Solana program source code using syn compiler models without requiring local binary compilation.

### Step 2: CFG & SSA Representation
Constructs Control Flow Graphs (CFG) and converts instruction logic into Single Static Assignment (SSA) form to map variable versioning.

### Step 3: Dominance Evaluation
Computes block dominance trees to statically verify that safety guards and ownership checks strictly dominate all write operations.

### Step 4: Layout Drift Diffs
Compares structural schema models between the old and new AST versions, outputting compiler-level upgrade safety verdicts.

---

## 5. CTA Section

### Headline
Eliminate Solana Deployment Risk Today

### Copy
Ship upgrades with absolute layout and safety confidence. Integrate EPIC into your local dev workflow and CI/CD pipelines.

### Primary CTA
`npm install -g @solana-epic/cli`
