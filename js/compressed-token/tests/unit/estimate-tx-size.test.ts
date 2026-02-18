import { describe, it, expect } from 'vitest';
import {
    Keypair,
    PublicKey,
    SystemProgram,
    ComputeBudgetProgram,
    TransactionInstruction,
    TransactionMessage,
    VersionedTransaction,
} from '@solana/web3.js';
import {
    estimateTransactionSize,
    MAX_TRANSACTION_SIZE,
} from '../../src/v3/utils/estimate-tx-size';

/**
 * Build an actual unsigned VersionedTransaction and return its serialized
 * byte length.  Used to cross-check the estimate.
 */
function actualTxSize(
    instructions: TransactionInstruction[],
    payer: PublicKey,
    numSigners: number,
): number {
    const dummyBlockhash = 'GWsqNcmNBbBigUdeFbMGjEiWRpWwR9bXZFaygD7RnPb8';
    const messageV0 = new TransactionMessage({
        payerKey: payer,
        recentBlockhash: dummyBlockhash,
        instructions,
    }).compileToV0Message();

    const tx = new VersionedTransaction(messageV0);
    // Unsigned tx has placeholder signatures (all zeros) for each signer
    // The serialized length includes those.
    return tx.serialize().length;
}

/** Helper: create a simple instruction with N keys and D bytes of data. */
function makeIx(
    programId: PublicKey,
    keys: { pubkey: PublicKey; isSigner: boolean; isWritable: boolean }[],
    dataLength: number,
): TransactionInstruction {
    return new TransactionInstruction({
        programId,
        keys,
        data: Buffer.alloc(dataLength),
    });
}

describe('estimateTransactionSize', () => {
    const payer = Keypair.generate().publicKey;

    it('estimates base size for empty instructions', () => {
        const estimate = estimateTransactionSize([], 1);
        // Should be: signatures(1 + 64) + prefix(1) + header(3) +
        // keys(1 + 0) + blockhash(32) + instructions(1) + lookups(1) = 104
        // But with the payer, there's at least 1 key... actually no:
        // estimateTransactionSize doesn't inject payer, only counts keys from ixs.
        // With 0 instructions and 0 keys:
        // sigs: 1 + 64 = 65
        // msg: 1 + 3 + 1 + 32 + 1 + 1 = 39
        // total = 104
        expect(estimate).toBe(104);
    });

    it('estimates correctly for a single simple instruction', () => {
        const programId = Keypair.generate().publicKey;
        const ix = makeIx(
            programId,
            [{ pubkey: payer, isSigner: true, isWritable: true }],
            9,
        );

        const estimate = estimateTransactionSize([ix], 1);
        // keys: programId + payer = 2 unique
        // sigs: 1 + 64 = 65
        // msg: 1 + 3 + (1 + 64) + 32 + (1 + 1 + 1 + 1 + 1 + 9) + 1 = 181
        // Breakdown:
        //   prefix=1, header=3, keys=1+2*32=65, blockhash=32
        //   ixs: count(1) + programIdIdx(1) + keysCount(1) + 1 key idx(1) + dataLen(1) + data(9) = 14
        //   lookups=1
        // total: 65 + 1 + 3 + 65 + 32 + 14 + 1 = 181
        expect(estimate).toBe(181);
    });

    it('deduplicates shared keys across instructions', () => {
        const programId = Keypair.generate().publicKey;
        const sharedKey = Keypair.generate().publicKey;

        const ix1 = makeIx(
            programId,
            [{ pubkey: sharedKey, isSigner: false, isWritable: true }],
            4,
        );
        const ix2 = makeIx(
            programId,
            [
                { pubkey: sharedKey, isSigner: false, isWritable: true },
                { pubkey: payer, isSigner: true, isWritable: true },
            ],
            8,
        );

        const estimate = estimateTransactionSize([ix1, ix2], 1);
        // Unique keys: programId, sharedKey, payer = 3
        // sigs: 65
        // msg: 1 + 3 + (1+96) + 32 + (1 + 8 + 13) + 1 = 156
        //   ix1: progIdx(1) + keysLen(1) + 1 idx(1) + dataLen(1) + data(4) = 8
        //   ix2: progIdx(1) + keysLen(1) + 2 idx(2) + dataLen(1) + data(8) = 13
        //   ixs total: 1(count) + 8 + 13 = 22
        // total: 65 + 1 + 3 + 97 + 32 + 22 + 1 = 221
        expect(estimate).toBe(221);
    });

    it('estimate matches actual serialized size for CU + transfer instruction', () => {
        const owner = Keypair.generate().publicKey;
        const source = Keypair.generate().publicKey;
        const dest = Keypair.generate().publicKey;
        const lightProgram = Keypair.generate().publicKey;

        const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
            units: 200_000,
        });
        const transferIx = makeIx(
            lightProgram,
            [
                { pubkey: source, isSigner: false, isWritable: true },
                { pubkey: dest, isSigner: false, isWritable: true },
                { pubkey: owner, isSigner: true, isWritable: true },
                {
                    pubkey: SystemProgram.programId,
                    isSigner: false,
                    isWritable: false,
                },
                { pubkey: owner, isSigner: true, isWritable: true },
            ],
            9,
        );

        const instructions = [cuIx, transferIx];
        const estimate = estimateTransactionSize(instructions, 1);
        const actual = actualTxSize(instructions, owner, 1);

        // Estimate should be very close to actual (within a few bytes)
        expect(Math.abs(estimate - actual)).toBeLessThanOrEqual(5);
        expect(estimate).toBeLessThan(MAX_TRANSACTION_SIZE);
    });

    it('estimate is deterministic for a complex multi-instruction batch', () => {
        // Simulate a decompress-like instruction with many keys and data.
        // We only test the estimate (not actualTxSize) because a tx this
        // large exceeds Solana's serialization buffer in @solana/web3.js.
        const owner = Keypair.generate().publicKey;
        const programId = Keypair.generate().publicKey;

        const keys = Array.from({ length: 12 }, () => ({
            pubkey: Keypair.generate().publicKey,
            isSigner: false,
            isWritable: true,
        }));
        keys[1] = { pubkey: owner, isSigner: true, isWritable: true };

        const decompressIx = makeIx(programId, keys, 360);
        const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
            units: 500_000,
        });

        const transferIx = makeIx(
            Keypair.generate().publicKey,
            [
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: true,
                },
                { pubkey: owner, isSigner: true, isWritable: true },
                {
                    pubkey: SystemProgram.programId,
                    isSigner: false,
                    isWritable: false,
                },
            ],
            9,
        );
        const ataIx = makeIx(
            Keypair.generate().publicKey,
            Array.from({ length: 7 }, () => ({
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            })),
            35,
        );

        const instructions = [cuIx, ataIx, decompressIx, transferIx];
        const est1 = estimateTransactionSize(instructions, 2);
        const est2 = estimateTransactionSize(instructions, 2);

        // Deterministic
        expect(est1).toBe(est2);
        // Above MAX_TRANSACTION_SIZE (this combined batch is too big)
        expect(est1).toBeGreaterThan(MAX_TRANSACTION_SIZE);
    });

    it('two decompress (8+4) + transfer + ATA + CU exceeds MAX_TRANSACTION_SIZE', () => {
        const owner = Keypair.generate().publicKey;

        // First decompress (8 inputs): ~360 bytes data, 12 keys
        const decompress1Keys = Array.from({ length: 12 }, () => ({
            pubkey: Keypair.generate().publicKey,
            isSigner: false,
            isWritable: true,
        }));
        const decompress1 = makeIx(
            Keypair.generate().publicKey,
            decompress1Keys,
            360,
        );

        // Second decompress (4 inputs): ~260 bytes data, same program but some new keys
        const decompress2Keys = [
            ...decompress1Keys.slice(0, 5), // share some keys
            ...Array.from({ length: 7 }, () => ({
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: true,
            })),
        ];
        const decompress2 = makeIx(decompress1.programId, decompress2Keys, 260);

        const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
            units: 600_000,
        });
        const ataIx = makeIx(
            Keypair.generate().publicKey,
            Array.from({ length: 7 }, () => ({
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            })),
            35,
        );
        const transferIx = makeIx(
            Keypair.generate().publicKey,
            [
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: true,
                },
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: true,
                },
                { pubkey: owner, isSigner: true, isWritable: true },
            ],
            9,
        );

        const instructions = [
            cuIx,
            ataIx,
            decompress1,
            decompress2,
            transferIx,
        ];
        const estimate = estimateTransactionSize(instructions, 2);

        // Two decompress instructions should push this over the limit
        expect(estimate).toBeGreaterThan(MAX_TRANSACTION_SIZE);
    });

    it('single decompress (8 inputs) + transfer + ATA + CU fits in MAX_TRANSACTION_SIZE', () => {
        const owner = Keypair.generate().publicKey;

        // Decompress with 8 inputs, realistic key sharing
        const mint = Keypair.generate().publicKey;
        const tree = Keypair.generate().publicKey;
        const queue = Keypair.generate().publicKey;
        const decompressProgram = Keypair.generate().publicKey;

        const decompressKeys = [
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            }, // light_system_program
            { pubkey: owner, isSigner: true, isWritable: true }, // fee_payer
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            }, // cpi_authority_pda
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            }, // registered_program_pda
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            }, // account_compression_authority
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: false,
            }, // account_compression_program
            {
                pubkey: SystemProgram.programId,
                isSigner: false,
                isWritable: false,
            }, // system_program
            { pubkey: tree, isSigner: false, isWritable: true }, // state tree
            { pubkey: queue, isSigner: false, isWritable: true }, // output queue
            { pubkey: mint, isSigner: false, isWritable: false }, // mint
            { pubkey: owner, isSigner: true, isWritable: true }, // owner
            {
                pubkey: Keypair.generate().publicKey,
                isSigner: false,
                isWritable: true,
            }, // destination ATA
        ];
        const decompressIx = makeIx(decompressProgram, decompressKeys, 360);

        const cuIx = ComputeBudgetProgram.setComputeUnitLimit({
            units: 500_000,
        });
        const transferProgram = Keypair.generate().publicKey;
        const senderAta = Keypair.generate().publicKey;
        const recipientAta = Keypair.generate().publicKey;
        const transferIx = makeIx(
            transferProgram,
            [
                { pubkey: senderAta, isSigner: false, isWritable: true },
                { pubkey: recipientAta, isSigner: false, isWritable: true },
                { pubkey: owner, isSigner: true, isWritable: true },
                {
                    pubkey: SystemProgram.programId,
                    isSigner: false,
                    isWritable: false,
                },
                { pubkey: owner, isSigner: true, isWritable: true },
            ],
            9,
        );
        const ataProgram = Keypair.generate().publicKey;
        const ataIx = makeIx(
            ataProgram,
            [
                { pubkey: owner, isSigner: false, isWritable: false },
                { pubkey: mint, isSigner: false, isWritable: false },
                { pubkey: owner, isSigner: true, isWritable: true },
                { pubkey: recipientAta, isSigner: false, isWritable: true },
                {
                    pubkey: SystemProgram.programId,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: false,
                },
                {
                    pubkey: Keypair.generate().publicKey,
                    isSigner: false,
                    isWritable: true,
                },
            ],
            35,
        );

        const instructions = [cuIx, ataIx, decompressIx, transferIx];
        const estimate = estimateTransactionSize(instructions, 2);

        expect(estimate).toBeLessThanOrEqual(MAX_TRANSACTION_SIZE);
    });

    it('handles 2 signers correctly', () => {
        const ix = makeIx(
            Keypair.generate().publicKey,
            [{ pubkey: payer, isSigner: true, isWritable: true }],
            4,
        );
        const est1 = estimateTransactionSize([ix], 1);
        const est2 = estimateTransactionSize([ix], 2);
        // 2 signers = 64 more bytes
        expect(est2 - est1).toBe(64);
    });
});
