import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import BN from 'bn.js';
import { TOKEN_PROGRAM_ID } from '@solana/spl-token';
import { MAX_TOP_UP } from '../../src/constants';
import { createWrapInstruction } from '../../src/v3/instructions/wrap';
import { createUnwrapInstruction } from '../../src/v3/instructions/unwrap';
import { createMintToInstruction } from '../../src/v3/instructions/mint-to';
import type { SplInterfaceInfo } from '../../src/utils/get-token-pool-infos';

const TRANSFER2_BASE_MAX_TOP_UP_OFFSET = 6;

function mockSplInterfaceInfo(mint: PublicKey): SplInterfaceInfo {
    const splInterfacePda = Keypair.generate().publicKey;
    return {
        mint,
        splInterfacePda,
        tokenProgram: TOKEN_PROGRAM_ID,
        isInitialized: true,
        balance: new BN(0),
        poolIndex: 0,
        bump: 255,
    };
}

function getTransfer2MaxTopUpFromInstructionData(data: Buffer): number {
    return data.readUInt16LE(TRANSFER2_BASE_MAX_TOP_UP_OFFSET);
}

describe('instructions maxTopUp encoding', () => {
    describe('createWrapInstruction', () => {
        it('should encode maxTopUp 65535 when maxTopUp is omitted', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createWrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(65535);
        });

        it('should encode maxTopUp 0 when maxTopUp is 0', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createWrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
                owner,
                0,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(0);
        });

        it('should encode custom maxTopUp when provided', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createWrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
                owner,
                10,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(10);
        });
    });

    describe('createUnwrapInstruction', () => {
        it('should encode maxTopUp 65535 when maxTopUp is omitted', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createUnwrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(65535);
        });

        it('should encode maxTopUp 0 when maxTopUp is 0', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createUnwrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
                owner,
                0,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(0);
        });

        it('should encode custom maxTopUp when provided', () => {
            const source = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const owner = Keypair.generate().publicKey;
            const mint = Keypair.generate().publicKey;
            const info = mockSplInterfaceInfo(mint);

            const ix = createUnwrapInstruction(
                source,
                destination,
                owner,
                mint,
                1000n,
                info,
                9,
                owner,
                100,
            );

            const maxTopUp = getTransfer2MaxTopUpFromInstructionData(ix.data);
            expect(maxTopUp).toBe(100);
        });
    });

    describe('createMintToInstruction', () => {
        it('should produce 9-byte data when maxTopUp is omitted', () => {
            const mint = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const authority = Keypair.generate().publicKey;

            const ix = createMintToInstruction({
                mint,
                destination,
                amount: 100n,
                authority,
            });

            expect(ix.data.length).toBe(9);
        });

        it('should produce 11-byte data with maxTopUp 0 when maxTopUp is 0', () => {
            const mint = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const authority = Keypair.generate().publicKey;

            const ix = createMintToInstruction({
                mint,
                destination,
                amount: 100n,
                authority,
                maxTopUp: 0,
            });

            expect(ix.data.length).toBe(11);
            expect(ix.data.readUInt16LE(9)).toBe(0);
        });

        it('should produce 11-byte data with maxTopUp 65535 when maxTopUp is MAX_TOP_UP', () => {
            const mint = Keypair.generate().publicKey;
            const destination = Keypair.generate().publicKey;
            const authority = Keypair.generate().publicKey;

            const ix = createMintToInstruction({
                mint,
                destination,
                amount: 100n,
                authority,
                maxTopUp: MAX_TOP_UP,
            });

            expect(ix.data.length).toBe(11);
            expect(ix.data.readUInt16LE(9)).toBe(65535);
        });
    });
});
