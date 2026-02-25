import type {
    Commitment,
    PublicKey,
    Signer,
    ConfirmOptions,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import type { Rpc } from '@lightprotocol/stateless.js';
import type {
    AccountInterface as MintAccountInterface,
    InterfaceOptions,
} from './v3';
import { getAtaInterface as _mintGetAtaInterface } from './v3';

export * from './actions';
export * from './utils';
export * from './constants';
export * from './idl';
export * from './layout';
export * from './program';
export { CompressedTokenProgram as LightTokenProgram } from './program';
export * from './types';
import {
    createLoadAccountsParams,
    createLoadAtaInstructionsFromInterface,
    createLoadAtaInstructions as _createLoadAtaInstructions,
    loadAta as _loadAta,
    calculateCompressibleLoadComputeUnits,
    selectInputsForAmount,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    CompressibleLoadParams,
    PackedCompressedAccount,
    LoadResult,
} from './v3/actions/load-ata';

export {
    createLoadAccountsParams,
    createLoadAtaInstructionsFromInterface,
    calculateCompressibleLoadComputeUnits,
    selectInputsForAmount,
    CompressibleAccountInput,
    ParsedAccountInfoInterface,
    CompressibleLoadParams,
    PackedCompressedAccount,
    LoadResult,
};

export {
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
    MAX_COMBINED_BATCH_BYTES,
    MAX_LOAD_ONLY_BATCH_BYTES,
} from './v3/utils/estimate-tx-size';

// Export mint module with explicit naming to avoid conflicts
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
    createUnwrapInstructions,
    createDecompressInterfaceInstruction,
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
    getAssociatedTokenAddressInterface,
    getOrCreateAtaInterface,
    transferInterface,
    createTransferInterfaceInstructions,
    sliceLast,
    decompressInterface,
    wrap,
    unwrap,
    decompressMint,
    mintTo as mintToCToken,
    mintToCompressed,
    mintToInterface,
    updateMintAuthority,
    updateFreezeAuthority,
    updateMetadataField,
    updateMetadataAuthority,
    removeMetadataKey,
    // Action types
    InterfaceOptions,
    // Helpers
    getMintInterface,
    unpackMintInterface,
    unpackMintData,
    MintInterface,
    getAccountInterface,
    Account,
    AccountState,
    ParsedTokenAccount as ParsedTokenAccountInterface,
    parseCTokenHot,
    parseCTokenCold,
    toAccountInfo,
    convertTokenDataToAccount,
    // Types
    AccountInterface,
    TokenAccountSource,
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
    // Metadata formatting (for use with any uploader)
    toOffChainMetadataJson,
    OffChainTokenMetadata,
    OffChainTokenMetadataJson,
} from './v3';

/**
 * Retrieve associated token account for a given owner and mint.
 *
 * @param rpc         RPC connection
 * @param ata         Associated token address
 * @param owner       Owner public key
 * @param mint        Mint public key
 * @param commitment  Optional commitment level
 * @param programId   Optional program ID
 * @returns AccountInterface with ATA metadata
 */
export async function getAtaInterface(
    rpc: Rpc,
    ata: PublicKey,
    owner: PublicKey,
    mint: PublicKey,
    commitment?: Commitment,
    programId?: PublicKey,
): Promise<MintAccountInterface> {
    return _mintGetAtaInterface(rpc, ata, owner, mint, commitment, programId);
}

/**
 * Create instruction batches for loading token balances into an ATA.
 * Returns batches of instructions, each batch is one transaction.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address
 * @param owner             Owner public key
 * @param mint              Mint public key
 * @param payer             Fee payer (defaults to owner)
 * @param options           Optional load options
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
        false,
    );
}

/**
 * Load token balances into a c-token ATA.
 *
 * @param rpc               RPC connection
 * @param ata               Associated token address
 * @param owner             Owner of the tokens (signer)
 * @param mint              Mint public key
 * @param payer             Fee payer (signer, defaults to owner)
 * @param confirmOptions    Optional confirm options
 * @param interfaceOptions  Optional interface options
 * @returns Transaction signature, or null if nothing to load
 */
export async function loadAta(
    rpc: Rpc,
    ata: PublicKey,
    owner: Signer,
    mint: PublicKey,
    payer?: Signer,
    confirmOptions?: ConfirmOptions,
    interfaceOptions?: InterfaceOptions,
): Promise<TransactionSignature | null> {
    return _loadAta(
        rpc,
        ata,
        owner,
        mint,
        payer,
        confirmOptions,
        interfaceOptions,
        false,
    );
}
