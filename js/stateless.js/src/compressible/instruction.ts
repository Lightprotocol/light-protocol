import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
    AccountMeta,
} from '@solana/web3.js';
import {
    CompressionConfigIxData,
    UpdateCompressionConfigData,
    GenericCompressAccountInstruction,
    DecompressMultipleAccountsIdempotentData,
} from './types';
import {
    InitializeCompressionConfigSchema,
    UpdateCompressionConfigSchema,
    GenericCompressAccountInstructionSchema,
    createDecompressMultipleAccountsIdempotentDataSchema,
    serializeInstructionData,
} from './layout';
import {
    deriveCompressionConfigAddress,
    getProgramDataAccount,
    checkProgramUpdateAuthority,
} from './utils';
import { serializeInitializeCompressionConfigData } from './layout';
import { COMPRESSIBLE_DISCRIMINATORS, CompressedAccountData } from './types';
import { CompressedAccount } from '../state/compressed-account';
import {
    PackedStateTreeInfo,
    CompressedAccountMeta,
} from '../state/compressed-account';

/**
 * Create an instruction to initialize a compression config.
 *
 * @param programId         Program ID for the compressible program
 * @param discriminator     Instruction discriminator (8 bytes)
 * @param payer             Fee payer
 * @param authority         Program upgrade authority
 * @param compressionDelay  Compression delay (in slots)
 * @param rentRecipient     Rent recipient public key
 * @param addressSpace      Array of address space public keys
 * @param configBump        Optional config bump (defaults to 0)
 * @returns                 TransactionInstruction
 */
export function createInitializeCompressionConfigInstruction(
    programId: PublicKey,
    discriminator: Uint8Array | number[],
    payer: PublicKey,
    authority: PublicKey,
    compressionDelay: number,
    rentRecipient: PublicKey,
    addressSpace: PublicKey[],
    configBump: number | null = null,
): TransactionInstruction {
    const actualConfigBump = configBump ?? 0;
    const [configPda] = deriveCompressionConfigAddress(
        programId,
        actualConfigBump,
    );

    // Get program data account for BPF Loader Upgradeable
    const bpfLoaderUpgradeableId = new PublicKey(
        'BPFLoaderUpgradeab1e11111111111111111111111',
    );
    const [programDataPda] = PublicKey.findProgramAddressSync(
        [programId.toBuffer()],
        bpfLoaderUpgradeableId,
    );

    const accounts = [
        { pubkey: payer, isSigner: true, isWritable: true }, // payer
        { pubkey: configPda, isSigner: false, isWritable: true }, // config
        { pubkey: programDataPda, isSigner: false, isWritable: false }, // program_data
        { pubkey: authority, isSigner: true, isWritable: false }, // authority
        {
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        }, // system_program
    ];

    const instructionData: CompressionConfigIxData = {
        compressionDelay,
        rentRecipient,
        addressSpace,
        configBump: actualConfigBump,
    };

    const data = serializeInstructionData(
        InitializeCompressionConfigSchema,
        instructionData,
        discriminator,
    );

    return new TransactionInstruction({
        programId,
        keys: accounts,
        data,
    });
}

/**
 * Create an instruction to update a compression config.
 *
 * @param programId             Program ID for the compressible program
 * @param discriminator         Instruction discriminator (8 bytes)
 * @param authority             Current config authority
 * @param newCompressionDelay   Optional new compression delay
 * @param newRentRecipient      Optional new rent recipient
 * @param newAddressSpace       Optional new address space array
 * @param newUpdateAuthority    Optional new update authority
 * @returns                     TransactionInstruction
 */
export function createUpdateCompressionConfigInstruction(
    programId: PublicKey,
    discriminator: Uint8Array | number[],
    authority: PublicKey,
    newCompressionDelay: number | null = null,
    newRentRecipient: PublicKey | null = null,
    newAddressSpace: PublicKey[] | null = null,
    newUpdateAuthority: PublicKey | null = null,
): TransactionInstruction {
    const [configPda] = deriveCompressionConfigAddress(programId, 0);

    const accounts = [
        { pubkey: configPda, isSigner: false, isWritable: true }, // config
        { pubkey: authority, isSigner: true, isWritable: false }, // authority
    ];

    const instructionData: UpdateCompressionConfigData = {
        newCompressionDelay,
        newRentRecipient,
        newAddressSpace,
        newUpdateAuthority,
    };

    const data = serializeInstructionData(
        UpdateCompressionConfigSchema,
        instructionData,
        discriminator,
    );

    return new TransactionInstruction({
        programId,
        keys: accounts,
        data,
    });
}

/**
 * Create an instruction to compress a generic compressible account.
 *
 * @param programId             Program ID for the compressible program
 * @param discriminator         Instruction discriminator (8 bytes)
 * @param payer                 Fee payer
 * @param pdaToCompress         PDA to compress
 * @param rentRecipient         Rent recipient public key
 * @param compressedAccountMeta Compressed account metadata
 * @param validityProof         Validity proof for compression
 * @param systemAccounts        Additional system accounts (optional)
 * @returns                     TransactionInstruction
 */
export function createCompressAccountInstruction(
    programId: PublicKey,
    discriminator: Uint8Array | number[],
    payer: PublicKey,
    pdaToCompress: PublicKey,
    rentRecipient: PublicKey,
    compressedAccountMeta: import('../state/compressed-account').CompressedAccountMeta,
    validityProof: import('../state/types').ValidityProof,
    systemAccounts: AccountMeta[] = [],
): TransactionInstruction {
    const [configPda] = deriveCompressionConfigAddress(programId, 0);

    // Create the instruction account metas
    const accounts = [
        { pubkey: payer, isSigner: true, isWritable: true }, // user (signer)
        { pubkey: pdaToCompress, isSigner: false, isWritable: true }, // pda_to_compress (writable)
        { pubkey: configPda, isSigner: false, isWritable: false }, // config
        { pubkey: rentRecipient, isSigner: false, isWritable: true }, // rent_recipient (writable)
        ...systemAccounts, // Additional system accounts (trees, queues, etc.)
    ];

    const instructionData: GenericCompressAccountInstruction = {
        proof: validityProof,
        compressedAccountMeta,
    };

    const data = serializeInstructionData(
        GenericCompressAccountInstructionSchema,
        instructionData,
        discriminator,
    );

    return new TransactionInstruction({
        programId,
        keys: accounts,
        data,
    });
}

/**
 * Create an instruction to decompress one or more compressed accounts idempotently.
 *
 * @param programId                 Program ID for the compressible program
 * @param discriminator             Instruction discriminator (8 bytes)
 * @param feePayer                  Fee payer
 * @param rentPayer                 Rent payer
 * @param solanaAccounts            Array of PDA accounts to decompress
 * @param compressedAccountsData    Array of compressed account data
 * @param bumps                     Array of PDA bumps
 * @param validityProof             Validity proof for decompression
 * @param systemAccounts            Additional system accounts (optional)
 * @param coder                Borsh schema for account data
 * @returns                         TransactionInstruction
 */
export function createDecompressAccountsIdempotentInstruction<T = any>(
    programId: PublicKey,
    discriminator: Uint8Array | number[],
    feePayer: PublicKey,
    rentPayer: PublicKey,
    solanaAccounts: PublicKey[],
    compressedAccountsData: import('./types').CompressedAccountData<T>[],
    bumps: number[],
    validityProof: import('../state/types').ValidityProof,
    systemAccounts: AccountMeta[] = [],
    coder: (data: any) => Buffer,
): TransactionInstruction {
    // Validation
    if (solanaAccounts.length !== compressedAccountsData.length) {
        throw new Error(
            'PDA accounts and compressed accounts must have the same length',
        );
    }
    if (solanaAccounts.length !== bumps.length) {
        throw new Error('PDA accounts and bumps must have the same length');
    }

    const [configPda] = deriveCompressionConfigAddress(programId, 0);

    // Build instruction accounts
    const accounts: AccountMeta[] = [
        { pubkey: feePayer, isSigner: true, isWritable: true }, // fee_payer
        { pubkey: rentPayer, isSigner: true, isWritable: true }, // rent_payer
        { pubkey: configPda, isSigner: false, isWritable: false }, // config
        ...systemAccounts, // Light Protocol system accounts (trees, queues, etc.)
    ];

    // Build instruction data
    const instructionData: DecompressMultipleAccountsIdempotentData<T> = {
        proof: validityProof,
        compressedAccounts: compressedAccountsData,
        bumps,
        systemAccountsOffset: solanaAccounts.length,
    };

    const data = coder(instructionData);

    return new TransactionInstruction({
        programId,
        keys: accounts,
        data,
    });
}

/**
 * Instruction builders for compressible accounts, following Solana SDK patterns.
 */
export class CompressibleInstruction {
    /**
     * Create an instruction to initialize a compression config.
     *
     * @param programId         Program ID for the compressible program
     * @param discriminator     Instruction discriminator (8 bytes)
     * @param payer             Fee payer
     * @param authority         Program upgrade authority
     * @param compressionDelay  Compression delay (in slots)
     * @param rentRecipient     Rent recipient public key
     * @param addressSpace      Array of address space public keys
     * @param configBump        Optional config bump (defaults to 0)
     * @returns                 TransactionInstruction
     */
    static initializeCompressionConfig(
        programId: PublicKey,
        discriminator: Uint8Array | number[],
        payer: PublicKey,
        authority: PublicKey,
        compressionDelay: number,
        rentRecipient: PublicKey,
        addressSpace: PublicKey[],
        configBump: number | null = null,
    ): TransactionInstruction {
        return createInitializeCompressionConfigInstruction(
            programId,
            discriminator,
            payer,
            authority,
            compressionDelay,
            rentRecipient,
            addressSpace,
            configBump,
        );
    }

    /**
     * Create an instruction to update a compression config.
     *
     * @param programId             Program ID for the compressible program
     * @param discriminator         Instruction discriminator (8 bytes)
     * @param authority             Current config authority
     * @param newCompressionDelay   Optional new compression delay
     * @param newRentRecipient      Optional new rent recipient
     * @param newAddressSpace       Optional new address space array
     * @param newUpdateAuthority    Optional new update authority
     * @returns                     TransactionInstruction
     */
    static updateCompressionConfig(
        programId: PublicKey,
        discriminator: Uint8Array | number[],
        authority: PublicKey,
        newCompressionDelay: number | null = null,
        newRentRecipient: PublicKey | null = null,
        newAddressSpace: PublicKey[] | null = null,
        newUpdateAuthority: PublicKey | null = null,
    ): TransactionInstruction {
        return createUpdateCompressionConfigInstruction(
            programId,
            discriminator,
            authority,
            newCompressionDelay,
            newRentRecipient,
            newAddressSpace,
            newUpdateAuthority,
        );
    }

    /**
     * Create an instruction to compress a generic compressible account.
     *
     * @param programId             Program ID for the compressible program
     * @param discriminator         Instruction discriminator (8 bytes)
     * @param payer                 Fee payer
     * @param pdaToCompress         PDA to compress
     * @param rentRecipient         Rent recipient public key
     * @param compressedAccountMeta Compressed account metadata
     * @param validityProof         Validity proof for compression
     * @param systemAccounts        Additional system accounts (optional)
     * @returns                     TransactionInstruction
     */
    static compressAccount(
        programId: PublicKey,
        discriminator: Uint8Array | number[],
        payer: PublicKey,
        pdaToCompress: PublicKey,
        rentRecipient: PublicKey,
        compressedAccountMeta: import('../state/compressed-account').CompressedAccountMeta,
        validityProof: import('../state/types').ValidityProof,
        systemAccounts: AccountMeta[] = [],
    ): TransactionInstruction {
        return createCompressAccountInstruction(
            programId,
            discriminator,
            payer,
            pdaToCompress,
            rentRecipient,
            compressedAccountMeta,
            validityProof,
            systemAccounts,
        );
    }

    /**
     * Create an instruction to decompress one or more compressed accounts idempotently.
     *
     * @param programId                 Program ID for the compressible program
     * @param discriminator             Instruction discriminator (8 bytes)
     * @param feePayer                  Fee payer
     * @param rentPayer                 Rent payer
     * @param solanaAccounts            Array of PDA accounts to decompress
     * @param compressedAccountsData    Array of compressed account data
     * @param bumps                     Array of PDA bumps
     * @param validityProof             Validity proof for decompression
     * @param systemAccounts            Additional system accounts (optional)
     * @param dataSchema                Borsh schema for account data
     * @returns                         TransactionInstruction
     */
    static decompressAccountsIdempotent<T = any>(
        programId: PublicKey,
        discriminator: Uint8Array | number[],
        feePayer: PublicKey,
        rentPayer: PublicKey,
        solanaAccounts: PublicKey[],
        compressedAccountsData: import('./types').CompressedAccountData<T>[],
        bumps: number[],
        validityProof: import('../state/types').ValidityProof,
        systemAccounts: AccountMeta[] = [],
        dataSchema?: any,
    ): TransactionInstruction {
        return createDecompressAccountsIdempotentInstruction<T>(
            programId,
            discriminator,
            feePayer,
            rentPayer,
            solanaAccounts,
            compressedAccountsData,
            bumps,
            validityProof,
            systemAccounts,
            dataSchema,
        );
    }

    /**
     * Standard instruction discriminators for compressible instructions
     */
    static readonly DISCRIMINATORS = COMPRESSIBLE_DISCRIMINATORS;

    /**
     * Derive the compression config PDA address
     *
     * @param programId     Program ID for the compressible program
     * @param configIndex   Config index (defaults to 0)
     * @returns             [PDA address, bump seed]
     */
    static deriveCompressionConfigAddress(
        programId: PublicKey,
        configIndex: number = 0,
    ): [PublicKey, number] {
        return deriveCompressionConfigAddress(programId, configIndex);
    }

    /**
     * Get the program data account address and its raw data for a given program
     *
     * @param programId     Program ID
     * @param connection    Solana connection
     * @returns             Program data address and account info
     */
    static async getProgramDataAccount(
        programId: PublicKey,
        connection: import('@solana/web3.js').Connection,
    ): Promise<{
        programDataAddress: PublicKey;
        programDataAccountInfo: import('@solana/web3.js').AccountInfo<Buffer>;
    }> {
        return await getProgramDataAccount(programId, connection);
    }

    /**
     * Check that the provided authority matches the program's upgrade authority
     *
     * @param programDataAccountInfo    Program data account info
     * @param providedAuthority         Authority to validate
     * @throws                          Error if authority doesn't match
     */
    static checkProgramUpdateAuthority(
        programDataAccountInfo: import('@solana/web3.js').AccountInfo<Buffer>,
        providedAuthority: PublicKey,
    ): void {
        checkProgramUpdateAuthority(programDataAccountInfo, providedAuthority);
    }

    /**
     * Serialize instruction data for initializeCompressionConfig using Borsh
     *
     * @param compressionDelay  Compression delay (in slots)
     * @param rentRecipient     Rent recipient public key
     * @param addressSpace      Array of address space public keys
     * @param configBump        Optional config bump
     * @returns                 Serialized instruction data with discriminator
     */
    static serializeInitializeCompressionConfigData(
        compressionDelay: number,
        rentRecipient: PublicKey,
        addressSpace: PublicKey[],
        configBump: number | null,
    ): Buffer {
        return serializeInitializeCompressionConfigData(
            compressionDelay,
            rentRecipient,
            addressSpace,
            configBump,
        );
    }

    /**
     * Convert a compressed account to the format expected by instruction builders
     *
     * @param compressedAccount     Compressed account from state
     * @param data                  Program-specific account data
     * @param seeds                 PDA seeds (without bump)
     * @param outputStateTreeIndex  Output state tree index
     * @returns                     Compressed account data for instructions
     */
    static createCompressedAccountData<T>(
        compressedAccount: CompressedAccount,
        data: T,
        seeds: Uint8Array[],
        outputStateTreeIndex: number,
    ): CompressedAccountData<T> {
        // Note: This is a simplified version. The full implementation would need
        // to handle proper tree info packing from ValidityProofWithContext
        const treeInfo: PackedStateTreeInfo = {
            rootIndex: 0, // Should be derived from ValidityProofWithContext
            proveByIndex: compressedAccount.proveByIndex,
            merkleTreePubkeyIndex: 0, // Should be derived from remaining accounts
            queuePubkeyIndex: 0, // Should be derived from remaining accounts
            leafIndex: compressedAccount.leafIndex,
        };

        const meta: CompressedAccountMeta = {
            treeInfo,
            address: compressedAccount.address
                ? Array.from(compressedAccount.address)
                : null,
            lamports: compressedAccount.lamports,
            outputStateTreeIndex,
        };

        return {
            meta,
            data,
            seeds,
        };
    }
}
