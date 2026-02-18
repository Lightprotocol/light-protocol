import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    TOKEN_2022_PROGRAM_ID,
    ASSOCIATED_TOKEN_PROGRAM_ID,
    getAssociatedTokenAddressSync,
} from '@solana/spl-token';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getAssociatedTokenAddressInterface } from '../../src/v3/get-associated-token-address-interface';
import { getAtaProgramId } from '../../src/v3/ata-utils';

describe('getAssociatedTokenAddressInterface', () => {
    const mint = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;

    describe('default behavior (LIGHT_TOKEN_PROGRAM_ID)', () => {
        it('should derive ATA using LIGHT_TOKEN_PROGRAM_ID by default', () => {
            const result = getAssociatedTokenAddressInterface(mint, owner);

            const expected = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                LIGHT_TOKEN_PROGRAM_ID,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result.toBase58()).toBe(expected.toBase58());
        });

        it('should be deterministic - same inputs produce same output', () => {
            const result1 = getAssociatedTokenAddressInterface(mint, owner);
            const result2 = getAssociatedTokenAddressInterface(mint, owner);

            expect(result1.toBase58()).toBe(result2.toBase58());
        });

        it('should produce different addresses for different mints', () => {
            const mint2 = Keypair.generate().publicKey;

            const result1 = getAssociatedTokenAddressInterface(mint, owner);
            const result2 = getAssociatedTokenAddressInterface(mint2, owner);

            expect(result1.toBase58()).not.toBe(result2.toBase58());
        });

        it('should produce different addresses for different owners', () => {
            const owner2 = Keypair.generate().publicKey;

            const result1 = getAssociatedTokenAddressInterface(mint, owner);
            const result2 = getAssociatedTokenAddressInterface(mint, owner2);

            expect(result1.toBase58()).not.toBe(result2.toBase58());
        });
    });

    describe('explicit TOKEN_PROGRAM_ID', () => {
        it('should derive ATA using TOKEN_PROGRAM_ID', () => {
            const result = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            );

            const expected = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            expect(result.toBase58()).toBe(expected.toBase58());
        });

        it('should use ASSOCIATED_TOKEN_PROGRAM_ID for TOKEN_PROGRAM_ID', () => {
            const result = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            );

            // Verify getAtaProgramId returns ASSOCIATED_TOKEN_PROGRAM_ID for TOKEN_PROGRAM_ID
            expect(getAtaProgramId(TOKEN_PROGRAM_ID).toBase58()).toBe(
                ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
            );

            // Verify the derived address matches SPL's derivation
            const splResult = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );
            expect(result.toBase58()).toBe(splResult.toBase58());
        });
    });

    describe('explicit TOKEN_2022_PROGRAM_ID', () => {
        it('should derive ATA using TOKEN_2022_PROGRAM_ID', () => {
            const result = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_2022_PROGRAM_ID,
            );

            const expected = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                TOKEN_2022_PROGRAM_ID,
                ASSOCIATED_TOKEN_PROGRAM_ID,
            );

            expect(result.toBase58()).toBe(expected.toBase58());
        });

        it('should use ASSOCIATED_TOKEN_PROGRAM_ID for TOKEN_2022_PROGRAM_ID', () => {
            expect(getAtaProgramId(TOKEN_2022_PROGRAM_ID).toBase58()).toBe(
                ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
            );
        });
    });

    describe('different programIds produce different ATAs', () => {
        it('should produce different ATAs for CTOKEN vs TOKEN_PROGRAM_ID', () => {
            const ctokenAta = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                LIGHT_TOKEN_PROGRAM_ID,
            );
            const splAta = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            );

            expect(ctokenAta.toBase58()).not.toBe(splAta.toBase58());
        });

        it('should produce different ATAs for TOKEN_PROGRAM_ID vs TOKEN_2022_PROGRAM_ID', () => {
            const splAta = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            );
            const t22Ata = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_2022_PROGRAM_ID,
            );

            expect(splAta.toBase58()).not.toBe(t22Ata.toBase58());
        });
    });

    describe('allowOwnerOffCurve parameter', () => {
        it('should allow PDA owners when allowOwnerOffCurve is true', () => {
            // Create a PDA (off-curve point)
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-seed')],
                LIGHT_TOKEN_PROGRAM_ID,
            );

            // Should not throw with allowOwnerOffCurve = true
            const result = getAssociatedTokenAddressInterface(
                mint,
                pdaOwner,
                true,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(result).toBeInstanceOf(PublicKey);
        });

        it('should throw for PDA owners when allowOwnerOffCurve is false', () => {
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('test-seed')],
                LIGHT_TOKEN_PROGRAM_ID,
            );

            expect(() =>
                getAssociatedTokenAddressInterface(
                    mint,
                    pdaOwner,
                    false,
                    LIGHT_TOKEN_PROGRAM_ID,
                ),
            ).toThrow();
        });

        it('should default allowOwnerOffCurve to false', () => {
            const [pdaOwner] = PublicKey.findProgramAddressSync(
                [Buffer.from('another-seed')],
                TOKEN_PROGRAM_ID,
            );

            // Default behavior (no third param) should throw for PDA
            expect(() =>
                getAssociatedTokenAddressInterface(mint, pdaOwner),
            ).toThrow();
        });

        it('should work with regular (on-curve) owner regardless of allowOwnerOffCurve', () => {
            const regularOwner = Keypair.generate().publicKey;

            const result1 = getAssociatedTokenAddressInterface(
                mint,
                regularOwner,
                false,
            );
            const result2 = getAssociatedTokenAddressInterface(
                mint,
                regularOwner,
                true,
            );

            // Both should succeed and produce the same address
            expect(result1.toBase58()).toBe(result2.toBase58());
        });
    });

    describe('explicit associatedTokenProgramId', () => {
        it('should use explicit associatedTokenProgramId when provided', () => {
            const customAssocProgram = Keypair.generate().publicKey;

            const result = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                customAssocProgram,
            );

            const expected = getAssociatedTokenAddressSync(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                customAssocProgram,
            );

            expect(result.toBase58()).toBe(expected.toBase58());
        });

        it('should override auto-detected associatedTokenProgramId', () => {
            // Force LIGHT_TOKEN_PROGRAM_ID as associated program even for TOKEN_PROGRAM_ID
            const result = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
                LIGHT_TOKEN_PROGRAM_ID,
            );

            const autoDetected = getAssociatedTokenAddressInterface(
                mint,
                owner,
                false,
                TOKEN_PROGRAM_ID,
            );

            // Should be different because we overrode the associated program
            expect(result.toBase58()).not.toBe(autoDetected.toBase58());
        });
    });

    describe('getAtaProgramId helper', () => {
        it('should return LIGHT_TOKEN_PROGRAM_ID for LIGHT_TOKEN_PROGRAM_ID', () => {
            expect(getAtaProgramId(LIGHT_TOKEN_PROGRAM_ID).toBase58()).toBe(
                LIGHT_TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should return ASSOCIATED_TOKEN_PROGRAM_ID for TOKEN_PROGRAM_ID', () => {
            expect(getAtaProgramId(TOKEN_PROGRAM_ID).toBase58()).toBe(
                ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should return ASSOCIATED_TOKEN_PROGRAM_ID for TOKEN_2022_PROGRAM_ID', () => {
            expect(getAtaProgramId(TOKEN_2022_PROGRAM_ID).toBase58()).toBe(
                ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
            );
        });

        it('should return ASSOCIATED_TOKEN_PROGRAM_ID for unknown program IDs', () => {
            const unknownProgram = Keypair.generate().publicKey;
            expect(getAtaProgramId(unknownProgram).toBase58()).toBe(
                ASSOCIATED_TOKEN_PROGRAM_ID.toBase58(),
            );
        });
    });

    describe('edge cases', () => {
        it('should handle PublicKey.default as mint', () => {
            const result = getAssociatedTokenAddressInterface(
                PublicKey.default,
                owner,
            );
            expect(result).toBeInstanceOf(PublicKey);
        });

        it('should handle well-known program IDs as mint', () => {
            const result = getAssociatedTokenAddressInterface(
                TOKEN_PROGRAM_ID,
                owner,
            );
            expect(result).toBeInstanceOf(PublicKey);
        });

        it('should handle system program as mint', () => {
            const systemProgram = new PublicKey(
                '11111111111111111111111111111111',
            );
            const result = getAssociatedTokenAddressInterface(
                systemProgram,
                owner,
            );
            expect(result).toBeInstanceOf(PublicKey);
        });
    });
});
