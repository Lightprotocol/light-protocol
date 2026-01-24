import {
    ParsedTokenAccount,
    InputTokenDataWithContext,
    getIndexOrAdd,
    bn,
    padOutputStateMerkleTrees,
    TreeType,
    featureFlags,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { PublicKey, AccountMeta } from '@solana/web3.js';
import {
    PackedTokenTransferOutputData,
    TokenTransferOutputData,
} from '../types';

export type PackCompressedTokenAccountsParams = {
    /** Input state to be consumed */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * Output state tree. Required for mint/compress (no inputs).
     * For transfer/merge with V1 inputs: pass a V2 tree for migration.
     * If not provided with inputs, uses input tree.
     */
    outputStateTreeInfo?: TreeInfo;
    /** Optional remaining accounts to append to */
    remainingAccounts?: PublicKey[];
    /**
     *  Root indices that are used on-chain to fetch the correct root
     *  from the state Merkle tree account for validity proof verification.
     */
    rootIndices: number[];
    tokenTransferOutputs: TokenTransferOutputData[];
};

/**
 * Packs Compressed Token Accounts.
 */
export function packCompressedTokenAccounts(
    params: PackCompressedTokenAccountsParams,
): {
    inputTokenDataWithContext: InputTokenDataWithContext[];
    remainingAccountMetas: AccountMeta[];
    packedOutputTokenData: PackedTokenTransferOutputData[];
} {
    const {
        inputCompressedTokenAccounts,
        outputStateTreeInfo,
        remainingAccounts = [],
        rootIndices,
        tokenTransferOutputs,
    } = params;

    const _remainingAccounts = remainingAccounts.slice();
    let delegateIndex: number | null = null;

    if (
        inputCompressedTokenAccounts.length > 0 &&
        inputCompressedTokenAccounts[0].parsed.delegate
    ) {
        delegateIndex = getIndexOrAdd(
            _remainingAccounts,
            inputCompressedTokenAccounts[0].parsed.delegate,
        );
    }

    const packedInputTokenData: InputTokenDataWithContext[] = [];
    /// pack inputs
    inputCompressedTokenAccounts.forEach(
        (account: ParsedTokenAccount, index) => {
            const merkleTreePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.treeInfo.tree,
            );

            const queuePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.treeInfo.queue,
            );

            packedInputTokenData.push({
                amount: account.parsed.amount,
                delegateIndex,
                merkleContext: {
                    merkleTreePubkeyIndex,
                    queuePubkeyIndex,
                    leafIndex: account.compressedAccount.leafIndex,
                    proveByIndex: account.compressedAccount.proveByIndex,
                },
                rootIndex: rootIndices[index],
                lamports: account.compressedAccount.lamports.eq(bn(0))
                    ? null
                    : account.compressedAccount.lamports,
                tlv: null,
            });
        },
    );

    // Determine output tree:
    // 1. If outputStateTreeInfo provided, use it (enables V1→V2 migration)
    // 2. Otherwise use input tree (requires inputs)
    let outputTreeInfo: TreeInfo;

    if (outputStateTreeInfo) {
        outputTreeInfo = outputStateTreeInfo;
    } else if (inputCompressedTokenAccounts.length > 0) {
        outputTreeInfo =
            inputCompressedTokenAccounts[0].compressedAccount.treeInfo;
    } else {
        throw new Error(
            'Either input accounts or outputStateTreeInfo must be provided',
        );
    }

    // Use next tree if available, otherwise fall back to current tree.
    // `nextTreeInfo` always takes precedence.
    const activeTreeInfo = outputTreeInfo.nextTreeInfo || outputTreeInfo;
    let activeTreeOrQueue = activeTreeInfo.tree;

    if (activeTreeInfo.treeType === TreeType.StateV2) {
        if (featureFlags.isV2()) {
            activeTreeOrQueue = activeTreeInfo.queue;
        } else throw new Error('V2 trees are not supported yet');
    }

    // Pack output state trees
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        activeTreeOrQueue,
        tokenTransferOutputs.length,
    );
    const packedOutputTokenData: PackedTokenTransferOutputData[] = [];
    paddedOutputStateMerkleTrees.forEach((account, index) => {
        const merkleTreeIndex = getIndexOrAdd(_remainingAccounts, account);
        packedOutputTokenData.push({
            owner: tokenTransferOutputs[index].owner,
            amount: tokenTransferOutputs[index].amount,
            lamports: tokenTransferOutputs[index].lamports?.eq(bn(0))
                ? null
                : tokenTransferOutputs[index].lamports,
            merkleTreeIndex,
            tlv: null,
        });
    });
    // to meta
    const remainingAccountMetas = _remainingAccounts.map(
        (account): AccountMeta => ({
            pubkey: account,
            isWritable: true,
            isSigner: false,
        }),
    );

    return {
        inputTokenDataWithContext: packedInputTokenData,
        remainingAccountMetas,
        packedOutputTokenData,
    };
}
