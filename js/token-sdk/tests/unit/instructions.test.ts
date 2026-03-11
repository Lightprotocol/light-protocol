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
import { address, getAddressCodec } from '@solana/addresses';
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
    createTokenAccountInstruction,
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
    createTransfer2Instruction,
    createClaimInstruction,
    createWithdrawFundingPoolInstruction,
    createMintActionInstruction,

    // Compression factory functions
    createCompress,
    createCompressSpl,
    createDecompress,
    createDecompressSpl,
    createCompressAndClose,

    // Constants
    LIGHT_TOKEN_PROGRAM_ID,
    LIGHT_SYSTEM_PROGRAM_ID,
    CPI_AUTHORITY,
    REGISTERED_PROGRAM_PDA,
    ACCOUNT_COMPRESSION_AUTHORITY_PDA,
    DISCRIMINATOR,
    SYSTEM_PROGRAM_ID,
    ACCOUNT_COMPRESSION_PROGRAM_ID,
    COMPRESSION_MODE,

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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
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

    it('with maxTopUp has 11-byte data and authority is WRITABLE_SIGNER', () => {
        const ix = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            maxTopUp: 5000,
        });
        // 1 (disc) + 8 (amount) + 2 (maxTopUp u16) = 11 bytes
        expect(ix.data.length).toBe(11);
        // authority is WRITABLE_SIGNER (default when no feePayer)
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
        expect(ix.accounts[3].role).toBe(AccountRole.WRITABLE_SIGNER);
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

    it('has correct number of accounts (4)', () => {
        const ix = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });
        expect(ix.accounts).toHaveLength(4);
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
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
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

    it('has correct number of accounts (4)', () => {
        const ix = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });
        expect(ix.accounts).toHaveLength(4);
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
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
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

    it('has correct number of accounts (4)', () => {
        const ix = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });
        expect(ix.accounts).toHaveLength(4);
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
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
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

    it('has correct number of accounts (4)', () => {
        const ix = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });
        expect(ix.accounts).toHaveLength(4);
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
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
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
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
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
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE_SIGNER);
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
// TEST: createTokenAccountInstruction
// ============================================================================

describe('createTokenAccountInstruction', () => {
    it('non-compressible path has 2 accounts and discriminator 18', () => {
        const ix = createTokenAccountInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            owner: TEST_OWNER,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(ix.accounts).toHaveLength(2);
        expect(ix.accounts[0].address).toBe(TEST_SOURCE);
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].address).toBe(TEST_MINT);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY);
        expect(ix.data[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);
    });

    it('compressible path includes payer/config/system/rent accounts', () => {
        const ix = createTokenAccountInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            owner: TEST_OWNER,
            payer: TEST_PAYER,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
            compressibleParams: {
                tokenAccountVersion: 3,
                rentPayment: 16,
                compressionOnly: 0,
                writeTopUp: 766,
                compressToPubkey: null,
            },
        });
        expect(ix.accounts).toHaveLength(6);
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[2].address).toBe(TEST_PAYER);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        expect(ix.accounts[3].address).toBe(TEST_CONFIG);
        expect(ix.accounts[4].address).toBe(SYSTEM_PROGRAM_ID);
        expect(ix.accounts[5].address).toBe(TEST_SPONSOR);
        expect(ix.data[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);
        expect(ix.data.length).toBeGreaterThan(33);
    });

    it('throws when compressibleParams is set without payer', () => {
        expect(() =>
            createTokenAccountInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                owner: TEST_OWNER,
                compressibleParams: {
                    tokenAccountVersion: 3,
                    rentPayment: 16,
                    compressionOnly: 0,
                    writeTopUp: 766,
                    compressToPubkey: null,
                },
            }),
        ).toThrow('payer is required when compressibleParams is provided');
    });

    it('throws when compressible-only accounts are provided without compressibleParams', () => {
        expect(() =>
            createTokenAccountInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                owner: TEST_OWNER,
                payer: TEST_PAYER,
            }),
        ).toThrow('payer/compressibleConfig/rentSponsor require compressibleParams');
    });

    it('supports SPL-compatible owner-only payload mode', () => {
        const ix = createTokenAccountInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            owner: TEST_OWNER,
            splCompatibleOwnerOnlyData: true,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT);
        expect(ix.data).toHaveLength(33);
        expect(ix.data.slice(1)).toEqual(
            new Uint8Array(getAddressCodec().encode(TEST_OWNER)),
        );
    });

    it('throws when SPL-compatible owner-only mode is used with compressible params', () => {
        expect(() =>
            createTokenAccountInstruction({
                tokenAccount: TEST_SOURCE,
                mint: TEST_MINT,
                owner: TEST_OWNER,
                payer: TEST_PAYER,
                splCompatibleOwnerOnlyData: true,
                compressibleParams: {
                    tokenAccountVersion: 3,
                    rentPayment: 16,
                    compressionOnly: 0,
                    writeTopUp: 766,
                    compressToPubkey: null,
                },
            }),
        ).toThrow(
            'splCompatibleOwnerOnlyData is only valid for non-compressible token account creation',
        );
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
        // discriminator (1) + compressibleConfig option prefix (1) + data
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

    it('CREATE_TOKEN_ACCOUNT = 18', () => {
        expect(DISCRIMINATOR.CREATE_TOKEN_ACCOUNT).toBe(18);
    });

    it('CREATE_ATA = 100', () => {
        expect(DISCRIMINATOR.CREATE_ATA).toBe(100);
    });

    it('CREATE_ATA_IDEMPOTENT = 102', () => {
        expect(DISCRIMINATOR.CREATE_ATA_IDEMPOTENT).toBe(102);
    });

    it('TRANSFER2 = 101', () => {
        expect(DISCRIMINATOR.TRANSFER2).toBe(101);
    });

    it('MINT_ACTION = 103', () => {
        expect(DISCRIMINATOR.MINT_ACTION).toBe(103);
    });
});

// ============================================================================
// TEST: createApproveInstruction with maxTopUp (no feePayer - Rust doesn't support it)
// ============================================================================

describe('createApproveInstruction (maxTopUp)', () => {
    it('includes maxTopUp in data when provided', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
            maxTopUp: 500,
        });
        // disc(1) + amount(8) + maxTopUp(2) = 11
        expect(ix.data.length).toBe(11);
        const maxTopUp = decodeMaxTopUp(ix.data, 9);
        expect(maxTopUp).toBe(500);
    });

    it('owner is always WRITABLE_SIGNER (payer at APPROVE_PAYER_IDX=2)', () => {
        const ix = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
            maxTopUp: 500,
        });
        // Always 3 accounts, no separate feePayer
        expect(ix.accounts).toHaveLength(3);
        expect(ix.accounts[2].address).toBe(TEST_OWNER);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
    });
});

// ============================================================================
// TEST: createRevokeInstruction with maxTopUp (no feePayer - Rust doesn't support it)
// ============================================================================

describe('createRevokeInstruction (maxTopUp)', () => {
    it('includes maxTopUp in data when provided', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
            maxTopUp: 1000,
        });
        // disc(1) + maxTopUp(2) = 3
        expect(ix.data.length).toBe(3);
        const maxTopUp = decodeMaxTopUp(ix.data, 1);
        expect(maxTopUp).toBe(1000);
    });

    it('owner is always WRITABLE_SIGNER (payer at REVOKE_PAYER_IDX=1)', () => {
        const ix = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
            maxTopUp: 1000,
        });
        // Always 2 accounts, no separate feePayer
        expect(ix.accounts).toHaveLength(2);
        expect(ix.accounts[1].address).toBe(TEST_OWNER);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE_SIGNER);
    });
});

// ============================================================================
// TEST: createTransfer2Instruction
// ============================================================================

describe('createTransfer2Instruction', () => {
    it('Path A: compression-only has cpiAuthority + feePayer + packed accounts', () => {
        const ix = createTransfer2Instruction({
            feePayer: TEST_PAYER,
            packedAccounts: [
                { address: TEST_MINT, role: AccountRole.READONLY },
                { address: TEST_SOURCE, role: AccountRole.WRITABLE },
            ],
            data: {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 65535,
                cpiContext: null,
                compressions: [{
                    mode: 0, amount: 1000n, mint: 0, sourceOrRecipient: 1,
                    authority: 0, poolAccountIndex: 0, poolIndex: 0, bump: 0, decimals: 2,
                }],
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            },
        });
        // Path A: 2 fixed + 2 packed = 4
        expect(ix.accounts).toHaveLength(4);
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
    });

    it('Path A: packed accounts preserve their roles', () => {
        const ix = createTransfer2Instruction({
            feePayer: TEST_PAYER,
            packedAccounts: [
                { address: TEST_MINT, role: AccountRole.READONLY },
                { address: TEST_SOURCE, role: AccountRole.WRITABLE },
                { address: TEST_OWNER, role: AccountRole.READONLY_SIGNER },
            ],
            data: {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 65535,
                cpiContext: null,
                compressions: [{
                    mode: 0, amount: 1000n, mint: 0, sourceOrRecipient: 1,
                    authority: 2, poolAccountIndex: 0, poolIndex: 0, bump: 0, decimals: 0,
                }],
                proof: null,
                inTokenData: [],
                outTokenData: [],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            },
        });
        // 2 fixed + 3 packed = 5
        expect(ix.accounts).toHaveLength(5);
        // Packed accounts start at index 2
        expect(ix.accounts[2].address).toBe(TEST_MINT);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[3].address).toBe(TEST_SOURCE);
        expect(ix.accounts[3].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[4].address).toBe(TEST_OWNER);
        expect(ix.accounts[4].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('Path B: full transfer has 7+ fixed accounts', () => {
        const ix = createTransfer2Instruction({
            feePayer: TEST_PAYER,
            packedAccounts: [
                { address: TEST_MINT, role: AccountRole.READONLY },
            ],
            data: {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 65535,
                cpiContext: null,
                compressions: null,
                proof: null,
                inTokenData: [{
                    owner: 0, amount: 1000n, hasDelegate: false, delegate: 0,
                    mint: 0, version: 3,
                    merkleContext: { merkleTreePubkeyIndex: 0, queuePubkeyIndex: 0, leafIndex: 0, proveByIndex: true },
                    rootIndex: 0,
                }],
                outTokenData: [{
                    owner: 0, amount: 1000n, hasDelegate: false, delegate: 0,
                    mint: 0, version: 3,
                }],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            },
        });
        // Path B: 7 fixed + 1 packed = 8
        expect(ix.accounts).toHaveLength(8);
        expect(ix.data[0]).toBe(DISCRIMINATOR.TRANSFER2);
        // Rust parity defaults for system CPI accounts
        expect(ix.accounts[3].address).toBe(REGISTERED_PROGRAM_PDA);
        expect(ix.accounts[4].address).toBe(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
        );
        // Packed account at index 7 preserves readonly role
        expect(ix.accounts[7].address).toBe(TEST_MINT);
        expect(ix.accounts[7].role).toBe(AccountRole.READONLY);
    });

    it('Path C: CPI context write has lightSystemProgram + feePayer + cpiAuthority + cpiContext + packed', () => {
        const cpiContextAccount = address('Sysvar1111111111111111111111111111111111111');
        const ix = createTransfer2Instruction({
            feePayer: TEST_PAYER,
            cpiContextAccount,
            packedAccounts: [
                { address: TEST_MINT, role: AccountRole.READONLY },
                { address: TEST_SOURCE, role: AccountRole.WRITABLE },
            ],
            data: {
                withTransactionHash: false,
                withLamportsChangeAccountMerkleTreeIndex: false,
                lamportsChangeAccountMerkleTreeIndex: 0,
                lamportsChangeAccountOwnerIndex: 0,
                outputQueue: 0,
                maxTopUp: 65535,
                cpiContext: { setContext: true, firstSetContext: true },
                compressions: null,
                proof: null,
                inTokenData: [{
                    owner: 0, amount: 1000n, hasDelegate: false, delegate: 0,
                    mint: 0, version: 3,
                    merkleContext: { merkleTreePubkeyIndex: 0, queuePubkeyIndex: 0, leafIndex: 0, proveByIndex: true },
                    rootIndex: 0,
                }],
                outTokenData: [{
                    owner: 0, amount: 1000n, hasDelegate: false, delegate: 0,
                    mint: 0, version: 3,
                }],
                inLamports: null,
                outLamports: null,
                inTlv: null,
                outTlv: null,
            },
        });
        // Path C: 4 fixed + 2 packed = 6
        expect(ix.accounts).toHaveLength(6);
        // Account 0: lightSystemProgram (readonly)
        expect(ix.accounts[0].address).toBe(LIGHT_SYSTEM_PROGRAM_ID);
        expect(ix.accounts[0].role).toBe(AccountRole.READONLY);
        // Account 1: feePayer (writable signer)
        expect(ix.accounts[1].address).toBe(TEST_PAYER);
        expect(ix.accounts[1].role).toBe(AccountRole.WRITABLE_SIGNER);
        // Account 2: cpiAuthorityPda (readonly)
        expect(ix.accounts[2].address).toBe(CPI_AUTHORITY);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY);
        // Account 3: cpiContext (writable — program writes CPI data to it)
        expect(ix.accounts[3].address).toBe(cpiContextAccount);
        expect(ix.accounts[3].role).toBe(AccountRole.WRITABLE);
        // Packed accounts
        expect(ix.accounts[4].address).toBe(TEST_MINT);
        expect(ix.accounts[5].address).toBe(TEST_SOURCE);
    });
});

// ============================================================================
// TEST: Compression factory functions
// ============================================================================

describe('Compression factory functions', () => {
    it('createCompress: CToken compression', () => {
        const comp = createCompress({
            amount: 5000n,
            mintIndex: 2,
            sourceIndex: 1,
            authorityIndex: 0,
        });
        expect(comp.mode).toBe(COMPRESSION_MODE.COMPRESS);
        expect(comp.amount).toBe(5000n);
        expect(comp.mint).toBe(2);
        expect(comp.sourceOrRecipient).toBe(1);
        expect(comp.authority).toBe(0);
        expect(comp.poolAccountIndex).toBe(0);
        expect(comp.poolIndex).toBe(0);
        expect(comp.bump).toBe(0);
        expect(comp.decimals).toBe(0);
    });

    it('createCompressSpl: SPL compression', () => {
        const comp = createCompressSpl({
            amount: 5000n,
            mintIndex: 3,
            sourceIndex: 4,
            authorityIndex: 0,
            poolAccountIndex: 5,
            poolIndex: 1,
            bump: 254,
            decimals: 6,
        });
        expect(comp.mode).toBe(COMPRESSION_MODE.COMPRESS);
        expect(comp.amount).toBe(5000n);
        expect(comp.mint).toBe(3);
        expect(comp.sourceOrRecipient).toBe(4);
        expect(comp.authority).toBe(0);
        expect(comp.poolAccountIndex).toBe(5);
        expect(comp.poolIndex).toBe(1);
        expect(comp.bump).toBe(254);
        expect(comp.decimals).toBe(6);
    });

    it('createDecompress: CToken decompression', () => {
        const comp = createDecompress({
            amount: 3000n,
            mintIndex: 2,
            recipientIndex: 7,
        });
        expect(comp.mode).toBe(COMPRESSION_MODE.DECOMPRESS);
        expect(comp.amount).toBe(3000n);
        expect(comp.mint).toBe(2);
        expect(comp.sourceOrRecipient).toBe(7);
        expect(comp.authority).toBe(0);
        expect(comp.poolAccountIndex).toBe(0);
    });

    it('createDecompressSpl: SPL decompression', () => {
        const comp = createDecompressSpl({
            amount: 2000n,
            mintIndex: 3,
            recipientIndex: 8,
            poolAccountIndex: 9,
            poolIndex: 0,
            bump: 123,
            decimals: 9,
        });
        expect(comp.mode).toBe(COMPRESSION_MODE.DECOMPRESS);
        expect(comp.amount).toBe(2000n);
        expect(comp.sourceOrRecipient).toBe(8);
        expect(comp.authority).toBe(0);
        expect(comp.poolAccountIndex).toBe(9);
        expect(comp.poolIndex).toBe(0);
        expect(comp.bump).toBe(123);
        expect(comp.decimals).toBe(9);
    });

    it('createCompressAndClose: repurposed fields', () => {
        const comp = createCompressAndClose({
            amount: 1000n,
            mintIndex: 2,
            sourceIndex: 1,
            authorityIndex: 0,
            rentSponsorIndex: 10,
            compressedAccountIndex: 11,
            destinationIndex: 5,
        });
        expect(comp.mode).toBe(COMPRESSION_MODE.COMPRESS_AND_CLOSE);
        expect(comp.amount).toBe(1000n);
        expect(comp.mint).toBe(2);
        expect(comp.sourceOrRecipient).toBe(1);
        expect(comp.authority).toBe(0);
        // Repurposed fields
        expect(comp.poolAccountIndex).toBe(10); // rentSponsorIndex
        expect(comp.poolIndex).toBe(11);         // compressedAccountIndex
        expect(comp.bump).toBe(5);               // destinationIndex
        expect(comp.decimals).toBe(0);
    });
});

// ============================================================================
// TEST: createClaimInstruction
// ============================================================================

describe('createClaimInstruction', () => {
    it('builds correct instruction with discriminator and accounts', () => {
        const ix = createClaimInstruction({
            rentSponsor: TEST_PAYER,
            compressionAuthority: TEST_AUTHORITY,
            compressibleConfig: TEST_MINT,
            tokenAccounts: [TEST_SOURCE, TEST_DEST],
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        // 3 fixed + 2 token accounts = 5
        expect(ix.accounts).toHaveLength(5);

        // Account roles
        expect(ix.accounts[0].address).toBe(TEST_PAYER);
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].address).toBe(TEST_AUTHORITY);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[2].address).toBe(TEST_MINT);
        expect(ix.accounts[2].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[3].address).toBe(TEST_SOURCE);
        expect(ix.accounts[3].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[4].address).toBe(TEST_DEST);
        expect(ix.accounts[4].role).toBe(AccountRole.WRITABLE);

        // Data: discriminator only (no instruction data)
        expect(ix.data).toHaveLength(1);
        expect(ix.data[0]).toBe(DISCRIMINATOR.CLAIM);
    });

    it('works with no token accounts', () => {
        const ix = createClaimInstruction({
            rentSponsor: TEST_PAYER,
            compressionAuthority: TEST_AUTHORITY,
            compressibleConfig: TEST_MINT,
            tokenAccounts: [],
        });
        expect(ix.accounts).toHaveLength(3);
    });
});

// ============================================================================
// TEST: createWithdrawFundingPoolInstruction
// ============================================================================

describe('createWithdrawFundingPoolInstruction', () => {
    it('builds correct instruction with amount encoding', () => {
        const ix = createWithdrawFundingPoolInstruction({
            rentSponsor: TEST_PAYER,
            compressionAuthority: TEST_AUTHORITY,
            destination: TEST_DEST,
            compressibleConfig: TEST_MINT,
            amount: 1_000_000_000n,
        });

        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(ix.accounts).toHaveLength(5);

        // Account roles
        expect(ix.accounts[0].address).toBe(TEST_PAYER);
        expect(ix.accounts[0].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[1].address).toBe(TEST_AUTHORITY);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY_SIGNER);
        expect(ix.accounts[2].address).toBe(TEST_DEST);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE);
        expect(ix.accounts[3].address).toBe(SYSTEM_PROGRAM_ID);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
        expect(ix.accounts[4].address).toBe(TEST_MINT);
        expect(ix.accounts[4].role).toBe(AccountRole.READONLY);

        // Data: discriminator (1) + u64 amount (8) = 9 bytes
        expect(ix.data).toHaveLength(9);
        expect(ix.data[0]).toBe(DISCRIMINATOR.WITHDRAW_FUNDING_POOL);

        // Decode amount (LE u64)
        const view = new DataView(ix.data.buffer, ix.data.byteOffset);
        const amount = view.getBigUint64(1, true);
        expect(amount).toBe(1_000_000_000n);
    });

    it('encodes zero amount', () => {
        const ix = createWithdrawFundingPoolInstruction({
            rentSponsor: TEST_PAYER,
            compressionAuthority: TEST_AUTHORITY,
            destination: TEST_DEST,
            compressibleConfig: TEST_MINT,
            amount: 0n,
        });

        const view = new DataView(ix.data.buffer, ix.data.byteOffset);
        expect(view.getBigUint64(1, true)).toBe(0n);
    });

    it('encodes large amount', () => {
        const largeAmount = 18_446_744_073_709_551_615n; // u64::MAX
        const ix = createWithdrawFundingPoolInstruction({
            rentSponsor: TEST_PAYER,
            compressionAuthority: TEST_AUTHORITY,
            destination: TEST_DEST,
            compressibleConfig: TEST_MINT,
            amount: largeAmount,
        });

        const view = new DataView(ix.data.buffer, ix.data.byteOffset);
        expect(view.getBigUint64(1, true)).toBe(largeAmount);
    });
});

// ============================================================================
// TEST: createMintActionInstruction
// ============================================================================

describe('createMintActionInstruction', () => {
    const TEST_OUT_QUEUE = address('Vote111111111111111111111111111111111111111');
    const TEST_MERKLE_TREE = address('BPFLoaderUpgradeab1e11111111111111111111111');
    const mintActionData = {
        leafIndex: 0,
        proveByIndex: false,
        rootIndex: 0,
        maxTopUp: 0,
        createMint: null,
        actions: [] as [],
        proof: null,
        cpiContext: null,
        mint: null,
    };

    it('has correct program address', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });
        expect(ix.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('has correct discriminator byte (103)', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });
        expect(ix.data[0]).toBe(DISCRIMINATOR.MINT_ACTION);
        expect(ix.data[0]).toBe(103);
    });

    it('normal path: lightSystemProgram, authority, LightSystemAccounts(6), queues, tree', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });

        // lightSystemProgram(1) + authority(1) + LightSystemAccounts(6) + outQueue(1) + merkleTree(1) = 10
        expect(ix.accounts).toHaveLength(10);

        // Account 0: Light System Program (readonly)
        expect(ix.accounts[0].address).toBe(LIGHT_SYSTEM_PROGRAM_ID);
        expect(ix.accounts[0].role).toBe(AccountRole.READONLY);

        // Account 1: authority (signer)
        expect(ix.accounts[1].address).toBe(TEST_AUTHORITY);
        expect(ix.accounts[1].role).toBe(AccountRole.READONLY_SIGNER);

        // LightSystemAccounts (6 accounts):
        // 2: feePayer (writable signer)
        expect(ix.accounts[2].address).toBe(TEST_PAYER);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        // 3: cpiAuthorityPda (readonly)
        expect(ix.accounts[3].address).toBe(CPI_AUTHORITY);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
        // 4: registeredProgramPda (readonly, defaults to REGISTERED_PROGRAM_PDA)
        expect(ix.accounts[4].address).toBe(REGISTERED_PROGRAM_PDA);
        expect(ix.accounts[4].role).toBe(AccountRole.READONLY);
        // 5: accountCompressionAuthority (readonly, defaults to ACCOUNT_COMPRESSION_AUTHORITY_PDA)
        expect(ix.accounts[5].address).toBe(
            ACCOUNT_COMPRESSION_AUTHORITY_PDA,
        );
        expect(ix.accounts[5].role).toBe(AccountRole.READONLY);
        // 6: accountCompressionProgram (readonly)
        expect(ix.accounts[6].address).toBe(ACCOUNT_COMPRESSION_PROGRAM_ID);
        expect(ix.accounts[6].role).toBe(AccountRole.READONLY);
        // 7: systemProgram (readonly)
        expect(ix.accounts[7].address).toBe(SYSTEM_PROGRAM_ID);
        expect(ix.accounts[7].role).toBe(AccountRole.READONLY);

        // 8: outOutputQueue (writable)
        expect(ix.accounts[8].address).toBe(TEST_OUT_QUEUE);
        expect(ix.accounts[8].role).toBe(AccountRole.WRITABLE);
        // 9: merkleTree (writable)
        expect(ix.accounts[9].address).toBe(TEST_MERKLE_TREE);
        expect(ix.accounts[9].role).toBe(AccountRole.WRITABLE);
    });

    it('includes CPI_AUTHORITY, ACCOUNT_COMPRESSION_PROGRAM_ID, SYSTEM_PROGRAM_ID', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });

        const addresses = ix.accounts.map(a => a.address);
        expect(addresses).toContain(CPI_AUTHORITY);
        expect(addresses).toContain(ACCOUNT_COMPRESSION_PROGRAM_ID);
        expect(addresses).toContain(SYSTEM_PROGRAM_ID);
    });

    it('output queue and merkle tree are writable', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });

        const outQueueAccount = ix.accounts.find(a => a.address === TEST_OUT_QUEUE);
        const treeAccount = ix.accounts.find(a => a.address === TEST_MERKLE_TREE);
        expect(outQueueAccount?.role).toBe(AccountRole.WRITABLE);
        expect(treeAccount?.role).toBe(AccountRole.WRITABLE);
    });

    it('with mintSigner: adds it as signer for createMint', () => {
        const mintSigner = address('Sysvar1111111111111111111111111111111111111');
        const ix = createMintActionInstruction({
            mintSigner,
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: {
                ...mintActionData,
                createMint: {
                    readOnlyAddressTrees: new Uint8Array(4),
                    readOnlyAddressTreeRootIndices: [0, 0, 0, 0],
                },
            },
        });

        const signerAccount = ix.accounts.find(a => a.address === mintSigner);
        expect(signerAccount).toBeDefined();
        expect(signerAccount?.role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('with mintSigner but no createMint: adds as readonly', () => {
        const mintSigner = address('Sysvar1111111111111111111111111111111111111');
        const ix = createMintActionInstruction({
            mintSigner,
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });

        const signerAccount = ix.accounts.find(a => a.address === mintSigner);
        expect(signerAccount).toBeDefined();
        expect(signerAccount?.role).toBe(AccountRole.READONLY);
    });

    it('packed accounts preserve their roles', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            packedAccounts: [
                { address: TEST_SOURCE, role: AccountRole.WRITABLE },
                { address: TEST_DEST, role: AccountRole.READONLY },
                { address: TEST_OWNER, role: AccountRole.READONLY_SIGNER },
            ],
            data: mintActionData,
        });

        // Packed accounts at the end
        const lastThree = ix.accounts.slice(-3);
        expect(lastThree[0].address).toBe(TEST_SOURCE);
        expect(lastThree[0].role).toBe(AccountRole.WRITABLE);
        expect(lastThree[1].address).toBe(TEST_DEST);
        expect(lastThree[1].role).toBe(AccountRole.READONLY);
        expect(lastThree[2].address).toBe(TEST_OWNER);
        expect(lastThree[2].role).toBe(AccountRole.READONLY_SIGNER);
    });

    it('optional accounts: compressibleConfig, cmint, rentSponsor', () => {
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            compressibleConfig: TEST_CONFIG,
            cmint: TEST_SOURCE,
            rentSponsor: TEST_SPONSOR,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            data: mintActionData,
        });

        const addresses = ix.accounts.map(a => a.address);
        expect(addresses).toContain(TEST_CONFIG);
        expect(addresses).toContain(TEST_SOURCE);
        expect(addresses).toContain(TEST_SPONSOR);

        // Config is readonly, cmint and rentSponsor are writable
        const configAccount = ix.accounts.find(a => a.address === TEST_CONFIG);
        expect(configAccount?.role).toBe(AccountRole.READONLY);
        const cmintAccount = ix.accounts.find(a => a.address === TEST_SOURCE);
        expect(cmintAccount?.role).toBe(AccountRole.WRITABLE);
        const sponsorAccount = ix.accounts.find(a => a.address === TEST_SPONSOR);
        expect(sponsorAccount?.role).toBe(AccountRole.WRITABLE);
    });

    it('CPI context path: feePayer + cpiAuthorityPda + cpiContext (3 accounts)', () => {
        const cpiContext = address('Sysvar1111111111111111111111111111111111111');
        const ix = createMintActionInstruction({
            authority: TEST_AUTHORITY,
            feePayer: TEST_PAYER,
            outOutputQueue: TEST_OUT_QUEUE,
            merkleTree: TEST_MERKLE_TREE,
            cpiContextAccounts: {
                feePayer: TEST_PAYER,
                cpiAuthorityPda: CPI_AUTHORITY,
                cpiContext,
            },
            data: mintActionData,
        });

        // CPI context path: lightSystemProgram(1) + authority(1) + CpiContextLightSystemAccounts(3) = 5
        expect(ix.accounts).toHaveLength(5);

        // Account 0: Light System Program
        expect(ix.accounts[0].address).toBe(LIGHT_SYSTEM_PROGRAM_ID);
        // Account 1: authority
        expect(ix.accounts[1].address).toBe(TEST_AUTHORITY);
        // Account 2: feePayer (writable signer)
        expect(ix.accounts[2].address).toBe(TEST_PAYER);
        expect(ix.accounts[2].role).toBe(AccountRole.WRITABLE_SIGNER);
        // Account 3: cpiAuthorityPda (readonly)
        expect(ix.accounts[3].address).toBe(CPI_AUTHORITY);
        expect(ix.accounts[3].role).toBe(AccountRole.READONLY);
        // Account 4: cpiContext (writable — program writes CPI data to it)
        expect(ix.accounts[4].address).toBe(cpiContext);
        expect(ix.accounts[4].role).toBe(AccountRole.WRITABLE);
    });
});
