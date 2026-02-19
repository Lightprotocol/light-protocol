/**
 * Exports for @lightprotocol/compressed-token/unified
 *
 * Import from `/unified` to get a single unified ATA for SPL/T22 and c-token
 * mints.
 */
import {
    PublicKey,
    Signer,
    ConfirmOptions,
    Commitment,
    ComputeBudgetProgram,
    TransactionInstruction,
} from '@solana/web3.js';
import {
    Rpc,
    LIGHT_TOKEN_PROGRAM_ID,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';

import {
    getAtaInterface as _getAtaInterface,
    AccountInterface,
} from '../get-account-interface';
import { getAssociatedTokenAddressInterface as _getAssociatedTokenAddressInterface } from '../get-associated-token-address-interface';
import {
    createLoadAtaInstructions as _createLoadAtaInstructions,
    loadAta as _loadAta,
} from '../actions/load-ata';
import { createAssociatedTokenAccountInterfaceIdempotentInstruction } from '../instructions/create-ata-interface';
import {
    transferInterface as _transferInterface,
    createTransferInterfaceInstructions as _createTransferInterfaceInstructions,
} from '../actions/transfer-interface';
import type { TransferOptions as _TransferOptions } from '../actions/transfer-interface';
import { _getOrCreateAtaInterface } from '../actions/get-or-create-ata-interface';
import {
    createUnwrapInstructions as _createUnwrapInstructions,
    unwrap as _unwrap,
} from '../actions/unwrap';
import { SplInterfaceInfo } from '../../utils/get-token-pool-infos';
import { getAtaProgramId } from '../ata-utils';
import { InterfaceOptions } from '..';

/**
 * Get associated token account with unified balance
 *
 * @param rpc         RPC connection
 * @param ata         Associated token address
 * @param owner       Owner public key
 * @param mint        Mint public key
 * @param commitment  Optional commitment level
 * @param programId   Optional program ID (omit for unified behavior)
 * @returns AccountInterface with aggregated balance from all sources
 */
export async function getAtaInterface(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<AccountInterface> {
    return _getAtaInterface(rpc, ata, owner, mint, commitment, programId, true);
}

/**
 * Derive the canonical token ATA for SPL/T22/c-token in the unified path.
 *
 * Enforces LIGHT_TOKEN_PROGRAM_ID.
 *
 * @param mint                      Mint public key
 * @param owner                     Owner public key
 * @param allowOwnerOffCurve        Allow owner to be a PDA. Default false.
 * @param programId                 Token program ID. Default c-token.
 * @param associatedTokenProgramId  Associated token program ID. Default
 *                                  auto-detected.
 * @returns                         Associated token address.
 */
export function getAssociatedTokenAddressInterface(
    mint: PublicKey,
    owner: PublicKey,
    allowOwnerOffCurve = false,
    programId: PublicKey = LIGHT_TOKEN_PROGRAM_ID,
    associatedTokenProgramId?: PublicKey,
): PublicKey {
    if (!programId.equals(LIGHT_TOKEN_PROGRAM_ID)) {
        throw new Error(
            'Please derive the unified ATA from the c-token program; balances across SPL, T22, and c-token are unified under the canonical c-token ATA.',
        );
    }

    return _getAssociatedTokenAddressInterface(
        mint,
        owner,
        allowOwnerOffCurve,
        programId,
        associatedTokenProgramId,
    );
}

/**
 * Create instruction batches for loading ALL token balances into a c-token ATA.
 *
 * @param rpc     RPC connection
 * @param ata     Associated token address
 * @param owner   Owner public key
 * @param mint    Mint public key
 * @param payer   Fee payer (defaults to owner)
 * @param options Optional interface options
 * @returns Instruction batches - each inner array is one transaction
 */
export async function createLoadAtaInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    options?: InterfaceOptions,
): Promise<TransactionInstruction[][]> {
    return _createLoadAtaInstructions(
        rpc,
        ata,
        owner,
        mint,
        payer,
        options,
        true,
    );
}

/**
 * Load all token balances into the c-token ATA.
 *
 * Wraps SPL/Token-2022 balances and decompresses compressed c-tokens
 * into the on-chain c-token ATA. If no balances exist and the ATA doesn't
 * exist, creates an empty ATA (idempotent).
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (c-token)
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @returns Transaction signature, or null if ATA exists and nothing to load
 */
export async function loadAta(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
) {
    payer ??= owner;

    const signature = await _loadAta(
        rpc,
        ata,
        owner,
        mint,
        payer,
        confirmOptions,
        interfaceOptions,
        true,
    );

    // If nothing to load, ensure ATA exists (idempotent).
    if (signature === null) {
        const accountInfo = await rpc.getAccountInfo(ata);
        if (!accountInfo) {
            const ix =
                createAssociatedTokenAccountInterfaceIdempotentInstruction(
                    payer.publicKey,
                    ata,
                    owner.publicKey,
                    mint,
                    LIGHT_TOKEN_PROGRAM_ID,
                );
            const { blockhash } = await rpc.getLatestBlockhash();
            const tx = buildAndSignTx(
                [
                    ComputeBudgetProgram.setComputeUnitLimit({ units: 30_000 }),
                    ix,
                ],
                payer,
                blockhash,
                payer.publicKey.equals(owner.publicKey) ? [] : [owner],
            );
            return sendAndConfirmTx(rpc, tx, confirmOptions);
        }
    }

    return signature;
}

/**
 * Transfer tokens using the unified ata interface.
 *
 * Destination ATA must exist. Automatically wraps SPL/T22 to c-token ATA.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source c-token ATA address
 * @param mint            Mint address
 * @param destination     Destination c-token ATA address (must exist)
 * @param owner           Source owner (signer)
 * @param amount          Amount to transfer
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @returns Transaction signature
 */
export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    mint: PublicKey,
    destination: PublicKey,
    owner: Signer,
    amount: number | bigint | BN,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
) {
    return _transferInterface(
        rpc,
        payer,
        source,
        mint,
        destination,
        owner,
        amount,
        undefined, // programId: use default LIGHT_TOKEN_PROGRAM_ID
        confirmOptions,
        options,
        true, // wrap=true for unified
    );
}

/**
 * Get or create c-token ATA with unified balance detection and auto-loading.
 *
 * Enforces LIGHT_TOKEN_PROGRAM_ID. Aggregates balances from:
 * - c-token hot (on-chain) account
 * - c-token cold (compressed) accounts
 * - SPL token accounts (for unified wrapping)
 * - Token-2022 accounts (for unified wrapping)
 *
 * When owner is a Signer:
 * - Creates hot ATA if it doesn't exist
 * - Loads cold (compressed) tokens into hot ATA
 * - Wraps SPL/T22 tokens into c-token ATA
 * - Returns account with all tokens ready to use
 *
 * When owner is a PublicKey:
 * - Creates hot ATA if it doesn't exist
 * - Returns aggregated balance but does NOT auto-load (can't sign)
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer
 * @param mint            Mint address
 * @param owner           Owner (Signer for auto-load, PublicKey for read-only)
 * @param allowOwnerOffCurve Allow PDA owners (default: false)
 * @param commitment      Optional commitment level
 * @param confirmOptions  Optional confirm options
 * @returns AccountInterface with unified balance and source breakdown
 */
export async function getOrCreateAtaInterface(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    owner: PublicKey | Signer,
    allowOwnerOffCurve = false,
    commitment?: Commitment,
    confirmOptions?: ConfirmOptions,
): Promise<AccountInterface> {
    return _getOrCreateAtaInterface(
        rpc,
        payer,
        mint,
        owner,
        allowOwnerOffCurve,
        commitment,
        confirmOptions,
        LIGHT_TOKEN_PROGRAM_ID,
        getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID),
        true, // wrap=true for unified path
    );
}

/**
 * Create transfer instructions for a unified token transfer.
 *
 * Unified variant: always wraps SPL/T22 to c-token ATA.
 *
 * Returns `TransactionInstruction[][]`. Send [0..n-2] in parallel, then [n-1].
 * Use `sliceLast` to separate the parallel prefix from the final transfer.
 *
 * @see createTransferInterfaceInstructions in v3/actions/transfer-interface.ts
 */
export async function createTransferInterfaceInstructions(
    rpc: Rpc,
    payer: PublicKey,
    mint: PublicKey,
    amount: number | bigint | BN,
    sender: PublicKey,
    recipient: PublicKey,
    options?: Omit<_TransferOptions, 'wrap'>,
): Promise<TransactionInstruction[][]> {
    return _createTransferInterfaceInstructions(
        rpc,
        payer,
        mint,
        amount,
        sender,
        recipient,
        {
            ...options,
            wrap: true,
        },
    );
}

/**
 * Build instruction batches for unwrapping c-tokens to SPL/T22.
 *
 * Unified variant: uses wrap=true for loading, so SPL/T22 balances are
 * consolidated before unwrapping.
 *
 * Returns `TransactionInstruction[][]`. Load batches (if any) come first,
 * followed by one final unwrap transaction.
 *
 * @param rpc               RPC connection
 * @param destination       Destination SPL/T22 token account (must exist)
 * @param owner             Owner of the c-token
 * @param mint              Mint address
 * @param amount            Amount to unwrap (defaults to full balance)
 * @param payer             Fee payer (defaults to owner)
 * @param splInterfaceInfo  Optional: SPL interface info
 * @param interfaceOptions  Optional: interface options for load
 * @returns Instruction batches - each inner array is one transaction
 */
export async function createUnwrapInstructions(
    rpc: Rpc,
    destination: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    amount?: number | bigint | BN,
    payer?: PublicKey,
    splInterfaceInfo?: SplInterfaceInfo,
    interfaceOptions?: InterfaceOptions,
): Promise<TransactionInstruction[][]> {
    return _createUnwrapInstructions(
        rpc,
        destination,
        owner,
        mint,
        amount,
        payer,
        splInterfaceInfo,
        undefined, // maxTopUp - use default
        interfaceOptions,
        true, // wrap=true for unified
    );
}

/**
 * Unwrap c-tokens to SPL tokens.
 *
 * Unified variant: loads all cold + SPL/T22 balances to c-token ATA first,
 * then unwraps to the destination SPL/T22 account.
 *
 * @param rpc                RPC connection
 * @param payer              Fee payer
 * @param destination        Destination SPL/T22 token account
 * @param owner              Owner of the c-token (signer)
 * @param mint               Mint address
 * @param amount             Amount to unwrap (defaults to all)
 * @param splInterfaceInfo   SPL interface info
 * @param confirmOptions     Confirm options
 * @returns Transaction signature of the unwrap transaction
 */
export async function unwrap(
    rpc: Rpc,
    payer: Signer,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount?: number | bigint | BN,
    splInterfaceInfo?: SplInterfaceInfo,
    confirmOptions?: ConfirmOptions,
): Promise<string> {
    return _unwrap(
        rpc,
        payer,
        destination,
        owner,
        mint,
        amount,
        splInterfaceInfo,
        undefined, // maxTopUp - use default
        confirmOptions,
    );
}

export type { _TransferOptions as TransferOptions };

export {
    getAccountInterface,
    AccountInterface,
    TokenAccountSource,
    // Note: Account is already exported from @solana/spl-token via get-account-interface
    AccountState,
    ParsedTokenAccount,
    parseCTokenHot,
    parseCTokenCold,
    toAccountInfo,
    convertTokenDataToAccount,
} from '../get-account-interface';

export {
    createLoadAccountsParams,
    calculateCompressibleLoadComputeUnits,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    CompressibleLoadParams,
    PackedCompressedAccount,
    LoadResult,
} from '../actions/load-ata';

export { InterfaceOptions, sliceLast } from '../actions/transfer-interface';

export * from '../../actions';
export * from '../../utils';
export * from '../../constants';
export * from '../../idl';
export * from '../../layout';
export * from '../../program';
export * from '../../types';
export * from '../derivation';

export {
    // Instructions
    createMintInstruction,
    createTokenMetadata,
    createAssociatedCTokenAccountInstruction,
    createAssociatedCTokenAccountIdempotentInstruction,
    createAssociatedTokenAccountInterfaceInstruction,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    createAtaInterfaceIdempotentInstruction,
    createMintToInstruction,
    createMintToCompressedInstruction,
    createMintToInterfaceInstruction,
    createUpdateMintAuthorityInstruction,
    createUpdateFreezeAuthorityInstruction,
    createUpdateMetadataFieldInstruction,
    createUpdateMetadataAuthorityInstruction,
    createRemoveMetadataKeyInstruction,
    createWrapInstruction,
    createUnwrapInstruction,
    createLightTokenTransferInstruction,
    // Types
    TokenMetadataInstructionData,
    CompressibleConfig,
    CTokenConfig,
    CreateAssociatedCTokenAccountParams,
    // Constants for rent sponsor
    DEFAULT_COMPRESSIBLE_CONFIG,
    // Actions
    createMintInterface,
    createAtaInterface,
    createAtaInterfaceIdempotent,
    // getOrCreateAtaInterface is defined locally with unified behavior
    wrap,
    // unwrap and createUnwrapInstructions are defined locally with unified behavior
    mintTo as mintToCToken,
    mintToCompressed,
    mintToInterface,
    updateMintAuthority,
    updateFreezeAuthority,
    updateMetadataField,
    updateMetadataAuthority,
    removeMetadataKey,
    // Helpers
    getMintInterface,
    unpackMintInterface,
    unpackMintData,
    MintInterface,
    // Serde
    BaseMint,
    MintContext,
    MintExtension,
    TokenMetadata,
    CompressedMint,
    deserializeMint,
    serializeMint,
    decodeTokenMetadata,
    encodeTokenMetadata,
    extractTokenMetadata,
    ExtensionType,
    // Metadata formatting
    toOffChainMetadataJson,
    OffChainTokenMetadata,
    OffChainTokenMetadataJson,
} from '..';
