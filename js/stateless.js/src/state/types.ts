import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { NewAddressParamsPacked } from '../utils';

export interface PackedCompressedAccountWithMerkleContext {
    compressedAccount: CompressedAccount;
    merkleContext: PackedMerkleContext;
    rootIndex: number; // u16
}

export interface PackedMerkleContext {
    merkleTreePubkeyIndex: number; // u8
    nullifierQueuePubkeyIndex: number; // u8
    leafIndex: number; // u32
    queueIndex: null | QueueIndex; // Option<QueueIndex>
}

export interface QueueIndex {
    queueId: number; // u8
    index: number; // u16
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
     * TODO: use PublicKey. Optional unique account ID that is persistent across
     * transactions.
     */
    address: number[] | null; // Option<PublicKey>
    /** Optional data attached to the account */
    data: CompressedAccountData | null; // Option<CompressedAccountData>
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 * */
export interface OutputCompressedAccountWithPackedContext {
    compressedAccount: CompressedAccount;
    merkleTreeIndex: number;
}

export interface CompressedAccountData {
    discriminator: number[]; // [u8; 8] // TODO: test with uint8Array instead
    data: Buffer; // bytes
    dataHash: number[]; // [u8; 32]
}

export interface PublicTransactionEvent {
    inputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    outputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    outputLeafIndices: number[]; // Vec<u32>
    relayFee: BN | null; // Option<u64>
    isCompress: boolean; // bool
    compressOrDecompressLamports: BN | null; // Option<u64>
    pubkeyArray: PublicKey[]; // Vec<PublicKey>
    message: Uint8Array | null; // Option<bytes>
}

export interface InstructionDataInvoke {
    proof: CompressedProof | null; // Option<CompressedProof>
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    relayFee: BN | null; // Option<u64>
    newAddressParams: NewAddressParamsPacked[]; // Vec<NewAddressParamsPacked>
    compressOrDecompressLamports: BN | null; // Option<u64>
    isCompress: boolean; // bool
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
    tlv: Buffer | null;
};

export type CompressedTokenInstructionDataTransfer = {
    proof: CompressedProof | null;
    mint: PublicKey;
    delegatedTransfer: null;
    inputTokenDataWithContext: InputTokenDataWithContext[];
    outputCompressedAccounts: TokenTransferOutputData[];
    isCompress: boolean;
    compressOrDecompressAmount: BN | null;
    cpiContext: null;
    lamportsChangeAccountMerkleTreeIndex: number | null;
};

export interface InputTokenDataWithContext {
    amount: BN;
    delegateIndex: number | null; // Option<u8>
    merkleContext: PackedMerkleContext;
    rootIndex: number; // u16
    lamports: BN | null;
    tlv: Buffer | null;
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
    /// TokenExtension tlv
    tlv: Buffer | null;
};
