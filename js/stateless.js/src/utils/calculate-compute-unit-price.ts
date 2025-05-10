/**
 * @param targetLamports - Target priority fee in lamports
 * @param computeUnits - Expected compute units used by the transaction
 * @returns microLamports per compute unit (use in
 * {@link https://github.com/solana-foundation/solana-web3.js/blob/maintenance/v1.x/src/programs/compute-budget.ts#L218})
 */
export function calculateComputeUnitPrice(
    targetLamports: number,
    computeUnits: number,
): number {
    return Math.ceil((targetLamports * 1_000_000) / computeUnits);
}
