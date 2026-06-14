import type { RentImpact } from "./index.js";

export type RentEstimatorInput = {
  oldSize: number | null;
  newSize: number | null;
};

export interface RentEstimator {
  estimate(input: RentEstimatorInput): RentImpact;
}

export class MvpRentEstimator implements RentEstimator {
  estimate(input: RentEstimatorInput): RentImpact {
    if (input.oldSize === null || input.newSize === null) {
      return {
        status: "Unknown",
        estimatedAdditionalBytes: 0,
        exactLamports: null,
        futureHook: "RPC rent exemption lookup"
      };
    }

    const delta = input.newSize - input.oldSize;

    if (delta > 0) {
      return {
        status: "Increased",
        estimatedAdditionalBytes: delta,
        exactLamports: null,
        futureHook: "RPC rent exemption lookup"
      };
    }

    if (delta < 0) {
      return {
        status: "Decreased",
        estimatedAdditionalBytes: 0,
        exactLamports: null,
        futureHook: "RPC rent exemption lookup"
      };
    }

    return {
      status: "Unchanged",
      estimatedAdditionalBytes: 0,
      exactLamports: null,
      futureHook: "RPC rent exemption lookup"
    };
  }
}

export const defaultRentEstimator = new MvpRentEstimator();
