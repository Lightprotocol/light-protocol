/**
 * Comprehensive unit tests for Light Token SDK instruction builders.
 *
 * Tests for every instruction builder exported from the SDK, verifying:
 * - Correct program address
 * - Correct number of accounts
 * - Correct account addresses in correct order
 * - Correct account roles (AccountRole enum)
 * - Correct discriminator byte (first byte of data)
 * - Correct data encoding via codec round-trip
 * - Optional fields (maxTopUp, feePayer, etc.)
 * - Validation (zero amount, invalid decimals, etc.)
 */

import { describe, it, expect } from 'vitest';
import { address } from '@solana/addresses';
import { AccountRole } from '@solana/instructions';

import {
    // Instruction builders
    createTransferInstruction,
    createTransferCheckedInstruction,
    createTransferInterfaceInstruction,
    createCloseAccountInstruction,
    createMintToInstruction,
    createMintToCheckedInstruction,
    createBurnInstruction,
    createBurnCheckedInstruction,
    createFreezeInstruction,
    createThawInstruction,
    createApproveInstruction,
    createRevokeInstruction,
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,

    // Constants
    LIGHT_TOKEN_PROGRAM_ID,
    DISCRIMINATOR,
    SYSTEM_PROGRAM_ID,

    // Codecs
    getAmountInstructionCodec,
    getCheckedInstructionCodec,
    getDiscriminatorOnlyCodec,
    decodeMaxTopUp,
} from '../../src/index.js';

// ============================================================================
// TEST ADDRESSES
// ============================================================================

const TEST_PAYER = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');
const TEST_OWNER = address('11111111111111111111111111111111');
const TEST_MINT = address('So11111111111111111111111111111111111111112');
const TEST_SOURCE = address('amt2kaJA14v3urZbZvnc5v2np8jqvc4Z8zDep5wbtzx');
const TEST_DEST = address('GXtd2izAiMJPwMEjfgTRH3d7k9mjn4Jq3JrWFv9gySYy');
const TEST_DELEGATE = address('SySTEM1eSU2p4BGQfQpimFEWWSC1XDFeun3Nqzz3rT7');
const TEST_AUTHORITY = address('compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq');
const TEST_FREEZE_AUTH = address('cTokenmWW8bLPjZEBAUgYy3zKxQZW6VKi7bqNFEVv3m');
const TEST_CONFIG = address('TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb');
const TEST_SPONSOR = address('BPFLoaderUpgradeab1e11111111111111111111111');

// ============================================================================
// TEST: createTransferInstruction
// ============================================================================

describe('createTransferInstruction', () => {
    it('has correct program address', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (4 without feePayer)', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        expect(ix.accounts).toHaveLength(4);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_DEST);
        expect(ix.accounts[2].address).toBe(TEST_AUTHORITY);
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
    });

    it('has correct account roles', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
    });

    it('has correct discriminator byte', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER);
        expect(ix.data[0]).toBe(3);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.TRANSFER);
        expect(decoded.amount).toBe(1000n);
    });

    it('has 9-byte data without maxTopUp', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });
        // 1 (disc) + 8 (amount) = 9 bytes
        expect(ix.data.length).toBe(9);
    });

    it('with maxTopUp has 11-byte data and authority becomes WRITABLE_SIGNER', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            maxTopUp: 5000,
        });
        // 1 (disc) + 8 (amount) + 2 (maxTopUp u16) = 11 bytes
        expect(ix.data.length).toBe(11);
        // authority becomes WRITABLE_SIGNER when maxTopUp is set and no feePayer
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);

        // Verify maxTopUp decoding
        const maxTopUp = decodeMaxTopUp(ix.data, 9);
        expect(maxTopUp).toBe(5000);
    });

    it('with feePayer has 5 accounts and authority stays READONLY_SIGNER', () => {
        const feePayer = address('Vote111111111111111111111111111111111111111');
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            maxTopUp: 5000,
            feePayer,
        });
        expect(ix.accounts).toHaveLength(5);
        // authority stays READONLY_SIGNER when feePayer is provided
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
        // feePayer is WRITABLE_SIGNER
        expect(ix.accounts[4].address).toBe(feePayer);
        expect(ix.accounts[4].role).toBe(AccountRole.WRITABLE_SIGNER);
    });

    it('with feePayer but no maxTopUp still adds feePayer account', () => {
        const feePayer = address('Vote111111111111111111111111111111111111111');
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            feePayer,
        });
        expect(ix.accounts).toHaveLength(5);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[4].address).toBe(feePayer);
        expect(ix.accounts[4].role).toBe(AccountRole.WRITABLE_SIGNER);
    });

    it('validation: zero amount throws "Amount must be positive"', () => {
        expect(() =>
            createTransferInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                amount: 0n,
                authority: TEST_AUTHORITY,
            }),
        ).toThrow('Amount must be positive');
    });

    it('validation: negative amount throws "Amount must be positive"', () => {
        expect(() =>
            createTransferInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                amount: -1n,
                authority: TEST_AUTHORITY,
            }),
        ).toThrow('Amount must be positive');
    });

    it('encodes large amounts correctly', () => {
        const largeAmount = 18_446_744_073_709_551_615n; // u64::MAX
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: largeAmount,
            authority: TEST_AUTHORITY,
        });
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.amount).toBe(largeAmount);
    });
});

// ============================================================================
// TEST: createTransferCheckedInstruction
// ============================================================================

describe('createTransferCheckedInstruction', () => {
    it('has correct program address', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (5 without feePayer)', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        expect(ix.accounts).toHaveLength(5);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[2].address).toBe(TEST_DEST);
        expect(ix.accounts[3].address).toBe(TEST_AUTHORITY);
        expect(ix.accounts[4].address).toBe(SYSTEM_PROGRAM_ID);
    });

    it('has correct account roles', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[4].role).toBe(AccountRole.READONLY);
    });

    it('has correct discriminator byte', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER_CHECKED);
        expect(ix.data[0]).toBe(12);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });
        const codec = getCheckedInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.TRANSFER_CHECKED);
        expect(decoded.amount).toBe(1000n);
        expect(decoded.decimals).toBe(9);
    });

    it('with maxTopUp: authority becomes WRITABLE_SIGNER', () => {
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
            maxTopUp: 3000,
        });
        expect(ix.accounts[3].role).toBe(AccountRole.WRITABLE_SIGNER);

        // Verify maxTopUp in data: disc(1) + amount(8) + decimals(1) = offset 10
        const maxTopUp = decodeMaxTopUp(ix.data, 10);
        expect(maxTopUp).toBe(3000);
    });

    it('with feePayer: 6 accounts, authority stays READONLY_SIGNER', () => {
        const feePayer = address('Vote111111111111111111111111111111111111111');
        const ix = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
            maxTopUp: 3000,
            feePayer,
        });
        expect(ix.accounts).toHaveLength(6);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[5].address).toBe(feePayer);
        expect(ix.accounts[5].role).toBe(AccountRole.WRITABLE_SIGNER);
    });

    it('validation: zero amount throws "Amount must be positive"', () => {
        expect(() =>
            createTransferCheckedInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                mint: TEST_MINT,
                amount: 0n,
                authority: TEST_AUTHORITY,
                decimals: 9,
            }),
        ).toThrow('Amount must be positive');
    });

    it('validation: invalid decimals throws', () => {
        expect(() =>
            createTransferCheckedInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                mint: TEST_MINT,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                decimals: 256,
            }),
        ).toThrow('Decimals must be an integer between 0 and 255');
    });

    it('validation: non-integer decimals throws', () => {
        expect(() =>
            createTransferCheckedInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                mint: TEST_MINT,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                decimals: 6.5,
            }),
        ).toThrow('Decimals must be an integer between 0 and 255');
    });

    it('validation: negative decimals throws', () => {
        expect(() =>
            createTransferCheckedInstruction({
                source: TEST_SOURCE,
                destination: TEST_DEST,
                mint: TEST_MINT,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                decimals: -1,
            }),
        ).toThrow('Decimals must be an integer between 0 and 255');
    });
});

// ============================================================================
// TEST: createMintToInstruction
// ============================================================================

describe('createMintToInstruction', () => {
    it('has correct program address', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.accounts[0].address).toBe(TEST_MINT);
        expect(ix.accounts[1].address).toBe(TEST_DEST);
        expect(ix.accounts[2].address).toBe(TEST_AUTHORITY);
    });

    it('has correct account roles', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_TO);
        expect(ix.data[0]).toBe(7);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.MINT_TO);
        expect(decoded.amount).toBe(1_000_000n);
    });

    it('validation: zero amount throws "Amount must be positive"', () => {
        expect(() =>
            createMintToInstruction({
                mint: TEST_MINT,
                tokenAccount: TEST_DEST,
                mintAuthority: TEST_AUTHORITY,
                amount: 0n,
            }),
        ).toThrow('Amount must be positive');
    });
});

// ============================================================================
// TEST: createMintToCheckedInstruction
// ============================================================================

describe('createMintToCheckedInstruction', () => {
    it('has correct program address', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.accounts[0].address).toBe(TEST_MINT);
        expect(ix.accounts[1].address).toBe(TEST_DEST);
        expect(ix.accounts[2].address).toBe(TEST_AUTHORITY);
    });

    it('has correct account roles', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_TO_CHECKED);
        expect(ix.data[0]).toBe(14);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        const codec = getCheckedInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.MINT_TO_CHECKED);
        expect(decoded.amount).toBe(1_000_000n);
        expect(decoded.decimals).toBe(6);
    });

    it('validation: zero amount throws', () => {
        expect(() =>
            createMintToCheckedInstruction({
                mint: TEST_MINT,
                tokenAccount: TEST_DEST,
                mintAuthority: TEST_AUTHORITY,
                amount: 0n,
                decimals: 6,
            }),
        ).toThrow('Amount must be positive');
    });

    it('validation: invalid decimals throws', () => {
        expect(() =>
            createMintToCheckedInstruction({
                mint: TEST_MINT,
                tokenAccount: TEST_DEST,
                mintAuthority: TEST_AUTHORITY,
                amount: 1000n,
                decimals: 256,
            }),
        ).toThrow('Decimals must be an integer between 0 and 255');
    });
});

// ============================================================================
// TEST: createBurnInstruction
// ============================================================================

describe('createBurnInstruction', () => {
    it('has correct program address', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[2].address).toBe(TEST_AUTHORITY);
    });

    it('has correct account roles', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.BURN);
        expect(ix.data[0]).toBe(8);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.BURN);
        expect(decoded.amount).toBe(500n);
    });

    it('validation: zero amount throws "Amount must be positive"', () => {
        expect(() =>
            createBurnInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                authority: TEST_AUTHORITY,
                amount: 0n,
            }),
        ).toThrow('Amount must be positive');
    });
});

// ============================================================================
// TEST: createBurnCheckedInstruction
// ============================================================================

describe('createBurnCheckedInstruction', () => {
    it('has correct program address', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[2].address).toBe(TEST_AUTHORITY);
    });

    it('has correct account roles', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.BURN_CHECKED);
        expect(ix.data[0]).toBe(15);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        const codec = getCheckedInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.BURN_CHECKED);
        expect(decoded.amount).toBe(500n);
        expect(decoded.decimals).toBe(9);
    });

    it('validation: zero amount throws', () => {
        expect(() =>
            createBurnCheckedInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                authority: TEST_AUTHORITY,
                amount: 0n,
                decimals: 9,
            }),
        ).toThrow('Amount must be positive');
    });

    it('validation: invalid decimals throws', () => {
        expect(() =>
            createBurnCheckedInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                authority: TEST_AUTHORITY,
                amount: 500n,
                decimals: 256,
            }),
        ).toThrow('Decimals must be an integer between 0 and 255');
    });
});

// ============================================================================
// TEST: createApproveInstruction
// ============================================================================

describe('createApproveInstruction', () => {
    it('has correct program address', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_DELEGATE);
        expect(ix.accounts[2].address).toBe(TEST_OWNER);
    });

    it('has correct account roles', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.APPROVE);
        expect(ix.data[0]).toBe(4);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.APPROVE);
        expect(decoded.amount).toBe(10_000n);
    });

    it('validation: zero amount throws "Amount must be positive"', () => {
        expect(() =>
            createApproveInstruction({
                tokenAccount: TEST_SOURCE,
                delegate: TEST_DELEGATE,
                owner: TEST_OWNER,
                amount: 0n,
            }),
        ).toThrow('Amount must be positive');
    });
});

// ============================================================================
// TEST: createRevokeInstruction
// ============================================================================

describe('createRevokeInstruction', () => {
    it('has correct program address', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (2)', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.accounts).toHaveLength(2);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_OWNER);
    });

    it('has correct account roles', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.REVOKE);
        expect(ix.data[0]).toBe(5);
    });

    it('has discriminator-only data (1 byte)', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        expect(ix.data.length).toBe(1);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });
        const codec = getDiscriminatorOnlyCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.REVOKE);
    });
});

// ============================================================================
// TEST: createFreezeInstruction
// ============================================================================

describe('createFreezeInstruction', () => {
    it('has correct program address', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[2].address).toBe(TEST_FREEZE_AUTH);
    });

    it('has correct account roles', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.FREEZE);
        expect(ix.data[0]).toBe(10);
    });

    it('has discriminator-only data (1 byte)', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.data.length).toBe(1);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        const codec = getDiscriminatorOnlyCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.FREEZE);
    });
});

// ============================================================================
// TEST: createThawInstruction
// ============================================================================

describe('createThawInstruction', () => {
    it('has correct program address', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[2].address).toBe(TEST_FREEZE_AUTH);
    });

    it('has correct account roles', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.THAW);
        expect(ix.data[0]).toBe(11);
    });

    it('has discriminator-only data (1 byte)', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        expect(ix.data.length).toBe(1);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });
        const codec = getDiscriminatorOnlyCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.THAW);
    });
});

// ============================================================================
// TEST: createCloseAccountInstruction
// ============================================================================

describe('createCloseAccountInstruction', () => {
    it('has correct program address', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct number of accounts (3)', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.accounts).toHaveLength(3);
    });

    it('has correct account addresses in correct order', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[1].address).toBe(TEST_DEST);
        expect(ix.accounts[2].address).toBe(TEST_OWNER);
    });

    it('has correct account roles', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('has correct discriminator byte', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.CLOSE);
        expect(ix.data[0]).toBe(9);
    });

    it('has discriminator-only data (1 byte)', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        expect(ix.data.length).toBe(1);
    });

    it('has correct data encoding via codec round-trip', () => {
        const ix = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });
        const codec = getDiscriminatorOnlyCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.CLOSE);
    });
});

// ============================================================================
// TEST: createTransferInterfaceInstruction
// ============================================================================

describe('createTransferInterfaceInstruction', () => {
    it('light-to-light: returns transferType "light-to-light" with 1 instruction', () => {
        const result = createTransferInterfaceInstruction({
            sourceOwner: LIGHT_TOKEN_PROGRAM_ID,
            destOwner: LIGHT_TOKEN_PROGRAM_ID,
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            mint: TEST_MINT,
        });
        expect(result.transferType).toBe('light-to-light');
        expect(result.instructions).toHaveLength(1);
        expect(result.instructions[0].programAddress).toBe(
            LIGHT_TOKEN_PROGRAM_ID,
        );
    });

    it('light-to-light: instruction has correct discriminator and amount', () => {
        const result = createTransferInterfaceInstruction({
            sourceOwner: LIGHT_TOKEN_PROGRAM_ID,
            destOwner: LIGHT_TOKEN_PROGRAM_ID,
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 2000n,
            authority: TEST_AUTHORITY,
            mint: TEST_MINT,
        });
        const ix = result.instructions[0];
        const codec = getAmountInstructionCodec();
        const decoded = codec.decode(ix.data);
        expect(decoded.discriminator).toBe(DISCRIMINATOR.TRANSFER);
        expect(decoded.amount).toBe(2000n);
    });

    it('light-to-light: passes maxTopUp through', () => {
        const result = createTransferInterfaceInstruction({
            sourceOwner: LIGHT_TOKEN_PROGRAM_ID,
            destOwner: LIGHT_TOKEN_PROGRAM_ID,
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            mint: TEST_MINT,
            maxTopUp: 7000,
        });
        const ix = result.instructions[0];
        // Data should include maxTopUp suffix: 1 + 8 + 2 = 11
        expect(ix.data.length).toBe(11);
        const maxTopUp = decodeMaxTopUp(ix.data, 9);
        expect(maxTopUp).toBe(7000);
    });

    it('light-to-spl: throws', () => {
        const splProgram = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );
        expect(() =>
            createTransferInterfaceInstruction({
                sourceOwner: LIGHT_TOKEN_PROGRAM_ID,
                destOwner: splProgram,
                source: TEST_SOURCE,
                destination: TEST_DEST,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                mint: TEST_MINT,
            }),
        ).toThrow('Light-to-SPL transfer requires Transfer2');
    });

    it('spl-to-light: throws', () => {
        const splProgram = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );
        expect(() =>
            createTransferInterfaceInstruction({
                sourceOwner: splProgram,
                destOwner: LIGHT_TOKEN_PROGRAM_ID,
                source: TEST_SOURCE,
                destination: TEST_DEST,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                mint: TEST_MINT,
            }),
        ).toThrow('SPL-to-Light transfer requires Transfer2');
    });

    it('spl-to-spl: throws', () => {
        const splProgram = address(
            'TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA',
        );
        expect(() =>
            createTransferInterfaceInstruction({
                sourceOwner: splProgram,
                destOwner: splProgram,
                source: TEST_SOURCE,
                destination: TEST_DEST,
                amount: 1000n,
                authority: TEST_AUTHORITY,
                mint: TEST_MINT,
            }),
        ).toThrow('SPL-to-SPL transfers should use the SPL Token program');
    });
});

// ============================================================================
// TEST: createAssociatedTokenAccountInstruction
// ============================================================================

describe('createAssociatedTokenAccountInstruction', () => {
    it('has correct program address', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        expect(result.instruction.programAddress).toBe(
            LIGHT_TOKEN_PROGRAM_ID,
        );
    });

    it('has correct number of accounts (7)', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        expect(result.instruction.accounts).toHaveLength(7);
    });

    it('has correct account addresses in correct order', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        const accounts = result.instruction.accounts;
        expect(accounts[0].address).toBe(TEST_OWNER);
        expect(accounts[1].address).toBe(TEST_MINT);
        expect(accounts[2].address).toBe(TEST_PAYER);
        expect(accounts[3].address).toBe(result.address); // derived ATA
        expect(accounts[4].address).toBe(SYSTEM_PROGRAM_ID);
        expect(accounts[5].address).toBe(TEST_CONFIG);
        expect(accounts[6].address).toBe(TEST_SPONSOR);
    });

    it('has correct account roles', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        const accounts = result.instruction.accounts;
        expect(accounts[0].role).toBe(AccountRole.READONLY);       // owner
        expect(accounts[1].role).toBe(AccountRole.READONLY);       // mint
        expect(accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER); // payer
        expect(accounts[3].role).toBe(AccountRole.WRITABLE);       // ata
        expect(accounts[4].role).toBe(AccountRole.READONLY);       // systemProgram
        expect(accounts[5].role).toBe(AccountRole.READONLY);       // compressibleConfig
        expect(accounts[6].role).toBe(AccountRole.WRITABLE);       // rentSponsor
    });

    it('data starts with CREATE_ATA discriminator (100)', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.CREATE_ATA);
        expect(result.instruction.data[0]).toBe(100);
    });

    it('returns valid address and bump', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);
    });

    it('consistent PDA derivation across calls', async () => {
        const result1 = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        const result2 = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        expect(result1.address).toBe(result2.address);
        expect(result1.bump).toBe(result2.bump);
    });

    it('data length is greater than 1 (discriminator + encoded payload)', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });
        // discriminator (1) + bump (1) + compressibleConfig option prefix (1) + data
        expect(result.instruction.data.length).toBeGreaterThan(1);
    });
});

// ============================================================================
// TEST: createAssociatedTokenAccountIdempotentInstruction
// ============================================================================

describe('createAssociatedTokenAccountIdempotentInstruction', () => {
    it('has correct program address', async () => {
        const result =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        expect(result.instruction.programAddress).toBe(
            LIGHT_TOKEN_PROGRAM_ID,
        );
    });

    it('has correct number of accounts (7)', async () => {
        const result =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        expect(result.instruction.accounts).toHaveLength(7);
    });

    it('data starts with CREATE_ATA_IDEMPOTENT discriminator (102)', async () => {
        const result =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        expect(result.instruction.data[0]).toBe(
            DISCRIMINATOR.CREATE_ATA_IDEMPOTENT,
        );
        expect(result.instruction.data[0]).toBe(102);
    });

    it('consistent PDA derivation matches non-idempotent variant', async () => {
        const normalResult =
            await createAssociatedTokenAccountInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        const idempotentResult =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        expect(normalResult.address).toBe(idempotentResult.address);
        expect(normalResult.bump).toBe(idempotentResult.bump);
    });

    it('has same account structure as non-idempotent variant', async () => {
        const normalResult =
            await createAssociatedTokenAccountInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        const idempotentResult =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });

        // Same number of accounts
        expect(idempotentResult.instruction.accounts).toHaveLength(
            normalResult.instruction.accounts.length,
        );

        // Same account addresses and roles
        for (let i = 0; i < normalResult.instruction.accounts.length; i++) {
            expect(idempotentResult.instruction.accounts[i].address).toBe(
                normalResult.instruction.accounts[i].address,
            );
            expect(idempotentResult.instruction.accounts[i].role).toBe(
                normalResult.instruction.accounts[i].role,
            );
        }
    });

    it('only differs from non-idempotent in discriminator byte', async () => {
        const normalResult =
            await createAssociatedTokenAccountInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });
        const idempotentResult =
            await createAssociatedTokenAccountIdempotentInstruction({
                payer: TEST_PAYER,
                owner: TEST_OWNER,
                mint: TEST_MINT,
                compressibleConfig: TEST_CONFIG,
                rentSponsor: TEST_SPONSOR,
            });

        // Discriminators differ
        expect(normalResult.instruction.data[0]).toBe(100);
        expect(idempotentResult.instruction.data[0]).toBe(102);

        // Rest of data is identical
        const normalPayload = normalResult.instruction.data.slice(1);
        const idempotentPayload = idempotentResult.instruction.data.slice(1);
        expect(normalPayload).toEqual(idempotentPayload);
    });
});

// ============================================================================
// TEST: AccountRole enum values
// ============================================================================

describe('AccountRole enum values', () => {
    it('READONLY = 0', () => {
        expect(AccountRole.READONLY).toBe(0);
    });

    it('WRITABLE = 1', () => {
        expect(AccountRole.WRITABLE).toBe(1);
    });

    it('READONLY_SIGNER = 2', () => {
        expect(AccountRole.READONLY_SIGNER).toBe(2);
    });

    it('WRITABLE_SIGNER = 3', () => {
        expect(AccountRole.WRITABLE_SIGNER).toBe(3);
    });
});

// ============================================================================
// TEST: DISCRIMINATOR constant values
// ============================================================================

describe('DISCRIMINATOR constant values', () => {
    it('TRANSFER = 3', () => {
        expect(DISCRIMINATOR.TRANSFER).toBe(3);
    });

    it('APPROVE = 4', () => {
        expect(DISCRIMINATOR.APPROVE).toBe(4);
    });

    it('REVOKE = 5', () => {
        expect(DISCRIMINATOR.REVOKE).toBe(5);
    });

    it('MINT_TO = 7', () => {
        expect(DISCRIMINATOR.MINT_TO).toBe(7);
    });

    it('BURN = 8', () => {
        expect(DISCRIMINATOR.BURN).toBe(8);
    });

    it('CLOSE = 9', () => {
        expect(DISCRIMINATOR.CLOSE).toBe(9);
    });

    it('FREEZE = 10', () => {
        expect(DISCRIMINATOR.FREEZE).toBe(10);
    });

    it('THAW = 11', () => {
        expect(DISCRIMINATOR.THAW).toBe(11);
    });

    it('TRANSFER_CHECKED = 12', () => {
        expect(DISCRIMINATOR.TRANSFER_CHECKED).toBe(12);
    });

    it('MINT_TO_CHECKED = 14', () => {
        expect(DISCRIMINATOR.MINT_TO_CHECKED).toBe(14);
    });

    it('BURN_CHECKED = 15', () => {
        expect(DISCRIMINATOR.BURN_CHECKED).toBe(15);
    });

    it('CREATE_ATA = 100', () => {
        expect(DISCRIMINATOR.CREATE_ATA).toBe(100);
    });

    it('CREATE_ATA_IDEMPOTENT = 102', () => {
        expect(DISCRIMINATOR.CREATE_ATA_IDEMPOTENT).toBe(102);
    });
});
