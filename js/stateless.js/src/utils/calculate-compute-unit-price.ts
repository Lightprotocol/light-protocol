/**
 * @param targetLamports - Target priority fee in lamports
 * @param computeUnits - Expected compute units used by the transaction
 * @returns microLamports per compute unit (use in
 * `ComputeBudgetProgram.setComputeUnitPrice`)
 */
export function calculateComputeUnitPrice(
    targetLamports: number,
    computeUnits: number,
): number {
    return Math.ceil((targetLamports * 1_000_000) / computeUnits);
}
