/**
 * Unit tests for Light Token SDK Utils
 *
 * Tests for:
 * - PDA derivation functions
 * - Validation functions
 */

import { describe, it, expect } from 'vitest';
import { address } from '@solana/addresses';

import {
    deriveAssociatedTokenAddress,
    getAssociatedTokenAddressWithBump,
    deriveMintAddress,
    derivePoolAddress,
    validatePositiveAmount,
    validateDecimals,
    validateAtaDerivation,
    isLightTokenAccount,
    determineTransferType,
    LIGHT_TOKEN_PROGRAM_ID,
} from '../../src/index.js';

// ============================================================================
// TEST: PDA Derivation Functions
// ============================================================================

describe('deriveAssociatedTokenAddress', () => {
    it('6.1 derives correct ATA address', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await deriveAssociatedTokenAddress(owner, mint);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
    });

    it('6.1.1 produces consistent results for same inputs', async () => {
        const owner = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
        const mint = address('So11111111111111111111111111111111111111112');

        const result1 = await deriveAssociatedTokenAddress(owner, mint);
        const result2 = await deriveAssociatedTokenAddress(owner, mint);

        expect(result1.address).toBe(result2.address);
        expect(result1.bump).toBe(result2.bump);
    });

    it('6.1.2 produces different addresses for different owners', async () => {
        const owner1 = address('11111111111111111111111111111111');
        const owner2 = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
        const mint = address('So11111111111111111111111111111111111111112');

        const result1 = await deriveAssociatedTokenAddress(owner1, mint);
        const result2 = await deriveAssociatedTokenAddress(owner2, mint);

        expect(result1.address).not.toBe(result2.address);
    });

    it('6.1.3 produces different addresses for different mints', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint1 = address('So11111111111111111111111111111111111111112');
        const mint2 = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

        const result1 = await deriveAssociatedTokenAddress(owner, mint1);
        const result2 = await deriveAssociatedTokenAddress(owner, mint2);

        expect(result1.address).not.toBe(result2.address);
    });
});

describe('getAssociatedTokenAddressWithBump', () => {
    it('6.2 returns address when bump matches', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        // First derive to get the correct bump
        const { address: expectedAddress, bump } =
            await deriveAssociatedTokenAddress(owner, mint);

        // Then verify with bump
        const result = await getAssociatedTokenAddressWithBump(
            owner,
            mint,
            bump,
        );

        expect(result).toBe(expectedAddress);
    });

    it('6.2.1 throws when bump does not match', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        // Get the correct bump first
        const { bump: correctBump } = await deriveAssociatedTokenAddress(
            owner,
            mint,
        );

        // Use wrong bump
        const wrongBump = (correctBump + 1) % 256;

        await expect(
            getAssociatedTokenAddressWithBump(owner, mint, wrongBump),
        ).rejects.toThrow('Bump mismatch');
    });
});

describe('deriveMintAddress', () => {
    it('6.3 derives correct mint address', async () => {
        const mintSigner = address('11111111111111111111111111111111');

        const result = await deriveMintAddress(mintSigner);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
    });

    it('6.3.1 produces consistent results', async () => {
        const mintSigner = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );

        const result1 = await deriveMintAddress(mintSigner);
        const result2 = await deriveMintAddress(mintSigner);

        expect(result1.address).toBe(result2.address);
        expect(result1.bump).toBe(result2.bump);
    });
});

describe('derivePoolAddress', () => {
    it('6.4 derives correct pool address without index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await derivePoolAddress(mint);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
    });

    it('6.4.1 derives correct pool address with index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await derivePoolAddress(mint, 0);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
    });

    it('6.4.2 different indices produce different addresses', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result0 = await derivePoolAddress(mint, 0);
        const result1 = await derivePoolAddress(mint, 1);

        expect(result0.address).not.toBe(result1.address);
    });

    it('6.4.3 no index equals index 0 (both omit index from seeds)', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const resultNoIndex = await derivePoolAddress(mint);
        const resultIndex0 = await derivePoolAddress(mint, 0);

        // Rust: index 0 means no index bytes in seeds, same as omitting index
        expect(resultNoIndex.address).toBe(resultIndex0.address);
    });

    it('6.4.4 restricted pool differs from regular pool', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const regular = await derivePoolAddress(mint, 0, false);
        const restricted = await derivePoolAddress(mint, 0, true);

        expect(regular.address).not.toBe(restricted.address);
    });

    it('6.4.5 restricted pool with index differs from without', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const restricted0 = await derivePoolAddress(mint, 0, true);
        const restricted1 = await derivePoolAddress(mint, 1, true);

        expect(restricted0.address).not.toBe(restricted1.address);
    });
});

// ============================================================================
// TEST: Validation Functions
// ============================================================================

describe('validatePositiveAmount', () => {
    it('7.1 passes for positive amount', () => {
        expect(() => validatePositiveAmount(1n)).not.toThrow();
        expect(() => validatePositiveAmount(100n)).not.toThrow();
        expect(() =>
            validatePositiveAmount(BigInt(Number.MAX_SAFE_INTEGER)),
        ).not.toThrow();
    });

    it('7.1.1 throws for zero', () => {
        expect(() => validatePositiveAmount(0n)).toThrow(
            'Amount must be positive',
        );
    });

    it('7.1.2 throws for negative', () => {
        expect(() => validatePositiveAmount(-1n)).toThrow(
            'Amount must be positive',
        );
        expect(() => validatePositiveAmount(-100n)).toThrow(
            'Amount must be positive',
        );
    });
});

describe('validateDecimals', () => {
    it('7.2 passes for valid decimals', () => {
        expect(() => validateDecimals(0)).not.toThrow();
        expect(() => validateDecimals(6)).not.toThrow();
        expect(() => validateDecimals(9)).not.toThrow();
        expect(() => validateDecimals(255)).not.toThrow();
    });

    it('7.2.1 throws for negative decimals', () => {
        expect(() => validateDecimals(-1)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });

    it('7.2.2 throws for decimals > 255', () => {
        expect(() => validateDecimals(256)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });

    it('7.2.3 throws for non-integer decimals', () => {
        expect(() => validateDecimals(1.5)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
        expect(() => validateDecimals(6.9)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });
});

describe('validateAtaDerivation', () => {
    it('7.3 validates correct ATA derivation', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        const { address: ata } = await deriveAssociatedTokenAddress(
            owner,
            mint,
        );

        const isValid = await validateAtaDerivation(ata, owner, mint);

        expect(isValid).toBe(true);
    });

    it('7.3.1 returns false for wrong ATA', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');
        const wrongAta = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );

        const isValid = await validateAtaDerivation(wrongAta, owner, mint);

        expect(isValid).toBe(false);
    });
});

describe('isLightTokenAccount', () => {
    it('7.4 correctly identifies Light token accounts', () => {
        expect(isLightTokenAccount(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('7.4.1 returns false for non-Light accounts', () => {
        const splToken = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );
        const systemProgram = address('11111111111111111111111111111111');

        expect(isLightTokenAccount(splToken)).toBe(false);
        expect(isLightTokenAccount(systemProgram)).toBe(false);
    });
});

describe('determineTransferType', () => {
    const lightProgram = LIGHT_TOKEN_PROGRAM_ID;
    const splProgram = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

    it('7.5 returns light-to-light for both Light accounts', () => {
        expect(determineTransferType(lightProgram, lightProgram)).toBe(
            'light-to-light',
        );
    });

    it('7.5.1 returns light-to-spl for Light source, SPL dest', () => {
        expect(determineTransferType(lightProgram, splProgram)).toBe(
            'light-to-spl',
        );
    });

    it('7.5.2 returns spl-to-light for SPL source, Light dest', () => {
        expect(determineTransferType(splProgram, lightProgram)).toBe(
            'spl-to-light',
        );
    });

    it('7.5.3 returns spl-to-spl for both SPL accounts', () => {
        expect(determineTransferType(splProgram, splProgram)).toBe(
            'spl-to-spl',
        );
    });
});
