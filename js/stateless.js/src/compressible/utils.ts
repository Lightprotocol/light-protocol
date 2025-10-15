import { Connection, PublicKey, AccountInfo } from '@solana/web3.js';
import { bn } from '..';

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

export function deriveTokenProgramConfig(
    version?: number,
): [PublicKey, number] {
    const versionValue = version ?? 1;
    const registryProgramId = new PublicKey(
        'Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX',
    );

    const [compressibleConfig, configBump] = PublicKey.findProgramAddressSync(
        [
            Buffer.from('compressible_config'),
            bn(versionValue).toArrayLike(Buffer, 'le', 2),
        ],
        registryProgramId,
    );

    const expected = new PublicKey(
        'ACXg8a7VaqecBWrSbdu73W4Pg9gsqXJ3EXAqkHyhvVXg',
    );
    if (!compressibleConfig.equals(expected)) {
        console.log('compressibleConfig:', compressibleConfig);
        console.log('expected:', expected);
        throw new Error('compressibleConfig is not correct');
    }
    return [compressibleConfig, configBump];
}
