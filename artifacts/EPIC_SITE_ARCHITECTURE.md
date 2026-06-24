# EPIC Website Architecture Specification

This document presents the sitemap, page hierarchy, content structure, and technical stack for the landing and documentation site (`solana-epic/epic-site`).

## 1. Technical Stack

*   **Framework**: Next.js 14+ (App Router)
*   **Styling**: Tailwind CSS
*   **Component System**: shadcn/ui
*   **Icons**: Lucide React
*   **Content Management**: Contentlayer / MDX (for rules and installation docs)

---

## 2. Sitemap & Page Hierarchy

```
solana-epic.dev/
├── Home (Overview, value prop, visual compilation pipeline, CTA to install)
├── Docs/ (General documentation layout)
│   ├── Installation (npm, Cargo, npx options)
│   ├── CLI Reference (Commands reference)
│   ├── Upgrade Safety (Layout drift, field shifting, type changing)
│   └── GitHub Action (Integration details)
├── Security Rules/ (Directory of registered security checks)
│   ├── EPIC-SEC-001 (Owner Validation)
│   ├── EPIC-SEC-002 (Signer Validation)
│   ├── EPIC-SEC-003 (Missing post-CPI State Reload)
│   ├── EPIC-SEC-004 (PDA Seed Collision)
│   └── EPIC-SEC-005 (Arbitrary CPI Targets)
└── Roadmap/ (Future plans, integrations, rules matrix)
```

---

## 3. CTA & Conversion Flow

*   **Primary CTA**: `npm install -g @solana-epic/cli` (with 1-click clipboard copy).
*   **Secondary CTA**: *Get Started with GitHub Actions* $\rightarrow$ points to GitHub Action setup guide.
*   **Developer Loop**: Read rule explanation $\rightarrow$ Copy safe pattern snippet $\rightarrow$ Integrate checker on PRs.
