import {
    ParsedTokenAccount,
    InputTokenDataWithContext,
    getIndexOrAdd,
    bn,
    padOutputStateMerkleTrees,
    TokenTransferOutputData,
} from '@lightprotocol/stateless.js';
import { PublicKey, AccountMeta } from '@solana/web3.js';
import { PackedTokenTransferOutputData } from '../types';

export type PackCompressedTokenAccountsParams = {
    /** Input state to be consumed */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /**
     * State trees that the output should be inserted into. Defaults to the 0th
     * state tree of the input state. Gets padded to the length of
     * outputCompressedAccounts.
     */
    outputStateTrees?: PublicKey[] | PublicKey;
    /** Optional remaining accounts to append to */
    remainingAccounts?: PublicKey[];
    /**
     *  Root indices that are used on-chain to fetch the correct root
     *  from the state Merkle tree account for validity proof verification.
     */
    rootIndices: number[];
    tokenTransferOutputs: TokenTransferOutputData[];
};

// TODO: include owner and lamports in packing.
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
        outputStateTrees,
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
    /// TODO: move pubkeyArray to remainingAccounts
    /// Currently just packs 'delegate' to pubkeyArray
    const packedInputTokenData: InputTokenDataWithContext[] = [];
    /// pack inputs
    inputCompressedTokenAccounts.forEach(
        (account: ParsedTokenAccount, index) => {
            const merkleTreePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.merkleTree,
            );

            const nullifierQueuePubkeyIndex = getIndexOrAdd(
                _remainingAccounts,
                account.compressedAccount.nullifierQueue,
            );

            packedInputTokenData.push({
                amount: account.parsed.amount,
                delegateIndex,
                merkleContext: {
                    merkleTreePubkeyIndex,
                    nullifierQueuePubkeyIndex,
                    leafIndex: account.compressedAccount.leafIndex,
                    queueIndex: null,
                },
                rootIndex: rootIndices[index],
                lamports: account.compressedAccount.lamports.eq(bn(0))
                    ? null
                    : account.compressedAccount.lamports,
                tlv: null,
            });
        },
    );

    /// pack output state trees
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        outputStateTrees,
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
