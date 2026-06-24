# EPIC External Validation Plan

This plan details the list of targeted Solana security practitioners, protocol developers, and audit firms to engage for beta feedback and tool validation.

## 1. Targeted Audiences & Feedback Goals

### Category A: Core Security Auditing Firms (OtterSec, Sec3, Neodyme)
*   **Why Relevant**: These teams review dozens of Solana programs every month and are directly familiar with layout drift and CPI safety edge cases.
*   **Feedback Request**: Ask them about false positive rates on CPI target evaluations, and if there are generic layout changes they catch during audits that EPIC should verify.

### Category B: Protocol Engineering Teams (Drift, Marginfi, Kamino, Squads)
*   **Why Relevant**: Large production workspaces containing multiple interconnected programs.
*   **Feedback Request**: Ask them to run the GitHub Action on their active PR pipelines to check if the speed and rule alerts fit their CI build times without blocking workflows.

### Category C: Developer Tooling Builders (Anchor Team, Solana Foundation)
*   **Why Relevant**: Gatekeepers of developers installing ecosystem tooling.
*   **Feedback Request**: Request review on binary distribution wrapper reliability and compatibility across Windows, Linux, and Apple Silicon.
