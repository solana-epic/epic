# EPIC Simulation Architecture

EPIC v0.3 introduces upgrade simulation as a reusable engine layer.

The current simulator is static. It uses the existing parser, diff engine, realloc guidance, and rent estimation framework to model what an upgrade requires before any chain execution is attempted.

## Current Flow

1. Parse the old program.
2. Parse the new program.
3. Build an upgrade readiness report.
4. Convert account diffs into affected accounts.
5. Collect realloc requirements.
6. Produce a migration plan.
7. Assign an overall risk level and numeric risk score.
8. Estimate rent increase from additional account bytes.

## Simulation Runner Boundary

Simulation is represented by the `UpgradeSimulationRunner` interface.

The default implementation is `StaticUpgradeSimulationRunner`, which does not execute transactions. It is deterministic and CI-friendly.

Future implementations can provide:

- Bankrun execution.
- Account fixture loading.
- Simulated realloc instructions.
- Rent exemption checks from a runtime context.
- Migration instruction validation.

## Future Bankrun Integration

A Bankrun-backed runner should implement the same simulation interface:

```ts
interface UpgradeSimulationRunner {
  simulate(oldProjectPath: string, newProjectPath: string): Promise<UpgradeSimulation>;
}
```

That keeps the CLI, GitHub Actions, SHIFT, and SolDeploy on the same contract while allowing the execution backend to become more realistic over time.
