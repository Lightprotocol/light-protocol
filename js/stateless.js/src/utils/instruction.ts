import { AccountMeta, PublicKey, SystemProgram } from '@solana/web3.js';
import { defaultStaticAccountsStruct } from '../constants';
import { LightSystemProgram } from '../programs';

export class PackedAccounts {
    private preAccounts: AccountMeta[] = [];
    private systemAccounts: AccountMeta[] = [];
    private nextIndex: number = 0;
    private map: Map<PublicKey, [number, AccountMeta]> = new Map();

    static newWithSystemAccounts(
        config: SystemAccountMetaConfig,
    ): PackedAccounts {
        const instance = new PackedAccounts();
        instance.addSystemAccounts(config);
        return instance;
    }

    static newWithSystemAccountsV2(
        config: SystemAccountMetaConfig,
    ): PackedAccounts {
        const instance = new PackedAccounts();
        instance.addSystemAccountsV2(config);
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

    addSystemAccountsV2(config: SystemAccountMetaConfig): void {
        this.systemAccounts.push(...getLightSystemAccountMetasV2(config));
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
        const entry = this.map.get(pubkey);
        if (entry) {
            return entry[0];
        }
        const index = this.nextIndex++;
        const meta: AccountMeta = { pubkey, isSigner, isWritable };
        this.map.set(pubkey, [index, meta]);
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

export function getLightSystemAccountMetas(
    config: SystemAccountMetaConfig,
): AccountMeta[] {
    let signerSeed = new TextEncoder().encode('cpi_authority');
    const cpiSigner = PublicKey.findProgramAddressSync(
        [signerSeed],
        config.selfProgram,
    )[0];
    const defaults = defaultStaticAccountsStruct();
    const metas: AccountMeta[] = [
        {
            pubkey: LightSystemProgram.programId,
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
        pubkey: SystemProgram.programId,
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

export function getLightSystemAccountMetasV2(
    config: SystemAccountMetaConfig,
): AccountMeta[] {
    let signerSeed = new TextEncoder().encode('cpi_authority');
    const cpiSigner = PublicKey.findProgramAddressSync(
        [signerSeed],
        config.selfProgram,
    )[0];
    const defaults = defaultStaticAccountsStruct();
    const metas: AccountMeta[] = [
        {
            pubkey: LightSystemProgram.programId,
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
            pubkey: SystemProgram.programId,
            isSigner: false,
            isWritable: false,
        },
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
    if (config.cpiContext) {
        metas.push({
            pubkey: config.cpiContext,
            isSigner: false,
            isWritable: true,
        });
    }
    return metas;
}
