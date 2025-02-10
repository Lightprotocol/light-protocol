import { PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import {
    CompressedProof,
    PackedMerkleContext,
} from '@lightprotocol/stateless.js';

export type CompressedCpiContext = {
    setContext: boolean;
    firstSetContext: boolean;
    cpiContextAccountIndex: number; // u8
};

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
    /**
     * TokenExtension tlv
     */
    tlv: Buffer | null;
};

export type PackedTokenTransferOutputData = {
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
    /**
     * Merkle tree pubkey index in remaining accounts
     */
    merkleTreeIndex: number;
    /**
     * TokenExtension tlv
     */
    tlv: Buffer | null;
};

export type InputTokenDataWithContext = {
    amount: BN;
    delegateIndex: number | null;
    merkleContext: PackedMerkleContext;
    rootIndex: number;
    lamports: BN | null;
    tlv: Buffer | null;
};

export type DelegatedTransfer = {
    owner: PublicKey;
    delegateChangeAccountIndex: number | null;
};

export type MintToInstructionData = {
    recipients: PublicKey[];
    amounts: BN[];
    lamports: BN | null;
};

export type CompressV2InstructionData = {
    publicKeys: PublicKey[];
    amount: BN;
    lamports: BN | null;
};

export type CompressSplTokenAccountInstructionData = {
    owner: PublicKey;
    remainingAmount: BN | null;
    cpiContext: CompressedCpiContext | null;
};

export type CompressedTokenInstructionDataTransfer = {
    /**
     * Validity proof
     */
    proof: CompressedProof | null;
    /**
     * The mint of the transfer
     */
    mint: PublicKey;
    /**
     * Whether the signer is a delegate
     * TODO: implement delegated transfer struct
     */
    delegatedTransfer: DelegatedTransfer | null;
    /**
     * Input token data with packed merkle context
     */
    inputTokenDataWithContext: InputTokenDataWithContext[];
    /**
     * Data of the output token accounts
     */
    outputCompressedAccounts: PackedTokenTransferOutputData[];
    /**
     * Whether it's a compress or decompress action if compressOrDecompressAmount is non-null
     */
    isCompress: boolean;
    /**
     * If null, it's a transfer.
     * If some, the amount that is being deposited into (compress) or withdrawn from (decompress) the token escrow
     */
    compressOrDecompressAmount: BN | null;
    /**
     * CPI context if
     */
    cpiContext: CompressedCpiContext | null;
    /**
     * The index of the Merkle tree for a lamport change account.
     */
    lamportsChangeAccountMerkleTreeIndex: number | null;
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
     * TokenExtension tlv
     */
    tlv: Buffer | null;
};
