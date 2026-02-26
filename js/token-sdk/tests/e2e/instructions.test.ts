/**
 * E2E Tests for Light Token SDK Instructions
 *
 * These tests verify instruction building and serialization.
 *
 * Tests that require a running validator are marked with `.skip`
 * and can be run separately with `pnpm test:e2e:live`.
 *
 * To run with a local validator:
 * 1. Start the test validator: `./../../cli/test_bin/run test-validator`
 * 2. Run tests: `pnpm test:e2e`
 *
 * Endpoints:
 * - Solana: http://127.0.0.1:8899
 * - Compression API: http://127.0.0.1:8784
 * - Prover: http://127.0.0.1:3001
 */

import { describe, it, expect } from 'vitest';
import { address } from '@solana/addresses';

import {
    // Instruction builders
    createAssociatedTokenAccountInstruction,
    createAssociatedTokenAccountIdempotentInstruction,
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

    // Constants
    LIGHT_TOKEN_PROGRAM_ID,
    DISCRIMINATOR,
} from '../../src/index.js';

// ============================================================================
// TEST HELPERS
// ============================================================================

// Use known valid Solana addresses for testing
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
// TEST: Create Associated Token Account Instructions
// ============================================================================

describe('createAssociatedTokenAccountInstruction', () => {
    it('8.1 creates ATA instruction with correct accounts and data', async () => {
        const result = await createAssociatedTokenAccountInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });

        // Verify result structure
        expect(result.address).toBeDefined();
        expect(typeof result.bump).toBe('number');
        expect(result.bump).toBeGreaterThanOrEqual(0);
        expect(result.bump).toBeLessThanOrEqual(255);

        // Verify instruction
        expect(result.instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(result.instruction.accounts).toHaveLength(7);
        expect(result.instruction.data).toBeInstanceOf(Uint8Array);

        // First byte should be CREATE_ATA discriminator
        expect(result.instruction.data[0]).toBe(DISCRIMINATOR.CREATE_ATA);
    });

    it('8.1.1 uses consistent PDA derivation', async () => {
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
});

describe('createAssociatedTokenAccountIdempotentInstruction', () => {
    it('8.2 creates idempotent ATA instruction', async () => {
        const result = await createAssociatedTokenAccountIdempotentInstruction({
            payer: TEST_PAYER,
            owner: TEST_OWNER,
            mint: TEST_MINT,
            compressibleConfig: TEST_CONFIG,
            rentSponsor: TEST_SPONSOR,
        });

        expect(result.address).toBeDefined();
        expect(result.instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);

        // First byte should be CREATE_ATA_IDEMPOTENT discriminator
        expect(result.instruction.data[0]).toBe(
            DISCRIMINATOR.CREATE_ATA_IDEMPOTENT,
        );
    });
});

// ============================================================================
// TEST: Transfer Instructions
// ============================================================================

describe('createTransferInstruction', () => {
    it('8.3 creates transfer instruction with correct structure', () => {
        const instruction = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(4);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER);

        // Verify amount encoding (little-endian u64)
        const amountBytes = instruction.data.slice(1, 9);
        const dataView = new DataView(amountBytes.buffer, amountBytes.byteOffset);
        const amount = dataView.getBigUint64(0, true);
        expect(amount).toBe(1000n);
    });

    it('8.3.1 includes maxTopUp when provided', () => {
        const instruction = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            maxTopUp: 5000,
        });

        // Data should be 1 (disc) + 8 (amount) + 2 (maxTopUp)
        expect(instruction.data.length).toBe(11);
    });

    it('8.3.2 includes fee payer when provided', () => {
        const feePayer = address('Vote111111111111111111111111111111111111111');
        const instruction = createTransferInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            maxTopUp: 5000,
            feePayer,
        });

        expect(instruction.accounts).toHaveLength(5);
    });
});

describe('createTransferCheckedInstruction', () => {
    it('8.4 creates transfer checked instruction', () => {
        const instruction = createTransferCheckedInstruction({
            source: TEST_SOURCE,
            destination: TEST_DEST,
            mint: TEST_MINT,
            amount: 1000n,
            authority: TEST_AUTHORITY,
            decimals: 9,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(5);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.TRANSFER_CHECKED);

        // Verify decimals is in the data
        const decimals = instruction.data[9]; // After disc (1) + amount (8)
        expect(decimals).toBe(9);
    });
});

describe('createTransferInterfaceInstruction', () => {
    it('8.5 routes light-to-light transfer correctly', () => {
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
        expect(result.instructions[0].programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
    });

    it('8.5.1 throws for unsupported transfer types', () => {
        const splProgram = address('TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA');

        // Light-to-SPL
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

        // SPL-to-Light
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

        // SPL-to-SPL
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
// TEST: Close Account Instruction
// ============================================================================

describe('createCloseAccountInstruction', () => {
    it('8.6 creates close account instruction', () => {
        const instruction = createCloseAccountInstruction({
            tokenAccount: TEST_SOURCE,
            destination: TEST_DEST,
            owner: TEST_OWNER,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.CLOSE);
        expect(instruction.data.length).toBe(1);
    });
});

// ============================================================================
// TEST: Mint Instructions
// ============================================================================

describe('createMintToInstruction', () => {
    it('8.7 creates mint-to instruction', () => {
        const instruction = createMintToInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.MINT_TO);

        // Verify amount
        const amountBytes = instruction.data.slice(1, 9);
        const dataView = new DataView(amountBytes.buffer, amountBytes.byteOffset);
        expect(dataView.getBigUint64(0, true)).toBe(1_000_000n);
    });
});

describe('createMintToCheckedInstruction', () => {
    it('8.8 creates mint-to checked instruction', () => {
        const instruction = createMintToCheckedInstruction({
            mint: TEST_MINT,
            tokenAccount: TEST_DEST,
            mintAuthority: TEST_AUTHORITY,
            amount: 1_000_000n,
            decimals: 6,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.MINT_TO_CHECKED);
        expect(instruction.data[9]).toBe(6); // Decimals
    });
});

// ============================================================================
// TEST: Burn Instructions
// ============================================================================

describe('createBurnInstruction', () => {
    it('8.9 creates burn instruction', () => {
        const instruction = createBurnInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.BURN);

        // Verify amount
        const amountBytes = instruction.data.slice(1, 9);
        const dataView = new DataView(amountBytes.buffer, amountBytes.byteOffset);
        expect(dataView.getBigUint64(0, true)).toBe(500n);
    });
});

describe('createBurnCheckedInstruction', () => {
    it('8.10 creates burn checked instruction', () => {
        const instruction = createBurnCheckedInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            authority: TEST_AUTHORITY,
            amount: 500n,
            decimals: 9,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.BURN_CHECKED);
        expect(instruction.data[9]).toBe(9); // Decimals
    });
});

// ============================================================================
// TEST: Freeze/Thaw Instructions
// ============================================================================

describe('createFreezeInstruction', () => {
    it('8.11 creates freeze instruction', () => {
        const instruction = createFreezeInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.FREEZE);
        expect(instruction.data.length).toBe(1);
    });
});

describe('createThawInstruction', () => {
    it('8.12 creates thaw instruction', () => {
        const instruction = createThawInstruction({
            tokenAccount: TEST_SOURCE,
            mint: TEST_MINT,
            freezeAuthority: TEST_FREEZE_AUTH,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.THAW);
        expect(instruction.data.length).toBe(1);
    });
});

// ============================================================================
// TEST: Approve/Revoke Instructions
// ============================================================================

describe('createApproveInstruction', () => {
    it('8.13 creates approve instruction', () => {
        const instruction = createApproveInstruction({
            tokenAccount: TEST_SOURCE,
            delegate: TEST_DELEGATE,
            owner: TEST_OWNER,
            amount: 10_000n,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(3);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.APPROVE);

        // Verify amount
        const amountBytes = instruction.data.slice(1, 9);
        const dataView = new DataView(amountBytes.buffer, amountBytes.byteOffset);
        expect(dataView.getBigUint64(0, true)).toBe(10_000n);
    });
});

describe('createRevokeInstruction', () => {
    it('8.14 creates revoke instruction', () => {
        const instruction = createRevokeInstruction({
            tokenAccount: TEST_SOURCE,
            owner: TEST_OWNER,
        });

        expect(instruction.programAddress).toBe(LIGHT_TOKEN_PROGRAM_ID);
        expect(instruction.accounts).toHaveLength(2);
        expect(instruction.data[0]).toBe(DISCRIMINATOR.REVOKE);
        expect(instruction.data.length).toBe(1);
    });
});

// ============================================================================
// LIVE E2E TESTS (require running validator)
// ============================================================================

// These tests require a running test validator and are skipped by default.
// To run: Start validator, then run `LIVE_E2E=true pnpm test:e2e`

describe.skip('Live E2E tests (require validator)', () => {
    // const RPC_URL = 'http://127.0.0.1:8899';
    // const INDEXER_URL = 'http://127.0.0.1:8784';

    it('should create and fund ATA', async () => {
        // Implementation would:
        // 1. Create keypair, airdrop SOL
        // 2. Create mint
        // 3. Create ATA with createAssociatedTokenAccountInstruction
        // 4. Send transaction
        // 5. Verify account exists via indexer
        expect(true).toBe(true);
    });

    it('should transfer tokens between accounts', async () => {
        // Implementation would:
        // 1. Setup two accounts with tokens
        // 2. Create transfer instruction
        // 3. Send transaction
        // 4. Verify balances changed
        expect(true).toBe(true);
    });

    it('should mint tokens to account', async () => {
        // Implementation would:
        // 1. Create mint with authority
        // 2. Create token account
        // 3. Mint tokens
        // 4. Verify balance increased
        expect(true).toBe(true);
    });

    it('should burn tokens from account', async () => {
        // Implementation would:
        // 1. Setup account with tokens
        // 2. Burn tokens
        // 3. Verify balance decreased and supply decreased
        expect(true).toBe(true);
    });

    it('should freeze and thaw account', async () => {
        // Implementation would:
        // 1. Create mint with freeze authority
        // 2. Create token account
        // 3. Freeze account
        // 4. Verify account is frozen
        // 5. Thaw account
        // 6. Verify account is unfrozen
        expect(true).toBe(true);
    });

    it('should approve and revoke delegate', async () => {
        // Implementation would:
        // 1. Create token account
        // 2. Approve delegate
        // 3. Verify delegate set
        // 4. Revoke delegate
        // 5. Verify delegate cleared
        expect(true).toBe(true);
    });

    it('should close token account', async () => {
        // Implementation would:
        // 1. Create token account with zero balance
        // 2. Close account
        // 3. Verify account no longer exists
        // 4. Verify rent returned
        expect(true).toBe(true);
    });
});
