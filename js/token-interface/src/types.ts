import type {
    AddressTreeInfo,
    ParsedTokenAccount,
    Rpc,
    TreeInfo,
    ValidityProofWithContext,
} from '@lightprotocol/stateless.js';
import type { Commitment, PublicKey, Signer } from '@solana/web3.js';

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
    payer?: PublicKey;
    programId?: PublicKey;
}

export interface CreateLoadInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
}

export interface CreateTransferInstructionsInput {
    rpc: Rpc;
    payer?: PublicKey;
    mint: PublicKey;
    sourceOwner: PublicKey;
    authority: PublicKey;
    recipient: PublicKey;
    tokenProgram?: PublicKey;
    amount: number | bigint;
}

export interface CreateApproveInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
    delegate: PublicKey;
    amount: number | bigint;
}

export interface CreateRevokeInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
}

export interface CreateBurnInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
    authority: PublicKey;
    amount: number | bigint;
    /** When set, emits BurnChecked; otherwise Burn. */
    decimals?: number;
}

/** Single freeze ix (hot token account address already known). */
export interface CreateRawFreezeInstructionInput {
    tokenAccount: PublicKey;
    mint: PublicKey;
    freezeAuthority: PublicKey;
}

/** Single thaw ix (hot token account address already known). */
export interface CreateRawThawInstructionInput {
    tokenAccount: PublicKey;
    mint: PublicKey;
    freezeAuthority: PublicKey;
}

export interface CreateFreezeInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
    freezeAuthority: PublicKey;
}

export interface CreateThawInstructionsInput extends AtaOwnerInput {
    rpc: Rpc;
    payer?: PublicKey;
    freezeAuthority: PublicKey;
}

export type CreateRawAtaInstructionInput = CreateAtaInstructionsInput;
export type CreateRawLoadInstructionInput = CreateLoadInstructionsInput;

export interface CreateRawTransferInstructionInput {
    source: PublicKey;
    destination: PublicKey;
    mint: PublicKey;
    authority: PublicKey;
    payer?: PublicKey;
    amount: number | bigint;
    decimals: number;
}

/** Light-token CTokenBurn (hot account only). `mint` is the CMint account. */
export interface CreateRawBurnInstructionInput {
    source: PublicKey;
    mint: PublicKey;
    authority: PublicKey;
    amount: number | bigint;
    payer?: PublicKey;
}

/** Light-token CTokenBurnChecked (hot account only). */
export interface CreateRawBurnCheckedInstructionInput
    extends CreateRawBurnInstructionInput {
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

export interface CreateRawMintInstructionInput {
    mint: PublicKey;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority?: PublicKey | null;
    tokenProgramId?: PublicKey;
}

export interface CreateMintInstructionsInput {
    rpc: Rpc;
    payer: PublicKey;
    keypair: PublicKey | Signer;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority?: PublicKey | null;
    tokenProgramId?: PublicKey;
    mintSize?: number;
    rentExemptBalance?: number;
    splInterfaceIndex?: number;
    tokenMetadata?: TokenMetadataInput;
    outputStateTreeInfo?: TreeInfo;
    addressTreeInfo?: AddressTreeInfo;
    maxTopUp?: number;
}

export interface CreateRawMintToInstructionInput {
    mint: PublicKey;
    destination: PublicKey;
    authority: PublicKey;
    amount: number | bigint;
    payer?: PublicKey;
    tokenProgramId?: PublicKey;
    multiSigners?: PublicKey[];
    maxTopUp?: number;
}

export interface CreateMintToInstructionsInput
    extends Omit<CreateRawMintToInstructionInput, 'tokenProgramId'> {
    tokenProgramId?: PublicKey;
}

export interface TokenMetadataInput {
    name: string;
    symbol: string;
    uri: string;
    updateAuthority?: PublicKey | null;
    additionalMetadata?: { key: string; value: string }[] | null;
}

export interface CreateRawLightMintInstructionInput {
    mintSigner: PublicKey;
    decimals: number;
    mintAuthority: PublicKey;
    freezeAuthority?: PublicKey | null;
    payer: PublicKey;
    validityProof: ValidityProofWithContext;
    addressTreeInfo: AddressTreeInfo;
    outputStateTreeInfo: TreeInfo;
    tokenMetadata?: TokenMetadataInput;
    maxTopUp?: number;
}
