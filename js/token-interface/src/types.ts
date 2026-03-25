import type { ParsedTokenAccount, Rpc } from '@lightprotocol/stateless.js';
import type { Commitment, PublicKey } from '@solana/web3.js';

export interface TokenInterfaceParsedAta {
    address: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    amount: bigint;
    delegate: PublicKey | null;
    delegatedAmount: bigint;
    isInitialized: boolean;
    isFrozen: boolean;
}

export interface TokenInterfaceAccount {
    address: PublicKey;
    owner: PublicKey;
    mint: PublicKey;
    amount: bigint;
    hotAmount: bigint;
    compressedAmount: bigint;
    hasHotAccount: boolean;
    requiresLoad: boolean;
    parsed: TokenInterfaceParsedAta;
    compressedAccount: ParsedTokenAccount | null;
    ignoredCompressedAccounts: ParsedTokenAccount[];
    ignoredCompressedAmount: bigint;
}

export interface AtaOwnerInput {
    owner: PublicKey;
    mint: PublicKey;
    programId?: PublicKey;
}

export interface GetAtaInput extends AtaOwnerInput {
    rpc: Rpc;
    commitment?: Commitment;
}

export interface CreateAtaInstructionsInput extends AtaOwnerInput {
    payer: PublicKey;
    programId?: PublicKey;
}

export interface CreateLoadInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer: PublicKey;
}

export interface CreateTransferInstructionsInput {
    rpc: Rpc;
    payer: PublicKey;
    mint: PublicKey;
    sourceOwner: PublicKey;
    authority: PublicKey;
    recipient: PublicKey;
    tokenProgram?: PublicKey;
    amount: number | bigint;
}

export interface CreateApproveInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer: PublicKey;
    delegate: PublicKey;
    amount: number | bigint;
}

export interface CreateRevokeInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer: PublicKey;
}

export interface CreateFreezeInstructionsInput {
    tokenAccount: PublicKey;
    mint: PublicKey;
    freezeAuthority: PublicKey;
}

export interface CreateThawInstructionsInput {
    tokenAccount: PublicKey;
    mint: PublicKey;
    freezeAuthority: PublicKey;
}

export type CreateRawAtaInstructionInput = CreateAtaInstructionsInput;
export type CreateRawLoadInstructionInput = CreateLoadInstructionsInput;

export interface CreateRawTransferInstructionInput {
    source: PublicKey;
    destination: PublicKey;
    mint: PublicKey;
    authority: PublicKey;
    payer: PublicKey;
    amount: number | bigint;
    decimals: number;
}

export interface CreateRawApproveInstructionInput {
    tokenAccount: PublicKey;
    delegate: PublicKey;
    owner: PublicKey;
    amount: number | bigint;
    payer?: PublicKey;
}

export interface CreateRawRevokeInstructionInput {
    tokenAccount: PublicKey;
    owner: PublicKey;
    payer?: PublicKey;
}
