import { PublicKey, AccountMeta } from '@solana/web3.js';
import { ValidityProof } from '../state';
import { TreeInfo } from '../state/types';

export interface AccountDataWithTreeInfo {
    key: string;
    data: any;
    treeInfo: TreeInfo;
}

export interface PackedDecompressResult {
    proofOption: { 0: ValidityProof | null };
    compressedAccounts: any[];
    systemAccountsOffset: number;
    remainingAccounts: AccountMeta[];
}

/**
 * Pack accounts and proof for decompressAccountsIdempotent instruction.
 * This function prepares compressed account data, validity proof, and remaining accounts
 * for idempotent decompression operations.
 *
 * @param programId - The program ID
 * @param proof - The validity proof with context
 * @param accountsData - Array of account data with tree info
 * @param addresses - Array of account addresses
 * @returns Packed instruction parameters
 */
export async function packDecompressAccountsIdempotent(
    programId: PublicKey,
    proof: { compressedProof: ValidityProof | null; treeInfos: TreeInfo[] },
    accountsData: AccountDataWithTreeInfo[],
    addresses: PublicKey[],
): Promise<PackedDecompressResult> {
    const remainingAccounts: AccountMeta[] = [];
    const remainingAccountsMap = new Map<string, number>();

    const getOrAddAccount = (pubkey: PublicKey, isWritable: boolean): number => {
        const key = pubkey.toBase58();
        if (!remainingAccountsMap.has(key)) {
            const index = remainingAccounts.length;
            remainingAccounts.push({
                pubkey,
                isSigner: false,
                isWritable,
            });
            remainingAccountsMap.set(key, index);
            return index;
        }
        return remainingAccountsMap.get(key)!;
    };

    // Add tree accounts to remaining accounts
    const compressedAccounts = accountsData.map((acc, index) => {
        const merkleTreePubkeyIndex = getOrAddAccount(acc.treeInfo.tree, true);
        const queuePubkeyIndex = getOrAddAccount(acc.treeInfo.queue, true);

        return {
            [acc.key]: acc.data,
            merkleContext: {
                merkleTreePubkeyIndex,
                queuePubkeyIndex,
            },
        };
    });

    // Add addresses as system accounts
    const systemAccountsOffset = remainingAccounts.length;
    addresses.forEach(addr => {
        getOrAddAccount(addr, true);
    });

    return {
        proofOption: { 0: proof.compressedProof },
        compressedAccounts,
        systemAccountsOffset,
        remainingAccounts,
    };
}

