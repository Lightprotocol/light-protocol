import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import {
    PackedAccounts,
    SystemAccountMetaConfig,
} from '../../../src/utils/instruction';
import { getAccountCompressionAuthority } from '../../../src/constants';
import { LightSystemProgram } from '../../../src/programs/system/program';

describe('PackedAccounts runtime boolean normalization', () => {
    it('normalizes numeric AccountMeta flags from addPreAccountsMeta', () => {
        const packedAccounts = new PackedAccounts();
        const pubkey = PublicKey.unique();

        packedAccounts.addPreAccountsMeta({
            pubkey,
            isSigner: 0 as unknown as boolean,
            isWritable: 1 as unknown as boolean,
        });

        const [meta] = packedAccounts.toAccountMetas().remainingAccounts;
        expect(typeof meta.isSigner).toBe('boolean');
        expect(typeof meta.isWritable).toBe('boolean');
        expect(meta.isSigner).toBe(false);
        expect(meta.isWritable).toBe(true);
    });

    it('normalizes numeric flags from insertOrGetConfig', () => {
        const packedAccounts = new PackedAccounts();
        const pubkey = PublicKey.unique();

        packedAccounts.insertOrGetConfig(
            pubkey,
            1 as unknown as boolean,
            0 as unknown as boolean,
        );

        const [meta] = packedAccounts.toAccountMetas().remainingAccounts;
        expect(typeof meta.isSigner).toBe('boolean');
        expect(typeof meta.isWritable).toBe('boolean');
        expect(meta.isSigner).toBe(true);
        expect(meta.isWritable).toBe(false);
    });

    it('derives identical system accounts when PublicKey.findProgramAddressSync is unavailable (@solana/web3.js v3)', () => {
        const selfProgram = new PublicKey('11111111111111111111111111111111');
        const config = SystemAccountMetaConfig.new(selfProgram);

        const expectedPacked = new PackedAccounts();
        expectedPacked.addSystemAccounts(config);
        const expectedAuthority = getAccountCompressionAuthority();
        const expectedSolPda = LightSystemProgram.deriveCompressedSolPda();

        const origFind = PublicKey.findProgramAddressSync;
        const origCreate = PublicKey.createProgramAddressSync;

        try {
            (
                PublicKey as unknown as Record<string, unknown>
            ).findProgramAddressSync = undefined;
            (
                PublicKey as unknown as Record<string, unknown>
            ).createProgramAddressSync = undefined;

            const fallbackPacked = new PackedAccounts();
            fallbackPacked.addSystemAccounts(config);

            expect(fallbackPacked.toAccountMetas()).toEqual(
                expectedPacked.toAccountMetas(),
            );
            expect(getAccountCompressionAuthority().toBase58()).toBe(
                expectedAuthority.toBase58(),
            );
            expect(LightSystemProgram.deriveCompressedSolPda().toBase58()).toBe(
                expectedSolPda.toBase58(),
            );
        } finally {
            PublicKey.findProgramAddressSync = origFind;
            PublicKey.createProgramAddressSync = origCreate;
        }
    });
});
