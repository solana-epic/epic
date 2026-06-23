# Installation

EPIC consists of a high-performance Rust parsing engine packaged inside a Node.js CLI wrapper.

---

## Prerequisites
*   **Node.js**: v18.0.0 or higher.
*   **macOS / Linux**: Supported out-of-the-box with precompiled binaries.

---

## Installation Methods

### 1. Global Installation (NPM)
Install the CLI globally using npm:
```bash
npm install -g @epic/cli
```

### 2. Run via NPX
Alternatively, you can run EPIC directly without permanent installation:
```bash
npx @epic/cli audit .
```

### 3. Local Development Build
To install and build from source:
1.  Clone the repository:
    ```bash
    git clone https://github.com/akxh5/Solana-EPIC.git
    cd Solana-EPIC
    ```
2.  Install dependencies:
    ```bash
    npm install
    ```
3.  Build the Rust parsing v2 engine:
    ```bash
    cd packages/parser-v2
    cargo build --release
    ```
4.  Build and link the TypeScript CLI wrapper:
    ```bash
    cd ../cli
    npm run build
    npm link
    ```
