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
    deriveCompressedAddress,
    deriveCompressedMintAddress,
    validatePositiveAmount,
    validateDecimals,
    validateAtaDerivation,
    isLightTokenAccount,
    determineTransferType,
    LIGHT_TOKEN_PROGRAM_ID,
    MINT_ADDRESS_TREE,
} from '../../src/index.js';

// ============================================================================
// TEST: PDA Derivation Functions
// ============================================================================

describe('deriveAssociatedTokenAddress', () => {
    it('derives correct ATA address', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await deriveAssociatedTokenAddress(owner, mint);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
    });

    it('produces consistent results for same inputs', async () => {
        const owner = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
        const mint = address('So11111111111111111111111111111111111111112');

        const result1 = await deriveAssociatedTokenAddress(owner, mint);
        const result2 = await deriveAssociatedTokenAddress(owner, mint);

        expect(result1.address).toBe(result2.address);
        expect(result1.bump).toBe(result2.bump);
    });

    it('produces different addresses for different owners', async () => {
        const owner1 = address('11111111111111111111111111111111');
        const owner2 = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
        const mint = address('So11111111111111111111111111111111111111112');

        const result1 = await deriveAssociatedTokenAddress(owner1, mint);
        const result2 = await deriveAssociatedTokenAddress(owner2, mint);

        expect(result1.address).not.toBe(result2.address);
    });

    it('produces different addresses for different mints', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint1 = address('So11111111111111111111111111111111111111112');
        const mint2 = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

        const result1 = await deriveAssociatedTokenAddress(owner, mint1);
        const result2 = await deriveAssociatedTokenAddress(owner, mint2);

        expect(result1.address).not.toBe(result2.address);
    });
});

describe('getAssociatedTokenAddressWithBump', () => {
    it('returns address when bump matches', async () => {
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

    it('throws when bump does not match', async () => {
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
    it('derives correct mint address', async () => {
        const mintSigner = address('11111111111111111111111111111111');

        const result = await deriveMintAddress(mintSigner);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
    });

    it('produces consistent results', async () => {
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
    it('derives correct pool address without index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await derivePoolAddress(mint);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
    });

    it('derives correct pool address with index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result = await derivePoolAddress(mint, 0);

        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
    });

    it('different indices produce different addresses', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const result0 = await derivePoolAddress(mint, 0);
        const result1 = await derivePoolAddress(mint, 1);

        expect(result0.address).not.toBe(result1.address);
    });

    it('no index equals index 0 (both omit index from seeds)', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const resultNoIndex = await derivePoolAddress(mint);
        const resultIndex0 = await derivePoolAddress(mint, 0);

        // Rust: index 0 means no index bytes in seeds, same as omitting index
        expect(resultNoIndex.address).toBe(resultIndex0.address);
    });

    it('restricted pool differs from regular pool', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const regular = await derivePoolAddress(mint, 0, false);
        const restricted = await derivePoolAddress(mint, 0, true);

        expect(regular.address).not.toBe(restricted.address);
    });

    it('restricted pool with index differs from without', async () => {
        const mint = address('So11111111111111111111111111111111111111112');

        const restricted0 = await derivePoolAddress(mint, 0, true);
        const restricted1 = await derivePoolAddress(mint, 1, true);

        expect(restricted0.address).not.toBe(restricted1.address);
    });

    it('throws for index > 4', async () => {
        const mint = address('So11111111111111111111111111111111111111112');
        await expect(derivePoolAddress(mint, 5)).rejects.toThrow(
            'Pool index must be an integer between 0 and 4',
        );
        await expect(derivePoolAddress(mint, 255)).rejects.toThrow(
            'Pool index must be an integer between 0 and 4',
        );
    });

    it('throws for negative index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');
        await expect(derivePoolAddress(mint, -1)).rejects.toThrow(
            'Pool index must be an integer between 0 and 4',
        );
    });

    it('throws for non-integer index', async () => {
        const mint = address('So11111111111111111111111111111111111111112');
        await expect(derivePoolAddress(mint, 1.5)).rejects.toThrow(
            'Pool index must be an integer between 0 and 4',
        );
    });
});

// ============================================================================
// TEST: Compressed Address Derivation
// ============================================================================

describe('deriveCompressedAddress', () => {
    it('produces a 32-byte result with high bit cleared', () => {
        const seed = new Uint8Array(32).fill(0x42);
        const tree = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
        const programId = LIGHT_TOKEN_PROGRAM_ID;

        const result = deriveCompressedAddress(seed, tree, programId);

        expect(result).toBeInstanceOf(Uint8Array);
        expect(result.length).toBe(32);
        // High bit must be cleared for BN254 field
        expect(result[0] & 0x80).toBe(0);
    });

    it('produces consistent results for same inputs', () => {
        const seed = new Uint8Array(32).fill(0x01);
        const tree = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
        const programId = LIGHT_TOKEN_PROGRAM_ID;

        const result1 = deriveCompressedAddress(seed, tree, programId);
        const result2 = deriveCompressedAddress(seed, tree, programId);

        expect(result1).toEqual(result2);
    });

    it('produces different results for different seeds', () => {
        const seed1 = new Uint8Array(32).fill(0x01);
        const seed2 = new Uint8Array(32).fill(0x02);
        const tree = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
        const programId = LIGHT_TOKEN_PROGRAM_ID;

        const result1 = deriveCompressedAddress(seed1, tree, programId);
        const result2 = deriveCompressedAddress(seed2, tree, programId);

        expect(result1).not.toEqual(result2);
    });
});

describe('deriveCompressedMintAddress', () => {
    it('produces a 32-byte result', () => {
        const mintSigner = address('11111111111111111111111111111111');
        const result = deriveCompressedMintAddress(mintSigner);

        expect(result).toBeInstanceOf(Uint8Array);
        expect(result.length).toBe(32);
        expect(result[0] & 0x80).toBe(0);
    });

    it('produces consistent results', () => {
        const mintSigner = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

        const result1 = deriveCompressedMintAddress(mintSigner);
        const result2 = deriveCompressedMintAddress(mintSigner);

        expect(result1).toEqual(result2);
    });

    it('uses MINT_ADDRESS_TREE as default', () => {
        const mintSigner = address('11111111111111111111111111111111');

        const withDefault = deriveCompressedMintAddress(mintSigner);
        const withExplicit = deriveCompressedMintAddress(mintSigner, MINT_ADDRESS_TREE);

        expect(withDefault).toEqual(withExplicit);
    });
});

// ============================================================================
// TEST: Validation Functions
// ============================================================================

describe('validatePositiveAmount', () => {
    it('passes for positive amount', () => {
        expect(() => validatePositiveAmount(1n)).not.toThrow();
        expect(() => validatePositiveAmount(100n)).not.toThrow();
        expect(() =>
            validatePositiveAmount(BigInt(Number.MAX_SAFE_INTEGER)),
        ).not.toThrow();
    });

    it('throws for zero', () => {
        expect(() => validatePositiveAmount(0n)).toThrow(
            'Amount must be positive',
        );
    });

    it('throws for negative', () => {
        expect(() => validatePositiveAmount(-1n)).toThrow(
            'Amount must be positive',
        );
        expect(() => validatePositiveAmount(-100n)).toThrow(
            'Amount must be positive',
        );
    });
});

describe('validateDecimals', () => {
    it('passes for valid decimals', () => {
        expect(() => validateDecimals(0)).not.toThrow();
        expect(() => validateDecimals(6)).not.toThrow();
        expect(() => validateDecimals(9)).not.toThrow();
        expect(() => validateDecimals(255)).not.toThrow();
    });

    it('throws for negative decimals', () => {
        expect(() => validateDecimals(-1)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });

    it('throws for decimals > 255', () => {
        expect(() => validateDecimals(256)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });

    it('throws for non-integer decimals', () => {
        expect(() => validateDecimals(1.5)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
        expect(() => validateDecimals(6.9)).toThrow(
            'Decimals must be an integer between 0 and 255',
        );
    });
});

describe('validateAtaDerivation', () => {
    it('validates correct ATA derivation', async () => {
        const owner = address('11111111111111111111111111111111');
        const mint = address('So11111111111111111111111111111111111111112');

        const { address: ata } = await deriveAssociatedTokenAddress(
            owner,
            mint,
        );

        const isValid = await validateAtaDerivation(ata, owner, mint);

        expect(isValid).toBe(true);
    });

    it('returns false for wrong ATA', async () => {
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
    it('correctly identifies Light token accounts', () => {
        expect(isLightTokenAccount(LIGHT_TOKEN_PROGRAM_ID)).toBe(true);
    });

    it('returns false for non-Light accounts', () => {
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

    it('returns light-to-light for both Light accounts', () => {
        expect(determineTransferType(lightProgram, lightProgram)).toBe(
            'light-to-light',
        );
    });

    it('returns light-to-spl for Light source, SPL dest', () => {
        expect(determineTransferType(lightProgram, splProgram)).toBe(
            'light-to-spl',
        );
    });

    it('returns spl-to-light for SPL source, Light dest', () => {
        expect(determineTransferType(splProgram, lightProgram)).toBe(
            'spl-to-light',
        );
    });

    it('returns spl-to-spl for both SPL accounts', () => {
        expect(determineTransferType(splProgram, splProgram)).toBe(
            'spl-to-spl',
        );
    });
});
