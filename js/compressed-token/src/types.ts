import { PublicKey } from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import { CompressedProof } from '@lightprotocol/stateless.js';

/// TODO: remove index_mt_account on-chain. passed as part of
/// CompressedTokenInstructionDataInvoke
export type TokenTransferOutputData = {
    owner: PublicKey;
    amount: BN;
    lamports: BN | null;
};

export type InputTokenDataWithContext = {
    amount: BN;
    delegateIndex: number | null;
    delegatedAmount: BN | null;
    isNative: BN | null;
    merkleTreePubkeyIndex: number;
    nullifierQueuePubkeyIndex: number;
    leafIndex: number;
};

export type CompressedTokenInstructionDataInvoke = {
    proof: CompressedProof | null;
    rootIndices: number[];
    mint: PublicKey;
    signerIsDelegate: boolean;
    inputTokenDataWithContext: InputTokenDataWithContext[];
    outputCompressedAccounts: TokenTransferOutputData[];
    outputStateMerkleTreeAccountIndices: Buffer;
};

export type TokenData = {
    /// The mint associated with this account
    mint: PublicKey;
    /// The owner of this account.
    owner: PublicKey;
    /// The amount of tokens this account holds.
    amount: BN;
    /// If `delegate` is `Some` then `delegated_amount` represents
    /// the amount authorized by the delegate
    delegate: PublicKey | null;
    /// The account's state
    state: number; // AccountState_IdlType;
    /// If is_some, this is a native token, and the value logs the rent-exempt
    /// reserve. An Account is required to be rent-exempt, so the value is
    /// used by the Processor to ensure that wrapped SOL accounts do not
    /// drop below this threshold.
    isNative: BN | null;
    /// The amount delegated
    delegatedAmount: BN;
    // TODO: validate that we don't need close authority
    // /// Optional authority to close the account.
    // close_authority?: PublicKey,
};
