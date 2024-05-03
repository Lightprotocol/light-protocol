import { BN } from '@coral-xyz/anchor';
import { PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import { CompressedAccountWithMerkleContext } from './compressed-account';

export interface PackedCompressedAccountWithMerkleContext {
    /**
     * The compressed account details
     */
    compressedAccount: CompressedAccount;
    /**
     * The packed merkle context details
     */
    merkleContext: PackedMerkleContext;
}

/**
 * Packed merkle context details
 */
export interface PackedMerkleContext {
    /**
     * Index of the merkle tree pubkey in remaining accounts
     */
    merkleTreePubkeyIndex: number;
    /**
     * Index of the nullifier queue pubkey in remaining accounts
     */
    nullifierQueuePubkeyIndex: number;
    /**
     * Index of the leaf in the merkle tree
     */
    leafIndex: number;
}

/**
 * Describes the generic account details applicable to every compressed account.
 */
export interface CompressedAccount {
    /**
     * Public key of program or user that owns the account
     */
    owner: PublicKey;
    /**
     * Lamports attached to the account
     */
    lamports: BN;
    /**
     * Optional unique id that is persistent across transactions.
     */
    address: PublicKey | null;
    /**
     * Optional data attached to the account
     */
    data: CompressedAccountData | null;
}

/**
 * Compressed account data
 */
export interface CompressedAccountData {
    /**
     * Discriminator for the compressed account data
     */
    discriminator: number[];
    /**
     * The data attached to the account
     */
    data: Buffer;
    /**
     * The hash of the data attached to the account
     */
    dataHash: number[];
}

/**
 * Event details as emitted by the light system program
 */
export interface PublicTransactionEvent {
    /**
     * Hashes of the transaction's input state (accounts)
     */
    inputCompressedAccountHashes: number[][];
    /**
     * Compressed accounts of the transaction's output state
     */
    outputCompressedAccountHashes: number[][];
    /**
     * Compressed accounts of the transaction's output state
     */
    outputCompressedAccounts: CompressedAccount[];
    /**
     * State tree account indices of the transaction's output state
     */
    outputStateMerkleTreeAccountIndices: Buffer;
    /**
     * Leaf indices of the transaction's output state in their repsective state
     * trees
     */
    outputLeafIndices: number[];
    /**
     * Optional relay fee in compressed lamports
     */
    relayFee: BN | null;
    /**
     * If `compressionLamports` is some(), whether it is a `compress` or
     * `decompress` instruction
     */
    isCompress: boolean;
    /**
     * The amount of lamports getting compressed or decompressed
     */
    compressionLamports: BN | null;
    /**
     * Deduped array of public keys referenced in the transaction.
     */
    pubkeyArray: PublicKey[];
    /**
     * Optional message attached to the transaction
     */
    message: Uint8Array | null;
}

/**
 * Generic instruction data for invoking the Light system program
 */
export interface InstructionDataInvoke {
    /**
     * Recent validity proof. Must be set if input state is read in the
     * transaction.
     */
    proof: CompressedProof | null;
    /**
     * Recent state root indices for input state.
     *
     * Must be set if input state is read in the transaction. Points to the
     * respective root hash position in the on-chain state root history array
     */
    inputRootIndices: number[];
    /**
     * Input state (compressed accounts) to be read in the instruction
     */
    inputCompressedAccountsWithMerkleContext: PackedCompressedAccountWithMerkleContext[];
    /**
     * Output state (compressed accounts) to be created in the instruction
     */
    outputCompressedAccounts: CompressedAccount[];
    /**
     * State tree account indices of the output accounts in 'remaining accounts'
     */
    outputStateMerkleTreeAccountIndices: Buffer;
    /**
     * Optional relay fee in compressed lamports
     */
    relayFee: BN | null;
    /**
     * Optional compression amount
     */
    compressionLamports: BN | null;
    /**
     * If `compressionLamports` is some(), whether it is a `compress` or
     * `decompress` instruction
     */
    isCompress: boolean;
    /**
     * Params if creating new persistent addresses
     */
    newAddressParams: NewAddressParamsPacked[];
}

export interface NewAddressParamsPacked {
    /**
     * Seed for the new address
     */
    seed: number[];
    /**
     * Index of the address queue account in remaining accounts
     */
    addressQueueAccountIndex: number;
    /**
     * Index of the address merkle tree account in remaining accounts
     */
    addressMerkleTreeAccountIndex: number;
    /**
     * Index of the address merkle tree root in the root buffer
     */
    addressMerkleTreeRootIndex: number;
}

/**
 * Validity proof
 */
export interface CompressedProof {
    a: number[];
    b: number[];
    c: number[];
}

/**
 * TODO: Token-related code should ideally not have to go into stateless.js.
 * Find a better altnerative way to extend the RPC.
 */
export interface ParsedTokenAccount {
    /**
     * Compressed account details
     */
    compressedAccount: CompressedAccountWithMerkleContext;
    /**
     * Parsed token data
     */
    parsed: TokenData;
}

export type EventWithParsedTokenTlvData = {
    /**
     * Hashes of the transaction's input state (accounts)
     */
    inputCompressedAccountHashes: number[][];
    /**
     * Parsed token accounts
     */
    outputCompressedAccounts: ParsedTokenAccount[];
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
