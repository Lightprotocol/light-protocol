import {
    ParsedTokenAccount,
    InputTokenDataWithContext,
    getIndexOrAdd,
    bn,
    padOutputStateMerkleTrees,
    StateTreeInfo,
    TreeType,
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
     * State trees that the output should be inserted into. Defaults to the 0th
     * state tree of the input state. Gets padded to the length of
     * outputCompressedAccounts.
     */
    outputStateTreeInfo: StateTreeInfo;
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

    /// Currently just packs 'delegate' to pubkeyArray
    const packedInputTokenData: InputTokenDataWithContext[] = [];

    /// pack inputs
    inputCompressedTokenAccounts.forEach(
        (account: ParsedTokenAccount, index) => {
            const merkleTreePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.merkleTree,
            );

            const queuePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.queue,
            );

            packedInputTokenData.push({
                amount: account.parsed.amount,
                delegateIndex,
                merkleContext: {
                    merkleTreePubkeyIndex,
                    queuePubkeyIndex: queuePubkeyIndex,
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

    // V2 trees require the output queue account instead of directly appending
    // to the merkle tree.
    const outputTreeOrQueue =
        outputStateTreeInfo.treeType === TreeType.StateV2
            ? outputStateTreeInfo.queue!
            : outputStateTreeInfo.tree;

    /// pack output state trees
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        outputTreeOrQueue,
        tokenTransferOutputs.length,
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
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
