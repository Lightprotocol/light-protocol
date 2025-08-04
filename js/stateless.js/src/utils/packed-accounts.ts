import { defaultStaticAccountsStruct } from '../constants';
import { LightSystemProgram } from '../programs/system';
import { AccountMeta, PublicKey, SystemProgram } from '@solana/web3.js';

/**
 * Create a PackedAccounts instance to pack the light protocol system accounts
 * for your custom program instruction. Typically, you will append them to the
 * end of your instruction's accounts / remainingAccounts.
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
    let signerSeed = new TextEncoder().encode('cpi_authority');
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
