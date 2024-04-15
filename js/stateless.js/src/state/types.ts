import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';

/// TODO: Consider flattening and implementing an IR in Beet.
export interface PackedCompressedAccountWithMerkleContext {
    compressedAccount: CompressedAccount;
    merkleTreePubkeyIndex: number; // u8
    nullifierQueuePubkeyIndex: number; // u8
    leafIndex: number; // u32 FIXME: switch on-chain to u64.
    // Missing: hash
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 * */
export interface CompressedAccount {
    /** Public key of program or user that owns the account */
    owner: PublicKey;
    /** Lamports attached to the account */
    lamports: BN; // u64 // FIXME: optional
    /**
     * TODO: Implement address functionality. Optional unique account ID that is
     * persistent across transactions.
     */
    address: PublicKey | null; // Option<PublicKey>
    /** Optional data attached to the account */
    data: CompressedAccountData | null; // Option<CompressedAccountData>
}

export interface CompressedAccountData {
    discriminator: number[]; // [u8; 8] // TODO: test with uint8Array instead
    data: Buffer; // bytes
    dataHash: number[]; // [u8; 32]
}

export interface PublicTransactionEvent {
    inputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    outputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    inputCompressedAccounts: PackedCompressedAccountWithMerkleContext[];
    outputCompressedAccounts: CompressedAccount[];
    outputStateMerkleTreeAccountIndices: Uint8Array; // bytes
    outputLeafIndices: number[]; // Vec<u32>
    relayFee: BN | null; // Option<u64>
    isCompress: boolean; // bool
    deCompressLamports: BN | null; // Option<u64>
    pubkeyArray: PublicKey[]; // Vec<PublicKey>
    message: Uint8Array | null; // Option<bytes>
}

export interface InstructionDataTransfer {
    proof: CompressedProof | null; // Option<CompressedProof>
    inputRootIndices: number[]; // Vec<u16>
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    outputCompressedAccounts: CompressedAccount[];
    outputStateMerkleTreeAccountIndices: Buffer; // bytes // FIXME: into Vec<u8> on-chain
    relayFee: BN | null; // Option<u64>
    deCompressLamports: BN | null; // Option<u64>
    isCompression: boolean; // bool
    newAddressParams: NewAddressParamsPacked[]; // Vec<NewAddressParamsPacked>
}

export interface NewAddressParamsPacked {
    seed: number[];
    addressQueueAccountIndex: number; // u8
    addressMerkleTreeAccountIndex: number; // u8
    addressMerkleTreeRootIndex: number; // u16
}

export interface CompressedProof {
    a: number[]; // [u8; 32]
    b: number[]; // [u8; 64]
    c: number[]; // [u8; 32]
}

/**
 * Compressed-token types
 *
 * TODO: Token-related code should ideally not have to go into stateless.js.
 * Find a better altnerative way to extend the RPC.
 *
 */
export type TokenTransferOutputData = {
    owner: PublicKey;
    amount: BN;
    lamports: BN | null;
};

export type CompressedTokenInstructionDataTransfer = {
    proof: CompressedProof | null;
    rootIndices: number[];
    mint: PublicKey;
    signerIsDelegate: boolean;
    inputTokenDataWithContext: InputTokenDataWithContext[];
    outputCompressedAccounts: TokenTransferOutputData[];
    outputStateMerkleTreeAccountIndices: Buffer;
    pubkeyArray: PublicKey[];
    isCompress: boolean;
    compressionAmount: BN | null;
};

export interface InputTokenDataWithContext {
    amount: BN;
    delegateIndex: number | null; // Option<u8>
    delegatedAmount: BN | null; // Option<u64>
    isNative: BN | null; // Option<u64>
    merkleTreePubkeyIndex: number; // u8
    nullifierQueuePubkeyIndex: number; // u8
    leafIndex: number; // u32
}
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
