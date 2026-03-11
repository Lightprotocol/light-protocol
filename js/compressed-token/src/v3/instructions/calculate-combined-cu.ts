import { rawLoadBatchComputeUnits, type InternalLoadBatch } from './load-ata';

const CU_BUFFER_FACTOR = 1.3;
const CU_MIN = 50_000;
const CU_MAX = 1_400_000;

export function calculateCombinedCU(
    baseCu: number,
    loadBatch: InternalLoadBatch | null,
): number {
    const rawLoadCu = loadBatch ? rawLoadBatchComputeUnits(loadBatch) : 0;
    const cu = Math.ceil((baseCu + rawLoadCu) * CU_BUFFER_FACTOR);
    return Math.max(CU_MIN, Math.min(CU_MAX, cu));
}
