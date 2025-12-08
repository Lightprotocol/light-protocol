import { describe, it, expect } from 'vitest';
import { Keypair } from '@solana/web3.js';
import { TOKEN_PROGRAM_ID, TOKEN_2022_PROGRAM_ID } from '@solana/spl-token';
import { bn } from '@lightprotocol/stateless.js';
import {
    // New names
    deriveSplInterfaceInfo,
    SplInterfaceInfo,
    // Deprecated aliases - should still work
    deriveTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { CompressedTokenProgram } from '../../src/program';

describe('deriveSplInterfaceInfo', () => {
    const mint = Keypair.generate().publicKey;

    it('should derive SplInterfaceInfo for TOKEN_PROGRAM_ID', () => {
        const result = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);

        expect(result.mint.toBase58()).toBe(mint.toBase58());
        expect(result.tokenProgram.toBase58()).toBe(
            TOKEN_PROGRAM_ID.toBase58(),
        );
        expect(result.isInitialized).toBe(true);
        expect(result.balance.eq(bn(0))).toBe(true);
        expect(result.poolIndex).toBe(0);
        expect(typeof result.bump).toBe('number');
        expect(result.splInterfacePda).toBeDefined();
    });

    it('should derive SplInterfaceInfo for TOKEN_2022_PROGRAM_ID', () => {
        const result = deriveSplInterfaceInfo(mint, TOKEN_2022_PROGRAM_ID);

        expect(result.mint.toBase58()).toBe(mint.toBase58());
        expect(result.tokenProgram.toBase58()).toBe(
            TOKEN_2022_PROGRAM_ID.toBase58(),
        );
        expect(result.isInitialized).toBe(true);
    });

    it('should derive correct PDA matching CompressedTokenProgram', () => {
        const result = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);
        const [expectedPda, expectedBump] =
            CompressedTokenProgram.deriveSplInterfacePdaWithIndex(mint, 0);

        expect(result.splInterfacePda.toBase58()).toBe(expectedPda.toBase58());
        expect(result.bump).toBe(expectedBump);
    });

    it('should support different pool indices', () => {
        const result0 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID, 0);
        const result1 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID, 1);
        const result2 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID, 2);

        expect(result0.poolIndex).toBe(0);
        expect(result1.poolIndex).toBe(1);
        expect(result2.poolIndex).toBe(2);

        // Different indices should produce different PDAs
        expect(result0.splInterfacePda.toBase58()).not.toBe(
            result1.splInterfacePda.toBase58(),
        );
        expect(result1.splInterfacePda.toBase58()).not.toBe(
            result2.splInterfacePda.toBase58(),
        );
    });

    it('should match PDA for non-zero pool indices', () => {
        for (let i = 0; i < 4; i++) {
            const result = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID, i);
            const [expectedPda, expectedBump] =
                CompressedTokenProgram.deriveSplInterfacePdaWithIndex(mint, i);

            expect(result.splInterfacePda.toBase58()).toBe(
                expectedPda.toBase58(),
            );
            expect(result.bump).toBe(expectedBump);
            expect(result.poolIndex).toBe(i);
        }
    });

    it('should be deterministic', () => {
        const result1 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);
        const result2 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);

        expect(result1.splInterfacePda.toBase58()).toBe(
            result2.splInterfacePda.toBase58(),
        );
        expect(result1.bump).toBe(result2.bump);
    });

    it('should produce different PDAs for different mints', () => {
        const mint2 = Keypair.generate().publicKey;

        const result1 = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);
        const result2 = deriveSplInterfaceInfo(mint2, TOKEN_PROGRAM_ID);

        expect(result1.splInterfacePda.toBase58()).not.toBe(
            result2.splInterfacePda.toBase58(),
        );
    });

    it('should have activity undefined', () => {
        const result = deriveSplInterfaceInfo(mint, TOKEN_PROGRAM_ID);
        expect(result.activity).toBeUndefined();
    });
});

describe('deprecated aliases', () => {
    const mint = Keypair.generate().publicKey;

    it('deriveTokenPoolInfo should work as alias for deriveSplInterfaceInfo', () => {
        // Test that old function name still works
        const result: TokenPoolInfo = deriveTokenPoolInfo(
            mint,
            TOKEN_PROGRAM_ID,
        );

        expect(result.mint.toBase58()).toBe(mint.toBase58());
        expect(result.isInitialized).toBe(true);
        expect(result.balance.eq(bn(0))).toBe(true);
        // Both tokenPoolPda (deprecated) and splInterfacePda should be accessible
        expect(result.tokenPoolPda).toBeDefined();
        expect(result.splInterfacePda).toBeDefined();
        // Both should point to the same value
        expect(result.tokenPoolPda.toBase58()).toBe(
            result.splInterfacePda.toBase58(),
        );
    });

    it('TokenPoolInfo type should be compatible with SplInterfaceInfo', () => {
        // Both types should work for the same result
        const newResult: SplInterfaceInfo = deriveSplInterfaceInfo(
            mint,
            TOKEN_PROGRAM_ID,
        );
        const oldResult: TokenPoolInfo = deriveTokenPoolInfo(
            mint,
            TOKEN_PROGRAM_ID,
        );

        // Both should have same data
        expect(newResult.mint.toBase58()).toBe(oldResult.mint.toBase58());
        expect(newResult.splInterfacePda.toBase58()).toBe(
            oldResult.splInterfacePda.toBase58(),
        );
        // TokenPoolInfo should have tokenPoolPda for backward compatibility
        expect(oldResult.tokenPoolPda.toBase58()).toBe(
            oldResult.splInterfacePda.toBase58(),
        );
    });

    it('deprecated deriveTokenPoolPdaWithIndex should work', () => {
        // Test the deprecated static method
        const [pda, bump] = CompressedTokenProgram.deriveTokenPoolPdaWithIndex(
            mint,
            0,
        );
        const [newPda, newBump] =
            CompressedTokenProgram.deriveSplInterfacePdaWithIndex(mint, 0);

        expect(pda.toBase58()).toBe(newPda.toBase58());
        expect(bump).toBe(newBump);
    });

    it('deprecated deriveTokenPoolPda should work', () => {
        const pda = CompressedTokenProgram.deriveTokenPoolPda(mint);
        const newPda = CompressedTokenProgram.deriveSplInterfacePda(mint);

        expect(pda.toBase58()).toBe(newPda.toBase58());
    });
});
