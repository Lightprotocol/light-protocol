import {
    Rpc,
    MerkleContext,
    ValidityProof,
    packDecompressAccountsIdempotent,
} from '@lightprotocol/stateless.js';
import { PublicKey, AccountInfo, AccountMeta } from '@solana/web3.js';

export interface AccountInput {
    address: PublicKey;
    info: {
        accountInfo?: AccountInfo<Buffer>;
        parsed: any;
        merkleContext?: MerkleContext;
    };
    accountType: string;
    tokenVariant?: string;
    seeds?: Uint8Array[]; // Seeds for PDA derivation (from get_X_seeds functions)
    authoritySeeds?: Uint8Array[]; // Authority seeds for CTokens (from get_X_authority_seeds)
}

export interface DecompressInstructionParams {
    proofOption: { 0: ValidityProof | null };
    compressedAccounts: any[];
    systemAccountsOffset: number;
    remainingAccounts: AccountMeta[];
}

/**
 * Build decompress params for decompressAccountsIdempotent instruction.
 * Automatically handles proof generation and account packing for both
 * custom PDAs and cToken accounts.
 *
 * @param programId   The program ID
 * @param rpc         RPC connection
 * @param accounts    Array of account inputs with address, parsed data, and merkle context
 * @returns           Packed params ready for instruction, or null if no compressed accounts
 *
 * @example
 * ```typescript
 * const params = await buildDecompressParams(programId, rpc, [
 *     { address: poolAddress, info: poolInfo, accountType: "poolState" },
 *     { address: vault0, info: vault0Info, accountType: "cTokenData", tokenVariant: "token0Vault" },
 * ]);
 *
 * if (params) {
 *     const ix = await program.methods
 *         .decompressAccountsIdempotent(
 *             params.proofOption,
 *             params.compressedAccounts,
 *             params.systemAccountsOffset
 *         )
 *         .remainingAccounts(params.remainingAccounts)
 *         .instruction();
 * }
 * ```
 */
export async function buildDecompressParams(
    programId: PublicKey,
    rpc: Rpc,
    accounts: AccountInput[],
): Promise<DecompressInstructionParams | null> {
    const compressedAccounts = accounts.filter(
        acc => acc.info.merkleContext !== undefined,
    );

    if (compressedAccounts.length === 0) {
        return null;
    }

    const proofInputs = compressedAccounts.map(acc => ({
        hash: acc.info.merkleContext!.hash,
        tree: acc.info.merkleContext!.treeInfo.tree,
        queue: acc.info.merkleContext!.treeInfo.queue,
    }));

    const proof = await rpc.getValidityProofV0(proofInputs, []);

    const accountsData = compressedAccounts.map(acc => {
        if (acc.accountType === 'cTokenData') {
            if (!acc.tokenVariant) {
                throw new Error(
                    `tokenVariant is required when accountType is "cTokenData"`,
                );
            }
            return {
                key: 'cTokenData',
                data: {
                    variant: { [acc.tokenVariant]: {} },
                    tokenData: acc.info.parsed,
                },
                treeInfo: acc.info.merkleContext!.treeInfo,
                seeds: acc.seeds,
                authoritySeeds: acc.authoritySeeds,
            };
        } else {
            return {
                key: acc.accountType,
                data: acc.info.parsed,
                treeInfo: acc.info.merkleContext!.treeInfo,
                seeds: acc.seeds,
            };
        }
    });

    const addresses = compressedAccounts.map(acc => acc.address);

    const packed = await packDecompressAccountsIdempotent(
        programId,
        proof,
        accountsData,
        addresses,
    );

    return {
        proofOption: packed.proofOption,
        compressedAccounts: packed.compressedAccounts,
        systemAccountsOffset: packed.systemAccountsOffset,
        remainingAccounts: packed.remainingAccounts,
    };
}
