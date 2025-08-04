import {
    Connection,
    PublicKey,
    TransactionInstruction,
    SystemProgram,
    AccountInfo,
    Signer,
    ConfirmOptions,
} from '@solana/web3.js';
import * as borsh from '@coral-xyz/borsh';
import { Rpc } from '../rpc';
import { buildAndSignTx, sendAndConfirmTx } from './send-and-confirm';

/**
 * Derive the compression config PDA address
 */
export function deriveCompressionConfigAddress(
    programId: PublicKey,
    configIndex: number = 0,
): [PublicKey, number] {
    const [configAddress, configBump] = PublicKey.findProgramAddressSync(
        [Buffer.from('compressible_config'), Buffer.from([configIndex])],
        programId,
    );
    return [configAddress, configBump];
}

/**
 * Get the program data account address and its raw data for a given program.
 */
export async function getProgramDataAccount(
    programId: PublicKey,
    connection: Connection,
): Promise<{
    programDataAddress: PublicKey;
    programDataAccountInfo: AccountInfo<Buffer>;
}> {
    const programAccount = await connection.getAccountInfo(programId);
    if (!programAccount) {
        throw new Error('Program account does not exist');
    }
    const programDataAddress = new PublicKey(programAccount.data.slice(4, 36));
    const programDataAccountInfo =
        await connection.getAccountInfo(programDataAddress);
    if (!programDataAccountInfo) {
        throw new Error('Program data account does not exist');
    }
    return { programDataAddress, programDataAccountInfo };
}

/**
 * Check that the provided authority matches the program's upgrade authority.
 */
export function checkProgramUpdateAuthority(
    programDataAccountInfo: AccountInfo<Buffer>,
    providedAuthority: PublicKey,
): void {
    // Check discriminator (should be 3 for ProgramData)
    const discriminator = programDataAccountInfo.data.readUInt32LE(0);
    if (discriminator !== 3) {
        throw new Error('Invalid program data discriminator');
    }
    // Check if authority exists
    const hasAuthority = programDataAccountInfo.data[12] === 1;
    if (!hasAuthority) {
        throw new Error('Program has no upgrade authority');
    }
    // Extract upgrade authority (bytes 13-44)
    const authorityBytes = programDataAccountInfo.data.slice(13, 45);
    const upgradeAuthority = new PublicKey(authorityBytes);
    if (!upgradeAuthority.equals(providedAuthority)) {
        throw new Error(
            `Provided authority ${providedAuthority.toBase58()} does not match program's upgrade authority ${upgradeAuthority.toBase58()}`,
        );
    }
}

/**
 * Borsh schema for initializeCompressionConfig instruction data
 */
export const InitializeCompressionConfigSchema: borsh.Layout<CompressionConfigIxData> =
    borsh.struct([
        borsh.u32('compressionDelay'),
        borsh.publicKey('rentRecipient'),
        borsh.vec(borsh.publicKey(), 'addressSpace'),
        borsh.option(borsh.u8(), 'configBump'),
    ]);

export type CompressionConfigIxData = {
    compressionDelay: number;
    rentRecipient: PublicKey;
    addressSpace: PublicKey[];
    configBump: number | null;
};

/**
 * Serialize instruction data for initializeCompressionConfig using Borsh
 */
export function serializeInitializeCompressionConfigData(
    compressionDelay: number,
    rentRecipient: PublicKey,
    addressSpace: PublicKey[],
    configBump: number | null,
): Buffer {
    const discriminator = Buffer.from([133, 228, 12, 169, 56, 76, 222, 61]);

    const instructionData: CompressionConfigIxData = {
        compressionDelay,
        rentRecipient,
        addressSpace,
        configBump,
    };

    const buffer = Buffer.alloc(1000);
    const len = InitializeCompressionConfigSchema.encode(
        instructionData,
        buffer,
    );
    const dataBuffer = Buffer.from(new Uint8Array(buffer.slice(0, len)));

    return Buffer.concat([
        new Uint8Array(discriminator),
        new Uint8Array(dataBuffer),
    ]);
}

/**
 * Create initializeCompressionConfig instruction.
 */
export async function createInitializeCompressionConfigInstruction(
    programId: PublicKey,
    connection: Connection,
    payer: PublicKey,
    authority: PublicKey,
    compressionDelay: number,
    rentRecipient: PublicKey,
    addressSpace: PublicKey[],
    configIndex: number = 0,
): Promise<TransactionInstruction> {
    if (configIndex !== 0) {
        throw new Error('configIndex must be 0');
    }
    const [configAddress, _configBump] = deriveCompressionConfigAddress(
        programId,
        configIndex,
    );

    const { programDataAddress, programDataAccountInfo } =
        await getProgramDataAccount(programId, connection);
    checkProgramUpdateAuthority(programDataAccountInfo, authority);

    const data = serializeInitializeCompressionConfigData(
        compressionDelay,
        rentRecipient,
        addressSpace,
        0,
    );

    return new TransactionInstruction({
        keys: [
            { pubkey: payer, isSigner: true, isWritable: true },
            { pubkey: configAddress, isSigner: false, isWritable: true },
            { pubkey: programDataAddress, isSigner: false, isWritable: false },
            { pubkey: authority, isSigner: true, isWritable: false },
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            },
        ],
        programId,
        data,
    });
}

/**
 * Helper function to initialize compression config.
 */
export async function initializeCompressionConfig(
    programId: PublicKey,
    connection: Rpc,
    payer: Signer,
    programUpdateAuthority: Signer,
    compressionDelay: number,
    rentRecipient: PublicKey,
    addressSpace: PublicKey[],
    confirmOptions?: ConfirmOptions,
    configIndex: number = 0,
): Promise<string> {
    const ix = await createInitializeCompressionConfigInstruction(
        programId,
        connection,
        payer.publicKey,
        programUpdateAuthority.publicKey,
        compressionDelay,
        rentRecipient,
        addressSpace,
        configIndex,
    );

    const { blockhash } = await connection.getLatestBlockhash();

    // dedupe signers
    const additionalSigners = payer.publicKey.equals(
        programUpdateAuthority.publicKey,
    )
        ? []
        : [programUpdateAuthority];

    const tx = buildAndSignTx([ix], payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(connection as Rpc, tx, confirmOptions);

    return txId;
}
