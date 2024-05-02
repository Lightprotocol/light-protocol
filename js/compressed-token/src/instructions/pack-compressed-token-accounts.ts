import {
    ParsedTokenAccount,
    InputTokenDataWithContext,
    getIndexOrAdd,
    bn,
    padOutputStateMerkleTrees,
} from '@lightprotocol/stateless.js';
import { PublicKey, AccountMeta } from '@solana/web3.js';

export type PackCompressedTokenAccountsParams = {
    /** Input state to be consumed */
    inputCompressedTokenAccounts: ParsedTokenAccount[];
    /** Length of output compressed accounts */
    outputCompressedAccountsLength: number;
    /**
     * State trees that the output should be inserted into. Defaults to the 0th
     * state tree of the input state. Gets padded to the length of
     * outputCompressedAccounts.
     */
    outputStateTrees?: PublicKey[] | PublicKey;
    /** Optional remaining accounts to append to */
    remainingAccounts?: PublicKey[];
};

// TODO: include owner and lamports in packing.
/**
 * Packs Compressed Token Accounts.
 */
export function packCompressedTokenAccounts(
    params: PackCompressedTokenAccountsParams,
): {
    inputTokenDataWithContext: InputTokenDataWithContext[];
    outputStateMerkleTreeIndices: number[];
    remainingAccountMetas: AccountMeta[];
} {
    const {
        inputCompressedTokenAccounts,
        outputCompressedAccountsLength,
        outputStateTrees,
        remainingAccounts = [],
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
    inputCompressedTokenAccounts.forEach((account: ParsedTokenAccount) => {
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
            delegatedAmount: account.parsed.delegatedAmount.eq(bn(0))
                ? null
                : account.parsed.delegatedAmount,
            isNative: account.parsed.isNative,
            merkleContext: {
                merkleTreePubkeyIndex,
                nullifierQueuePubkeyIndex,
                leafIndex: account.compressedAccount.leafIndex,
            },
        });
    });

    /// pack output state trees
    const paddedOutputStateMerkleTrees = padOutputStateMerkleTrees(
        outputStateTrees,
        outputCompressedAccountsLength,
        inputCompressedTokenAccounts.map(acc => acc.compressedAccount),
    );
    const outputStateMerkleTreeIndices: number[] = [];
    paddedOutputStateMerkleTrees.forEach(account => {
        const indexMerkleTree = getIndexOrAdd(_remainingAccounts, account);
        outputStateMerkleTreeIndices.push(indexMerkleTree);
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
        outputStateMerkleTreeIndices,
        remainingAccountMetas,
    };
}
