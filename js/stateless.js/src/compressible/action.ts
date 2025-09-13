import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
    AccountMeta,
} from '@solana/web3.js';
import { sendAndConfirmTx, buildAndSignTx, dedupeSigner } from '../utils';
import { Rpc } from '../rpc';
import { ValidityProof } from '../state/types';
import { CompressedAccountMeta } from '../state/compressed-account';
import {
    createInitializeCompressionConfigInstruction,
    createUpdateCompressionConfigInstruction,
    createCompressAccountInstruction,
    createDecompressAccountsIdempotentInstruction,
} from './instruction';
import { COMPRESSIBLE_DISCRIMINATORS, CompressedAccountData } from './types';

/**
 * Initialize a compression config for a compressible program
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer
 * @param programId             Program ID for the compressible program
 * @param authority             Program upgrade authority
 * @param compressionDelay      Compression delay (in slots)
 * @param rentRecipient         Rent recipient public key
 * @param addressSpace          Array of address space public keys
 * @param configBump            Optional config bump (defaults to 0)
 * @param discriminator         Optional custom discriminator (defaults to standard)
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function initializeCompressionConfig(
    rpc: Rpc,
    payer: Signer,
    programId: PublicKey,
    authority: Signer,
    compressionDelay: number,
    rentRecipient: PublicKey,
    addressSpace: PublicKey[],
    configBump: number | null = null,
    discriminator:
        | Uint8Array
        | number[] = COMPRESSIBLE_DISCRIMINATORS.INITIALIZE_COMPRESSION_CONFIG as unknown as number[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const ix = createInitializeCompressionConfigInstruction(
        programId,
        discriminator,
        payer.publicKey,
        authority.publicKey,
        compressionDelay,
        rentRecipient,
        addressSpace,
        configBump,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 200_000,
            }),
            ix,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Update a compression config for a compressible program
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer
 * @param programId             Program ID for the compressible program
 * @param authority             Current config authority
 * @param newCompressionDelay   Optional new compression delay
 * @param newRentRecipient      Optional new rent recipient
 * @param newAddressSpace       Optional new address space array
 * @param newUpdateAuthority    Optional new update authority
 * @param discriminator         Optional custom discriminator (defaults to standard)
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function updateCompressionConfig(
    rpc: Rpc,
    payer: Signer,
    programId: PublicKey,
    authority: Signer,
    newCompressionDelay: number | null = null,
    newRentRecipient: PublicKey | null = null,
    newAddressSpace: PublicKey[] | null = null,
    newUpdateAuthority: PublicKey | null = null,
    discriminator:
        | Uint8Array
        | number[] = COMPRESSIBLE_DISCRIMINATORS.UPDATE_COMPRESSION_CONFIG as unknown as number[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const ix = createUpdateCompressionConfigInstruction(
        programId,
        discriminator,
        authority.publicKey,
        newCompressionDelay,
        newRentRecipient,
        newAddressSpace,
        newUpdateAuthority,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 150_000,
            }),
            ix,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Compress a generic compressible account
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer and signer
 * @param programId             Program ID for the compressible program
 * @param pdaToCompress         PDA to compress
 * @param rentRecipient         Rent recipient public key
 * @param compressedAccountMeta Compressed account metadata
 * @param validityProof         Validity proof for compression
 * @param systemAccounts        Additional system accounts (trees, queues, etc.)
 * @param discriminator         Custom instruction discriminator (8 bytes)
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function compressAccount(
    rpc: Rpc,
    payer: Signer,
    programId: PublicKey,
    pdaToCompress: PublicKey,
    rentRecipient: PublicKey,
    compressedAccountMeta: CompressedAccountMeta,
    validityProof: ValidityProof,
    systemAccounts: AccountMeta[],
    discriminator: Uint8Array | number[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const ix = createCompressAccountInstruction(
        programId,
        discriminator,
        payer.publicKey,
        pdaToCompress,
        rentRecipient,
        compressedAccountMeta,
        validityProof,
        systemAccounts,
    );

    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 300_000,
            }),
            ix,
        ],
        payer,
        blockhash,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Decompress one or more compressed accounts idempotently
 *
 * @param rpc                       RPC connection to use
 * @param payer                     Fee payer
 * @param programId                 Program ID for the compressible program
 * @param feePayer                  Fee payer (can be same as payer)
 * @param rentPayer                 Rent payer
 * @param solanaAccounts            Array of PDA accounts to decompress
 * @param compressedAccountsData    Array of compressed account data
 * @param bumps                     Array of PDA bumps
 * @param validityProof             Validity proof for decompression
 * @param systemAccounts            Additional system accounts (trees, queues, etc.)
 * @param dataSchema                Borsh schema for account data serialization
 * @param discriminator             Optional custom discriminator (defaults to standard)
 * @param confirmOptions            Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function decompressAccountsIdempotent<T = any>(
    rpc: Rpc,
    payer: Signer,
    programId: PublicKey,
    feePayer: Signer,
    rentPayer: Signer,
    solanaAccounts: PublicKey[],
    compressedAccountsData: CompressedAccountData<T>[],
    bumps: number[],
    validityProof: ValidityProof,
    systemAccounts: AccountMeta[],
    dataSchema: any, // borsh.Layout<T>
    discriminator:
        | Uint8Array
        | number[] = COMPRESSIBLE_DISCRIMINATORS.DECOMPRESS_ACCOUNTS_IDEMPOTENT as unknown as number[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const ix = createDecompressAccountsIdempotentInstruction<T>(
        programId,
        discriminator,
        feePayer.publicKey,
        rentPayer.publicKey,
        solanaAccounts,
        compressedAccountsData,
        bumps,
        validityProof,
        systemAccounts,
        dataSchema,
    );

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [feePayer, rentPayer]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units: 400_000 + compressedAccountsData.length * 50_000,
            }),
            ix,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
