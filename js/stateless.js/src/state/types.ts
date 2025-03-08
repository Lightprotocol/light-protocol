import BN from 'bn.js';
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { NewAddressParamsPacked } from '../utils';

export enum TreeType {
    /**
     * v1 state merkle tree
     */
    StateV1 = 1,
    /**
     * v1 address merkle tree
     */
    AddressV1 = 2,
    /**
     * v2 state merkle tree
     */
    StateV2 = 3,
    /**
     * v2 address merkle tree
     */
    AddressV2 = 4,
}

/**
 * Public keys for a state tree, versioned via {@link TreeType}. The protocol
 * stores compressed accounts in state trees.
 *
 * Onchain Accounts are subject to Solana's write-lock limits.
 *
 * To load balance transactions, use {@link pickStateTreeInfo} to
 * select a random tree from active Trees.
 *
 * Example:
 * ```typescript
 * const stateTreeInfos = await getCachedActiveStateTreeInfos();
 * const randomStateTreeInfo = pickStateTreeInfo(stateTreeInfos);
 * const ix = CompressedTokenProgram.compress({
 *     ... // other params
 *     outputStateTreeInfo: randomStateTreeInfo
 * });
 * ```
 */
export type StateTreeInfo = {
    /**
     * Account containing the Sparse Merkle tree in which a compressed
     * account is stored.
     */
    tree: PublicKey;
    /**
     * The state nullfier queue belonging to merkleTree.
     */
    queue: PublicKey | null;
    /**
     * The compressed cpi context account.
     */
    cpiContext: PublicKey | null;
    /**
     * The type of tree. One of {@link TreeType}.
     */
    treeType: TreeType;
};

export interface PackedCompressedAccountWithMerkleContext {
    compressedAccount: CompressedAccount;
    merkleContext: PackedMerkleContext;
    rootIndex: number; // u16
    readOnly: boolean;
}

export interface PackedMerkleContext {
    merkleTreePubkeyIndex: number; // u8
    queuePubkeyIndex: number; // u8
    leafIndex: number; // u32
    proveByIndex: boolean; // bool
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 * */
export interface CompressedAccount {
    /**
     * Public key of program or user that owns the account
     */
    owner: PublicKey;
    /**
     * Lamports stored in the account.
     */
    lamports: BN; // u64
    /**
     * Optional unique account ID that is persistent across transactions.
     */
    address: number[] | null; // Option<PublicKey>
    /**
     * Optional data stored in the account.
     */
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
    discriminator: number[]; // [u8; 8]
    data: Buffer; // bytes
    dataHash: number[]; // [u8; 32]
}

export interface MerkleTreeSequenceNumber {
    pubkey: PublicKey;
    seq: BN;
}

export interface PublicTransactionEvent {
    inputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    outputCompressedAccountHashes: number[][]; // Vec<[u8; 32]>
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    outputLeafIndices: number[]; // Vec<u32>
    sequenceNumbers: MerkleTreeSequenceNumber[]; // Vec<MerkleTreeSequenceNumber>
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

export interface InstructionDataInvokeCpi {
    proof: CompressedProof | null; // Option<CompressedProof>
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    relayFee: BN | null; // Option<u64>
    newAddressParams: NewAddressParamsPacked[]; // Vec<NewAddressParamsPacked>
    compressOrDecompressLamports: BN | null; // Option<u64>
    isCompress: boolean; // bool
    compressedCpiContext: CompressedCpiContext | null;
}

export interface CompressedCpiContext {
    /// Is set by the program that is invoking the CPI to signal that is should
    /// set the cpi context.
    setContext: boolean;
    /// Is set to wipe the cpi context since someone could have set it before
    /// with unrelated data.
    firstSetContext: boolean;
    /// Index of cpi context account in remaining accounts.
    cpiContextAccountIndex: number;
}

export interface CompressedProof {
    a: number[]; // [u8; 32]
    b: number[]; // [u8; 64]
    c: number[]; // [u8; 32]
}

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
