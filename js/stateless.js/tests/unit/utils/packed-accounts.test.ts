import { describe, expect, it } from 'vitest';
import { PublicKey } from '@solana/web3.js';
import { PackedAccounts } from '../../../src/utils/instruction';

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
});
