import { PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { CompressedProof } from '@lightprotocol/stateless.js';

/// TODO: remove index_mt_account on-chain. passed as part of
/// CompressedTokenInstructionDataInvoke
export type TokenTransferOutputData = {
    /**
     * The owner of the output token account
     */
    owner: PublicKey;
    /**
     * The amount of tokens of the output token account
     */
    amount: BN;
    /**
     * lamports associated with the output token account
     */
    lamports: BN | null;
};

export type InputTokenDataWithContext = {
    /**
     * The amount of tokens to transfer
     */
    amount: BN;
    /**
     * Optional: The index of the delegate in remaining accounts
     */
    delegateIndex: number | null;
    /**
     * Optional: The amount of delegated tokens
     */
    delegatedAmount: BN | null;
    /**
     * Optional: Whether the token is native (wSOL)
     */
    isNative: BN | null;
    /**
     * The index of the merkle tree address in remaining accounts
     */
    merkleTreePubkeyIndex: number;
    /**
     * The index of the nullifier queue address in remaining accounts
     */
    nullifierQueuePubkeyIndex: number;
    /**
     * The index of the leaf in the merkle tree
     */
    leafIndex: number;
};

export type CompressedTokenInstructionDataInvoke = {
    /**
     * Validity proof
     */
    proof: CompressedProof | null;
    /**
     * The root indices of the transfer
     */
    rootIndices: number[];
    /**
     * The mint of the transfer
     */
    mint: PublicKey;
    /**
     * Whether the signer is a delegate
     */
    signerIsDelegate: boolean;
    /**
     * Input token data with packed merkle context
     */
    inputTokenDataWithContext: InputTokenDataWithContext[];
    /**
     * Data of the output token accounts
     */
    outputCompressedAccounts: TokenTransferOutputData[];
    /**
     * The indices of the output state merkle tree accounts in 'remaining
     * accounts'
     */
    outputStateMerkleTreeAccountIndices: Buffer;
};

export type TokenData = {
    /**
     * The mint associated with this account
     */
    mint: PublicKey;
    /**
     * The owner of this account
     */
    owner: PublicKey;
    /**
     * The amount of tokens this account holds
     */
    amount: BN;
    /**
     * If `delegate` is `Some` then `delegated_amount` represents the amount
     * authorized by the delegate
     */
    delegate: PublicKey | null;
    /**
     * The account's state
     */
    state: number;
    /**
     * If is_some, this is a native token, and the value logs the rent-exempt
     * reserve. An Account is required to be rent-exempt, so the value is used
     * by the Processor to ensure that wrapped SOL accounts do not drop below
     * this threshold.
     */
    isNative: BN | null;
    /**
     * The amount delegated
     */
    delegatedAmount: BN;
};
