/**
 * Unified exports for @lightprotocol/compressed-token/unified
*/
import { PublicKey, Signer, ConfirmOptions, Commitment } from '@solana/web3.js';
import { Rpc, CTOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import BN from 'bn.js';

import {
    getATAInterface as _getATAInterface,
    AccountInterface,
} from '../mint/get-account-interface';
import {
    createLoadATAInstructions as _createLoadATAInstructions,
    loadATA as _loadATA,
} from '../compressible/unified-load';
import { transferInterface as _transferInterface } from '../mint/actions/transfer-interface';
import { InterfaceOptions } from '../mint';

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
export async function getATAInterface(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<AccountInterface> {
    return _getATAInterface(rpc, ata, owner, mint, commitment, programId, true);
}

/**
 * Create instructions to load ALL token balances into a CToken ATA.
 * 
 * @param rpc     RPC connection
 * @param ata     Associated token address
 * @param owner   Owner public key
 * @param mint    Mint public key
 * @param payer   Fee payer (defaults to owner)
 * @param options Optional interface options
 * @returns Array of instructions (empty if nothing to load)
 */
export async function createLoadATAInstructions(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    payer?: PublicKey,
    options?: InterfaceOptions,
) {
    return _createLoadATAInstructions(
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
 * Wraps SPL/Token-2022 balances and decompresses compressed CTokens
 * into the on-chain CToken ATA. Idempotent: returns null if nothing to load.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address (c-token)
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @returns Transaction signature, or null if nothing to load
 */
export async function loadATA(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
) {
    return _loadATA(
        rpc,
        ata,
        owner,
        mint,
        payer,
        confirmOptions,
        interfaceOptions,
        true,
    );
}

/**
 * Transfer tokens using the unified ata interface.
 *
 * @param rpc             RPC connection
 * @param payer           Fee payer (signer)
 * @param source          Source c-token ATA address
 * @param destination     Destination CToken ATA address (must exist)
 * @param owner           Source owner (signer)
 * @param mint            Mint address
 * @param amount          Amount to transfer
 * @param programId       Token program ID (default: CTOKEN_PROGRAM_ID)
 * @param confirmOptions  Optional confirm options
 * @param options         Optional interface options
 * @returns Transaction signature
 */
export async function transferInterface(
    rpc: Rpc,
    payer: Signer,
    source: PublicKey,
    destination: PublicKey,
    owner: Signer,
    mint: PublicKey,
    amount: number | bigint | BN,
    programId: PublicKey = CTOKEN_PROGRAM_ID,
    confirmOptions?: ConfirmOptions,
    options?: InterfaceOptions,
) {
    return _transferInterface(
        rpc,
        payer,
        source,
        destination,
        owner,
        mint,
        amount,
        programId,
        confirmOptions,
        options,
        true,
    );
}

export {
    getAssociatedTokenAddressInterface,
    getAccountInterface,
    AccountInterface,
    TokenAccountSource,
    Account,
    AccountState,
    ParsedTokenAccount,
    parseCTokenHot,
    parseCTokenCold,
    toAccountInfo,
    convertTokenDataToAccount,
} from '../mint/get-account-interface';

export {
    createLoadAccountsParams,
    createLoadATAInstructionsFromInterface,
    calculateCompressibleLoadComputeUnits,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    CompressibleLoadParams,
    PackedCompressedAccount,
    LoadResult,
} from '../compressible/unified-load';

export {
    LoadOptions,
    TransferInterfaceOptions,
    InterfaceOptions,
} from '../mint/actions/transfer-interface';

export * from '../actions';
export * from '../utils';
export * from '../constants';
export * from '../idl';
export * from '../layout';
export * from '../program';
export * from '../types';
export * from '../compressible';

export {
    // Instructions
    createMintInstruction,
    createTokenMetadata,
    createAssociatedCTokenAccountInstruction,
    createAssociatedCTokenAccountIdempotentInstruction,
    createAssociatedTokenAccountInterfaceInstruction,
    createAssociatedTokenAccountInterfaceIdempotentInstruction,
    createATAInterfaceIdempotentInstruction,
    createMintToInstruction,
    createMintToCompressedInstruction,
    createMintToInterfaceInstruction,
    createUpdateMintAuthorityInstruction,
    createUpdateFreezeAuthorityInstruction,
    createUpdateMetadataFieldInstruction,
    createUpdateMetadataAuthorityInstruction,
    createRemoveMetadataKeyInstruction,
    createWrapInstruction,
    createTransferInterfaceInstruction,
    createCTokenTransferInstruction,
    // Types
    TokenMetadataInstructionData,
    CompressibleConfig,
    CTokenConfig,
    CreateAssociatedCTokenAccountParams,
    // Actions
    createMintInterface,
    createATAInterface,
    createATAInterfaceIdempotent,
    getOrCreateATAInterface,
    decompress2,
    wrap,
    mintTo as mintToCToken,
    mintToCompressed,
    mintToInterface,
    updateMintAuthority,
    updateFreezeAuthority,
    updateMetadataField,
    updateMetadataAuthority,
    removeMetadataKey,
    // Action types
    CreateATAInterfaceParams,
    CreateATAInterfaceResult,
    WrapParams,
    WrapResult,
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
} from '../mint';
