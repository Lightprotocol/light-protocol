export {
    COMPRESSIBLE_DISCRIMINATORS,
    DecompressMultipleAccountsIdempotentData,
    UpdateCompressionConfigData,
    GenericCompressAccountInstruction,
} from './types';

export {
    UpdateCompressionConfigSchema,
    ValidityProofSchema,
    PackedStateTreeInfoSchema,
    CompressedAccountMetaSchema,
    GenericCompressAccountInstructionSchema,
    createCompressedAccountDataSchema,
    createDecompressMultipleAccountsIdempotentDataSchema,
    serializeInstructionData,
} from './layout';

export {
    createInitializeCompressionConfigInstruction,
    createUpdateCompressionConfigInstruction,
    createCompressAccountInstruction,
    createDecompressAccountsIdempotentInstruction,
    CompressibleInstruction,
} from './instruction';

export {
    initializeCompressionConfig,
    updateCompressionConfig,
    compressAccount,
    decompressAccountsIdempotent,
} from './action';

export {
    deriveCompressionConfigAddress,
    getProgramDataAccount,
    checkProgramUpdateAuthority,
} from './utils';

export { serializeInitializeCompressionConfigData } from './layout';

import { CompressedAccount } from '../state/compressed-account';
import {
    PackedStateTreeInfo,
    CompressedAccountMeta,
} from '../state/compressed-account';
import { CompressedAccountData } from './types';

/**
 * Convert a compressed account to the format expected by instruction builders
 */
export function createCompressedAccountData<T>(
    compressedAccount: CompressedAccount,
    data: T,
    seeds: Uint8Array[],
    outputStateTreeIndex: number,
): CompressedAccountData<T> {
    // Note: This is a simplified version. The full implementation would need
    // to handle proper tree info packing from ValidityProofWithContext
    const treeInfo: PackedStateTreeInfo = {
        rootIndex: 0, // Should be derived from ValidityProofWithContext
        proveByIndex: compressedAccount.proveByIndex,
        merkleTreePubkeyIndex: 0, // Should be derived from remaining accounts
        queuePubkeyIndex: 0, // Should be derived from remaining accounts
        leafIndex: compressedAccount.leafIndex,
    };

    const meta: CompressedAccountMeta = {
        treeInfo,
        address: compressedAccount.address
            ? Array.from(compressedAccount.address)
            : null,
        lamports: compressedAccount.lamports,
        outputStateTreeIndex,
    };

    return {
        meta,
        data,
        seeds,
    };
}

// Re-export for easy access following Solana SDK patterns
export { CompressibleInstruction as compressibleInstruction } from './instruction';
