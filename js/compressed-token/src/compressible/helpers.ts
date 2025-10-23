import {
    Rpc,
    MerkleContext,
    ValidityProof,
    packDecompressAccountsIdempotent,
    CTOKEN_PROGRAM_ID,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import {
    PublicKey,
    AccountInfo,
    AccountMeta,
    Commitment,
} from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    TokenAccountNotFoundError,
} from '@solana/spl-token';
import { getAssociatedCTokenAddressAndBump } from './derivation';
import { Account, toAccountInfo } from '../mint/get-account-interface';
import { Buffer } from 'buffer';
import { getATAProgramId } from '../utils';

function parseTokenData(data: Buffer): {
    mint: PublicKey;
    owner: PublicKey;
    amount: BN;
    delegate: PublicKey | null;
    state: number;
    tlv: Buffer | null;
} | null {
    if (!data || data.length === 0) return null;

    try {
        let offset = 0;
        const mint = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const owner = new PublicKey(data.slice(offset, offset + 32));
        offset += 32;
        const amount = new BN(data.slice(offset, offset + 8), 'le');
        offset += 8;
        const delegateOption = data[offset];
        offset += 1;
        const delegate = delegateOption
            ? new PublicKey(data.slice(offset, offset + 32))
            : null;
        offset += 32;
        const state = data[offset];
        offset += 1;
        const tlvOption = data[offset];
        offset += 1;
        const tlv = tlvOption ? data.slice(offset) : null;

        return {
            mint,
            owner,
            amount,
            delegate,
            state,
            tlv,
        };
    } catch (error) {
        console.error('Token data parsing error:', error);
        return null;
    }
}

function convertTokenDataToAccount(
    address: PublicKey,
    tokenData: {
        mint: PublicKey;
        owner: PublicKey;
        amount: BN;
        delegate: PublicKey | null;
        state: number;
        tlv: Buffer | null;
    },
): Account {
    return {
        address,
        mint: tokenData.mint,
        owner: tokenData.owner,
        amount: BigInt(tokenData.amount.toString()),
        delegate: tokenData.delegate,
        delegatedAmount: BigInt(0),
        isInitialized: tokenData.state !== 0,
        isFrozen: tokenData.state === 2,
        isNative: false,
        rentExemptReserve: null,
        closeAuthority: null,
        tlvData: tokenData.tlv ? Buffer.from(tokenData.tlv) : Buffer.alloc(0),
    };
}

export interface AccountInput {
    address: PublicKey;
    info: {
        accountInfo?: AccountInfo<Buffer>;
        parsed: any;
        merkleContext?: MerkleContext;
    };
    accountType: string;
    tokenVariant?: string;
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
            };
        } else {
            return {
                key: acc.accountType,
                data: acc.info.parsed,
                treeInfo: acc.info.merkleContext!.treeInfo,
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
