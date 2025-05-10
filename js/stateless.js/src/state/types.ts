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
 * @deprecated Use {@link StateTreeInfo} instead.
 *
 * A bundle of active trees for a given tree type.
 */
export type ActiveTreeBundle = {
    /**
     * Tree.
     */
    tree: PublicKey;
    /**
     * Queue.
     */
    queue: PublicKey | null;
    /**
     * CPI context.
     */
    cpiContext: PublicKey | null;
    /**
     * Tree type.
     */
    treeType: TreeType;
};

/**
 * State tree info, versioned via {@link TreeType}. The protocol
 * stores compressed accounts in state trees.
 *
 * Onchain Accounts are subject to Solana's write-lock limits.
 *
 * To load balance transactions, use {@link selectStateTreeInfo} to
 * select a random tree from a range of active trees.
 *
 * Example:
 * ```typescript
 * const infos = await rpc.getStateTreeInfos();
 * const info = selectStateTreeInfo(infos);
 * const ix = await CompressedTokenProgram.compress({
 *     // ...
 *     outputStateTreeInfo: info
 * });
 * ```
 */
export type StateTreeInfo = {
    /**
     * Pubkey of the tree account.
     */
    tree: PublicKey;
    /**
     * Pubkey of the queue account associated with the tree.
     */
    queue: PublicKey;
    /**
     * The type of tree. One of {@link TreeType}.
     */
    treeType: TreeType;
    /**
     * Optional compressed cpi context account.
     */
    cpiContext?: PublicKey;
    /**
     * Optional next tree info if the tree is full.
     */
    nextTreeInfo: StateTreeInfo | null;
};

/**
 * Address tree info, versioned via {@link TreeType}. The protocol
 * stores PDAs in address trees.
 */
export type AddressTreeInfo = Omit<
    StateTreeInfo,
    'cpiContext' | 'nextTreeInfo'
> & {
    /**
     * Next tree info.
     */
    nextTreeInfo: AddressTreeInfo | null;
};

/**
 * Packed compressed account with merkle context.
 */
export interface PackedCompressedAccountWithMerkleContext {
    /**
     * Compressed account.
     */
    compressedAccount: CompressedAccount;
    /**
     * Merkle context.
     */
    merkleContext: PackedMerkleContext;
    /**
     * Root index.
     */
    rootIndex: number;
    /**
     * Read only.
     */
    readOnly: boolean;
}

export interface PackedMerkleContext {
    merkleTreePubkeyIndex: number; // u8
    nullifierQueuePubkeyIndex: number; // u8
    leafIndex: number; // u32
    queueIndex: null | QueueIndex; // Option<QueueIndex>
}

export interface QueueIndex {
    queueId: number;
    index: number;
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 * */
export interface CompressedAccount {
    /**
     * Public key of program or user owning the account.
     */
    owner: PublicKey;
    /**
     * Lamports attached to the account.
     */
    lamports: BN;
    /**
     * Optional unique account ID that is persistent across transactions.
     */
    address: number[] | null;
    /**
     * Optional data attached to the account.
     */
    data: CompressedAccountData | null;
}

/**
 * Describe the generic compressed account details applicable to every
 * compressed account.
 */
export interface OutputCompressedAccountWithPackedContext {
    compressedAccount: CompressedAccount;
    merkleTreeIndex: number;
}

/**
 * Describes compressed account data.
 */
export interface CompressedAccountData {
    /**
     * 8 bytes.
     */
    discriminator: number[];
    /**
     * Data.
     */
    data: Buffer;
    /**
     * 32 bytes.
     */
    dataHash: number[];
}

/**
 * Merkle tree sequence number.
 */
export interface MerkleTreeSequenceNumber {
    /**
     * Public key.
     */
    pubkey: PublicKey;
    /**
     * Sequence number.
     */
    seq: BN;
}

/**
 * Public transaction event.
 */
export interface PublicTransactionEvent {
    /**
     * Input compressed account hashes.
     */
    inputCompressedAccountHashes: number[][];
    /**
     * Output compressed account hashes.
     */
    outputCompressedAccountHashes: number[][];
    /**
     * Output compressed accounts.
     */
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    /**
     * Output leaf indices.
     */
    outputLeafIndices: number[];
    /**
     * Sequence numbers.
     */
    sequenceNumbers: MerkleTreeSequenceNumber[];
    /**
     * Relay fee. Default is null.
     */
    relayFee: BN | null;
    /**
     * Whether it's a compress or decompress instruction.
     */
    isCompress: boolean;
    /**
     * If some, it's either a compress or decompress instruction.
     */
    compressOrDecompressLamports: BN | null;
    /**
     * Public keys.
     */
    pubkeyArray: PublicKey[];
    /**
     * Message. Default is null.
     */
    message: Uint8Array | null;
}

/**
 * Instruction data for invoke.
 */
export interface InstructionDataInvoke {
    /**
     * Validity proof.
     */
    proof: ValidityProof | null;
    /**
     * Input compressed accounts with merkle context.
     */
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    /**
     * Output compressed accounts.
     */
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    /**
     * Relay fee. Default is null.
     */
    relayFee: BN | null;
    /**
     * Params for creating new addresses.
     */
    newAddressParams: NewAddressParamsPacked[];
    /**
     * If some, it's either a compress or decompress instruction.
     */
    compressOrDecompressLamports: BN | null;
    /**
     * Whether it's a compress or decompress instruction.
     */
    isCompress: boolean;
}

/**
 * Instruction data for invoking a CPI.
 */
export interface InstructionDataInvokeCpi {
    /**
     * Validity proof.
     */
    proof: ValidityProof | null;
    /**
     * Input compressed accounts with merkle context.
     */
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    /**
     * Output compressed accounts.
     */
    outputCompressedAccounts: OutputCompressedAccountWithPackedContext[];
    /**
     * Relay fee. Default is null.
     */
    relayFee: BN | null;
    /**
     * Params for creating new addresses.
     */
    newAddressParams: NewAddressParamsPacked[];
    /**
     * If some, it's either a compress or decompress instruction.
     */
    compressOrDecompressLamports: BN | null;
    /**
     * If `compressOrDecompressLamports` is some, whether it's a compress or
     * decompress instruction.
     */
    isCompress: boolean;
    /**
     * Optional compressed CPI context.
     */
    compressedCpiContext: CompressedCpiContext | null;
}

/**
 * Compressed CPI context.
 *
 * Use if you want to use a single {@link ValidityProof} to update two
 * compressed accounts owned by separate programs.
 */
export interface CompressedCpiContext {
    /**
     * Is set by the program that is invoking the CPI to signal that it should
     * set the cpi context.
     */
    set_context: boolean;
    /**
     * Is set to wipe the cpi context since someone could have set it before
     * with unrelated data.
     */
    first_set_context: boolean;
    /**
     * Index of cpi context account in remaining accounts.
     */
    cpi_context_account_index: number;
}

/**
 * @deprecated Use {@link ValidityProof} instead.
 */
export interface CompressedProof {
    /**
     * 32 bytes.
     */
    a: number[];
    /**
     * 64 bytes.
     */
    b: number[];
    /**
     * 32 bytes.
     */
    c: number[];
}

/**
 * Validity proof.
 *
 * You can request proofs via `rpc.getValidityProof` or
 * `rpc.getValidityProofV0`.
 *
 * One proof can prove the existence of N compressed accounts or the uniqueness
 * of N PDAs.
 */
export interface ValidityProof {
    /**
     * 32 bytes.
     */
    a: number[];
    /**
     * 64 bytes.
     */
    b: number[];
    /**
     * 32 bytes.
     */
    c: number[];
}

/**
 * Packed token data for input compressed accounts.
 */
export interface InputTokenDataWithContext {
    /**
     * Amount of tokens.
     */
    amount: BN;
    /**
     * Delegate index.
     */
    delegateIndex: number | null;
    /**
     * Merkle context.
     */
    merkleContext: PackedMerkleContext;
    /**
     * Root index.
     */
    rootIndex: number;
    /**
     * Lamports.
     */
    lamports: BN | null;
    /**
     * Tlv.
     */
    tlv: Buffer | null;
}

/**
 * Token data.
 */
export type TokenData = {
    /**
     * The mint associated with this account.
     */
    mint: PublicKey;
    /**
     * The owner of this account.
     */
    owner: PublicKey;
    /**
     * The amount of tokens this account holds.
     */
    amount: BN;
    /**
     * If `delegate` is `Some` then `delegated_amount` represents the amount
     * authorized by the delegate.
     */
    delegate: PublicKey | null;
    /**
     * The account's state.
     */
    state: number;
    /**
     * Token extension tlv.
     */
    tlv: Buffer | null;
};
