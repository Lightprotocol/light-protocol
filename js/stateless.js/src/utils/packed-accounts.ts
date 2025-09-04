import { defaultStaticAccountsStruct } from '../constants';
import { LightSystemProgram } from '../programs/system';
import { AccountMeta, PublicKey, SystemProgram } from '@solana/web3.js';

/**
 * This file provides two variants of packed accounts for Light Protocol:
 *
 * 1. PackedAccounts - Matches CpiAccounts (11 system accounts)
 *    - Includes: LightSystemProgram, Authority, RegisteredProgramPda, NoopProgram,
 *      AccountCompressionAuthority, AccountCompressionProgram, InvokingProgram,
 *      [Optional: SolPoolPda, DecompressionRecipient], SystemProgram, [Optional: CpiContext]
 *
 * 2. PackedAccountsSmall - Matches CpiAccountsSmall (9 system accounts max)
 *    - Includes: LightSystemProgram, Authority, RegisteredProgramPda,
 *      AccountCompressionAuthority, AccountCompressionProgram, SystemProgram,
 *      [Optional: SolPoolPda, DecompressionRecipient, CpiContext]
 *    - Excludes: NoopProgram and InvokingProgram for a more compact structure
 */

/**
 * Create a PackedAccounts instance to pack the light protocol system accounts
 * for your custom program instruction. Typically, you will append them to the
 * end of your instruction's accounts / remainingAccounts.
 *
 * This matches the full CpiAccounts structure with 11 system accounts including
 * NoopProgram and InvokingProgram. For a more compact version, use PackedAccountsSmall.
 *
 * @example
 * ```ts
 * const packedAccounts = PackedAccounts.newWithSystemAccounts(config);
 *
 * const instruction = new TransactionInstruction({
 *     keys: [...yourInstructionAccounts, ...packedAccounts.toAccountMetas()],
 *     programId: selfProgram,
 *     data: data,
 * });
 * ```
 */
export class PackedAccounts {
    private preAccounts: AccountMeta[] = [];
    private systemAccounts: AccountMeta[] = [];
    private nextIndex: number = 0;
    private map: Map<string, [number, AccountMeta]> = new Map();

    static newWithSystemAccounts(
        config: SystemAccountMetaConfig,
    ): PackedAccounts {
        const instance = new PackedAccounts();
        instance.addSystemAccounts(config);
        return instance;
    }

    addPreAccountsSigner(pubkey: PublicKey): void {
        this.preAccounts.push({ pubkey, isSigner: true, isWritable: false });
    }

    addPreAccountsSignerMut(pubkey: PublicKey): void {
        this.preAccounts.push({ pubkey, isSigner: true, isWritable: true });
    }

    addPreAccountsMeta(accountMeta: AccountMeta): void {
        this.preAccounts.push(accountMeta);
    }

    addSystemAccounts(config: SystemAccountMetaConfig): void {
        this.systemAccounts.push(...getLightSystemAccountMetas(config));
    }

    insertOrGet(pubkey: PublicKey): number {
        return this.insertOrGetConfig(pubkey, false, true);
    }

    insertOrGetReadOnly(pubkey: PublicKey): number {
        return this.insertOrGetConfig(pubkey, false, false);
    }

    insertOrGetConfig(
        pubkey: PublicKey,
        isSigner: boolean,
        isWritable: boolean,
    ): number {
        const key = pubkey.toString();
        const entry = this.map.get(key);
        if (entry) {
            return entry[0];
        }
        const index = this.nextIndex++;
        const meta: AccountMeta = { pubkey, isSigner, isWritable };
        this.map.set(key, [index, meta]);
        return index;
    }

    private hashSetAccountsToMetas(): AccountMeta[] {
        const entries = Array.from(this.map.entries());
        entries.sort((a, b) => a[1][0] - b[1][0]);
        return entries.map(([, [, meta]]) => meta);
    }

    private getOffsets(): [number, number] {
        const systemStart = this.preAccounts.length;
        const packedStart = systemStart + this.systemAccounts.length;
        return [systemStart, packedStart];
    }

    toAccountMetas(): {
        remainingAccounts: AccountMeta[];
        systemStart: number;
        packedStart: number;
    } {
        const packed = this.hashSetAccountsToMetas();
        const [systemStart, packedStart] = this.getOffsets();
        return {
            remainingAccounts: [
                ...this.preAccounts,
                ...this.systemAccounts,
                ...packed,
            ],
            systemStart,
            packedStart,
        };
    }
}

/**
 * Creates a PackedAccounts instance with system accounts for the specified
 * program. This is a convenience wrapper around SystemAccountMetaConfig.new()
 * and PackedAccounts.newWithSystemAccounts().
 *
 * @param programId - The program ID that will be using these system accounts
 * @returns A new PackedAccounts instance with system accounts configured
 *
 * @example
 * ```ts
 * const packedAccounts = createPackedAccounts(myProgram.programId);
 *
 * const instruction = new TransactionInstruction({
 *     keys: [...yourInstructionAccounts, ...packedAccounts.toAccountMetas().remainingAccounts],
 *     programId: myProgram.programId,
 *     data: instructionData,
 * });
 * ```
 */
export function createPackedAccounts(programId: PublicKey): PackedAccounts {
    const systemAccountConfig = SystemAccountMetaConfig.new(programId);
    return PackedAccounts.newWithSystemAccounts(systemAccountConfig);
}

/**
 * Creates a PackedAccounts instance with system accounts and CPI context for the specified program.
 * This is a convenience wrapper that includes CPI context configuration.
 *
 * @param programId - The program ID that will be using these system accounts
 * @param cpiContext - The CPI context account public key
 * @returns A new PackedAccounts instance with system accounts and CPI context configured
 *
 * @example
 * ```ts
 * const packedAccounts = createPackedAccountsWithCpiContext(
 *     myProgram.programId,
 *     cpiContextAccount
 * );
 * ```
 */
export function createPackedAccountsWithCpiContext(
    programId: PublicKey,
    cpiContext: PublicKey,
): PackedAccounts {
    const systemAccountConfig = SystemAccountMetaConfig.newWithCpiContext(
        programId,
        cpiContext,
    );
    return PackedAccounts.newWithSystemAccounts(systemAccountConfig);
}

export class SystemAccountMetaConfig {
    selfProgram: PublicKey;
    cpiContext?: PublicKey;
    solCompressionRecipient?: PublicKey;
    solPoolPda?: PublicKey;

    private constructor(
        selfProgram: PublicKey,
        cpiContext?: PublicKey,
        solCompressionRecipient?: PublicKey,
        solPoolPda?: PublicKey,
    ) {
        this.selfProgram = selfProgram;
        this.cpiContext = cpiContext;
        this.solCompressionRecipient = solCompressionRecipient;
        this.solPoolPda = solPoolPda;
    }

    static new(selfProgram: PublicKey): SystemAccountMetaConfig {
        return new SystemAccountMetaConfig(selfProgram);
    }

    static newWithCpiContext(
        selfProgram: PublicKey,
        cpiContext: PublicKey,
    ): SystemAccountMetaConfig {
        return new SystemAccountMetaConfig(selfProgram, cpiContext);
    }
}

/**
 * Get the light protocol system accounts for your custom program instruction.
 * Use via `link PackedAccounts.addSystemAccounts(config)`.
 */
export function getLightSystemAccountMetas(
    config: SystemAccountMetaConfig,
): AccountMeta[] {
    const signerSeed = new TextEncoder().encode('cpi_authority');
    const cpiSigner = PublicKey.findProgramAddressSync(
        [signerSeed],
        config.selfProgram,
    )[0];
    const defaults = SystemAccountPubkeys.default();
    const metas: AccountMeta[] = [
        {
            pubkey: defaults.lightSystemProgram,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: cpiSigner, isSigner: false, isWritable: false },
        {
            pubkey: defaults.registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: defaults.noopProgram, isSigner: false, isWritable: false },
        {
            pubkey: defaults.accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: defaults.accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: config.selfProgram, isSigner: false, isWritable: false },
    ];
    if (config.solPoolPda) {
        metas.push({
            pubkey: config.solPoolPda,
            isSigner: false,
            isWritable: true,
        });
    }
    if (config.solCompressionRecipient) {
        metas.push({
            pubkey: config.solCompressionRecipient,
            isSigner: false,
            isWritable: true,
        });
    }
    metas.push({
        pubkey: defaults.systemProgram,
        isSigner: false,
        isWritable: false,
    });
    if (config.cpiContext) {
        metas.push({
            pubkey: config.cpiContext,
            isSigner: false,
            isWritable: true,
        });
    }
    return metas;
}

/**
 * PackedAccountsSmall matches the CpiAccountsSmall structure with simplified account ordering.
 * This is a more compact version that excludes NoopProgram and InvokingProgram.
 */
export class PackedAccountsSmall {
    private preAccounts: AccountMeta[] = [];
    private systemAccounts: AccountMeta[] = [];
    private nextIndex: number = 0;
    private map: Map<string, [number, AccountMeta]> = new Map();

    static newWithSystemAccounts(
        config: SystemAccountMetaConfig,
    ): PackedAccountsSmall {
        const instance = new PackedAccountsSmall();
        instance.addSystemAccounts(config);
        return instance;
    }

    /**
     * Returns the internal map of pubkey to [index, AccountMeta].
     * For debugging purposes only.
     */
    getNamedMetas(): Map<string, [number, AccountMeta]> {
        return this.map;
    }

    addPreAccountsSigner(pubkey: PublicKey): void {
        this.preAccounts.push({ pubkey, isSigner: true, isWritable: false });
    }

    addPreAccountsSignerMut(pubkey: PublicKey): void {
        this.preAccounts.push({ pubkey, isSigner: true, isWritable: true });
    }

    addPreAccountsMeta(accountMeta: AccountMeta): void {
        this.preAccounts.push(accountMeta);
    }

    addSystemAccounts(config: SystemAccountMetaConfig): void {
        this.systemAccounts.push(...getLightSystemAccountMetasSmall(config));
    }

    insertOrGet(pubkey: PublicKey): number {
        return this.insertOrGetConfig(pubkey, false, true);
    }

    insertOrGetReadOnly(pubkey: PublicKey): number {
        return this.insertOrGetConfig(pubkey, false, false);
    }

    insertOrGetConfig(
        pubkey: PublicKey,
        isSigner: boolean,
        isWritable: boolean,
    ): number {
        const key = pubkey.toString();
        const entry = this.map.get(key);
        if (entry) {
            return entry[0];
        }
        const index = this.nextIndex++;
        const meta: AccountMeta = { pubkey, isSigner, isWritable };
        this.map.set(key, [index, meta]);
        return index;
    }

    private hashSetAccountsToMetas(): AccountMeta[] {
        const entries = Array.from(this.map.entries());
        entries.sort((a, b) => a[1][0] - b[1][0]);
        return entries.map(([, [, meta]]) => meta);
    }

    private getOffsets(): [number, number] {
        const systemStart = this.preAccounts.length;
        const packedStart = systemStart + this.systemAccounts.length;
        return [systemStart, packedStart];
    }

    toAccountMetas(): {
        remainingAccounts: AccountMeta[];
        systemStart: number;
        packedStart: number;
    } {
        const packed = this.hashSetAccountsToMetas();
        const [systemStart, packedStart] = this.getOffsets();
        return {
            remainingAccounts: [
                ...this.preAccounts,
                ...this.systemAccounts,
                ...packed,
            ],
            systemStart,
            packedStart,
        };
    }
}

/**
 * Get the light protocol system accounts for the small variant.
 * This matches CpiAccountsSmall ordering: removes NoopProgram and InvokingProgram.
 */
export function getLightSystemAccountMetasSmall(
    config: SystemAccountMetaConfig,
): AccountMeta[] {
    const signerSeed = new TextEncoder().encode('cpi_authority');
    const cpiSigner = PublicKey.findProgramAddressSync(
        [signerSeed],
        config.selfProgram,
    )[0];
    const defaults = SystemAccountPubkeys.default();

    // Small variant ordering: LightSystemProgram, Authority, RegisteredProgramPda,
    // AccountCompressionAuthority, AccountCompressionProgram, SystemProgram,
    // [Optional: SolPoolPda, DecompressionRecipient, CpiContext]
    const metas: AccountMeta[] = [
        {
            pubkey: defaults.lightSystemProgram,
            isSigner: false,
            isWritable: false,
        },
        { pubkey: cpiSigner, isSigner: false, isWritable: false },
        {
            pubkey: defaults.registeredProgramPda,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: defaults.accountCompressionAuthority,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: defaults.accountCompressionProgram,
            isSigner: false,
            isWritable: false,
        },
        {
            pubkey: defaults.systemProgram,
            isSigner: false,
            isWritable: false,
        },
    ];

    // Optional accounts in order
    if (config.solPoolPda) {
        metas.push({
            pubkey: config.solPoolPda,
            isSigner: false,
            isWritable: true,
        });
    }
    if (config.solCompressionRecipient) {
        metas.push({
            pubkey: config.solCompressionRecipient,
            isSigner: false,
            isWritable: true,
        });
    }
    if (config.cpiContext) {
        metas.push({
            pubkey: config.cpiContext,
            isSigner: false,
            isWritable: true,
        });
    }
    return metas;
}

/**
 * Creates a PackedAccountsSmall instance with system accounts for the specified program.
 * This uses the simplified account ordering that matches CpiAccountsSmall.
 */
export function createPackedAccountsSmall(
    programId: PublicKey,
): PackedAccountsSmall {
    const systemAccountConfig = SystemAccountMetaConfig.new(programId);
    return PackedAccountsSmall.newWithSystemAccounts(systemAccountConfig);
}

/**
 * Creates a PackedAccountsSmall instance with system accounts and CPI context.
 */
export function createPackedAccountsSmallWithCpiContext(
    programId: PublicKey,
    cpiContext: PublicKey,
): PackedAccountsSmall {
    const systemAccountConfig = SystemAccountMetaConfig.newWithCpiContext(
        programId,
        cpiContext,
    );
    return PackedAccountsSmall.newWithSystemAccounts(systemAccountConfig);
}

export class SystemAccountPubkeys {
    lightSystemProgram: PublicKey;
    systemProgram: PublicKey;
    accountCompressionProgram: PublicKey;
    accountCompressionAuthority: PublicKey;
    registeredProgramPda: PublicKey;
    noopProgram: PublicKey;
    solPoolPda: PublicKey;

    private constructor(
        lightSystemProgram: PublicKey,
        systemProgram: PublicKey,
        accountCompressionProgram: PublicKey,
        accountCompressionAuthority: PublicKey,
        registeredProgramPda: PublicKey,
        noopProgram: PublicKey,
        solPoolPda: PublicKey,
    ) {
        this.lightSystemProgram = lightSystemProgram;
        this.systemProgram = systemProgram;
        this.accountCompressionProgram = accountCompressionProgram;
        this.accountCompressionAuthority = accountCompressionAuthority;
        this.registeredProgramPda = registeredProgramPda;
        this.noopProgram = noopProgram;
        this.solPoolPda = solPoolPda;
    }

    static default(): SystemAccountPubkeys {
        return new SystemAccountPubkeys(
            LightSystemProgram.programId,
            SystemProgram.programId,
            defaultStaticAccountsStruct().accountCompressionProgram,
            defaultStaticAccountsStruct().accountCompressionAuthority,
            defaultStaticAccountsStruct().registeredProgramPda,
            defaultStaticAccountsStruct().noopProgram,
            PublicKey.default,
        );
    }
}
