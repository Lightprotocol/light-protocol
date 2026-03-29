import { describe, expect, it } from 'vitest';
import { ComputeBudgetProgram, Keypair, TransactionInstruction } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
  TOKEN_2022_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    unpackAccount,
} from '@solana/spl-token';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createApproveInstructions,
    buildTransferInstructionsNowrap,
    createAtaInstructions,
    createTransferInstructions,
    getAta,
    getAtaAddress,
} from '../../src';
import {
    createMintFixture,
    getCompressedAmounts,
    getHotBalance,
    mintCompressedToOwner,
    sendInstructions,
} from './helpers';

describe('transfer instructions', () => {
    const isSplOrT22CloseInstruction = (
        instruction: TransactionInstruction,
    ): boolean =>
        (instruction.programId.equals(TOKEN_PROGRAM_ID) ||
            instruction.programId.equals(TOKEN_2022_PROGRAM_ID)) &&
        instruction.data.length > 0 &&
        instruction.data[0] === 9;

    it('rejects transfer build for signer that is neither owner nor delegate', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const unauthorized = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);

        await expect(
            createTransferInstructions({
                rpc: fixture.rpc,
                payer: fixture.payer.publicKey,
                mint: fixture.mint,
                sourceOwner: owner.publicKey,
                authority: unauthorized.publicKey,
                recipient: recipient.publicKey,
                amount: 100n,
            }),
        ).rejects.toThrow('Signer is not the owner or a delegate of the account.');
    });

    it('builds a single-transaction transfer flow without compute budget instructions', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();

        await mintCompressedToOwner(fixture, sender.publicKey, 5_000n);

        const instructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 2_000n,
        });

        expect(instructions.length).toBeGreaterThan(0);
        expect(
            instructions.some(instruction =>
                instruction.programId.equals(ComputeBudgetProgram.programId),
            ),
        ).toBe(false);
        expect(instructions.some(isSplOrT22CloseInstruction)).toBe(false);

        await sendInstructions(fixture.rpc, fixture.payer, instructions, [
            sender,
        ]);

        const recipientAta = await getAta({
            rpc: fixture.rpc,
            owner: recipient.publicKey,
            mint: fixture.mint,
        });
        const senderAta = getAtaAddress({
            owner: sender.publicKey,
            mint: fixture.mint,
        });

        expect(recipientAta.parsed.amount).toBe(2_000n);
        expect(await getHotBalance(fixture.rpc, senderAta)).toBe(3_000n);
    });

    it('supports non-light destination path with SPL ATA recipient', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipientSplAta = getAssociatedTokenAddressSync(
            fixture.mint,
            recipient.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        await mintCompressedToOwner(fixture, sender.publicKey, 3_000n);

        const instructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            amount: 1_250n,
        });

        await sendInstructions(fixture.rpc, fixture.payer, instructions, [sender]);

        const recipientSplInfo = await fixture.rpc.getAccountInfo(recipientSplAta);
        expect(recipientSplInfo).not.toBeNull();
        const recipientSpl = unpackAccount(
            recipientSplAta,
            recipientSplInfo!,
            TOKEN_PROGRAM_ID,
        );
        expect(recipientSpl.amount).toBe(1_250n);
    });

    it('passes through on-chain insufficient-funds error for transfer', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();

        await mintCompressedToOwner(fixture, sender.publicKey, 500n);
        await mintCompressedToOwner(fixture, sender.publicKey, 300n);
        await mintCompressedToOwner(fixture, sender.publicKey, 200n);

        const instructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 600n,
        });

        await expect(
            sendInstructions(fixture.rpc, fixture.payer, instructions, [sender]),
        ).rejects.toThrow('custom program error');
    });

    it('does not pre-reject zero amount (on-chain behavior decides)', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();
        const senderAta = getAtaAddress({
            owner: sender.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, sender.publicKey, 500n);

        const instructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 0n,
        });

        await sendInstructions(fixture.rpc, fixture.payer, instructions, [sender]);
        expect(await getHotBalance(fixture.rpc, senderAta)).toBe(500n);
    });

    it('does not load the recipient compressed balance yet', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipientAtaAddress = getAtaAddress({
            owner: recipient.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, sender.publicKey, 400n);
        await mintCompressedToOwner(fixture, recipient.publicKey, 300n);

        const instructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 200n,
        });

        await sendInstructions(fixture.rpc, fixture.payer, instructions, [
            sender,
        ]);

        expect(await getHotBalance(fixture.rpc, recipientAtaAddress)).toBe(200n);
        expect(
            await getCompressedAmounts(
                fixture.rpc,
                recipient.publicKey,
                fixture.mint,
            ),
        ).toEqual([300n]);

        const recipientAta = await getAta({
            rpc: fixture.rpc,
            owner: recipient.publicKey,
            mint: fixture.mint,
        });

        expect(recipientAta.parsed.amount).toBe(500n);
        expect(recipientAta.compressedAmount).toBe(300n);
    });

    it('supports delegated payments after approval', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const delegate = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();
        const ownerAta = getAtaAddress({
            owner: owner.publicKey,
            mint: fixture.mint,
        });

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);

        const approveInstructions = await createApproveInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 300n,
        });

        await sendInstructions(fixture.rpc, fixture.payer, approveInstructions, [
            owner,
        ]);

        const transferInstructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: owner.publicKey,
            recipient: recipient.publicKey,
            amount: 250n,
            authority: delegate.publicKey,
        });

        await sendInstructions(fixture.rpc, fixture.payer, transferInstructions, [
            delegate,
        ]);

        const recipientAta = await getAta({
            rpc: fixture.rpc,
            owner: recipient.publicKey,
            mint: fixture.mint,
        });

        expect(recipientAta.parsed.amount).toBe(250n);
        expect(await getHotBalance(fixture.rpc, ownerAta)).toBe(250n);
    });

    it('rejects delegated transfer above delegated allowance', async () => {
        const fixture = await createMintFixture();
        const owner = await newAccountWithLamports(fixture.rpc, 1e9);
        const delegate = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();

        await mintCompressedToOwner(fixture, owner.publicKey, 500n);

        const approveInstructions = await createApproveInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            owner: owner.publicKey,
            mint: fixture.mint,
            delegate: delegate.publicKey,
            amount: 100n,
        });
        await sendInstructions(fixture.rpc, fixture.payer, approveInstructions, [
            owner,
        ]);

        const transferInstructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: owner.publicKey,
            recipient: recipient.publicKey,
            amount: 150n,
            authority: delegate.publicKey,
        });

        await expect(
            sendInstructions(fixture.rpc, fixture.payer, transferInstructions, [
                delegate,
            ]),
        ).rejects.toThrow('custom program error');
    });

    it('nowrap path fails when balance exists only in SPL ATA, canonical path succeeds', async () => {
        const fixture = await createMintFixture();
        const sender = await newAccountWithLamports(fixture.rpc, 1e9);
        const recipient = Keypair.generate();
        const senderSplAta = getAssociatedTokenAddressSync(
            fixture.mint,
            sender.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        await mintCompressedToOwner(fixture, sender.publicKey, 2_000n);

        // Stage funds into sender SPL ATA.
        const toSenderSplInstructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: sender.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            amount: 1_500n,
        });
        await sendInstructions(fixture.rpc, fixture.payer, toSenderSplInstructions, [
            sender,
        ]);

        const senderSplInfo = await fixture.rpc.getAccountInfo(senderSplAta);
        expect(senderSplInfo).not.toBeNull();
        const senderSpl = unpackAccount(
            senderSplAta,
            senderSplInfo!,
            TOKEN_PROGRAM_ID,
        );
        expect(senderSpl.amount).toBe(1_500n);

        // Nowrap does not wrap SPL/T22 balances, so transfer should fail.
        const nowrapInstructions = await buildTransferInstructionsNowrap({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 1_000n,
        });
        await expect(
            sendInstructions(fixture.rpc, fixture.payer, nowrapInstructions, [sender]),
        ).rejects.toThrow('custom program error');

        // Canonical transfer wraps SPL first, then succeeds.
        const canonicalInstructions = await createTransferInstructions({
            rpc: fixture.rpc,
            payer: fixture.payer.publicKey,
            mint: fixture.mint,
            sourceOwner: sender.publicKey,
            authority: sender.publicKey,
            recipient: recipient.publicKey,
            amount: 1_000n,
        });
        await sendInstructions(fixture.rpc, fixture.payer, canonicalInstructions, [
            sender,
        ]);

        const recipientAta = await getAta({
            rpc: fixture.rpc,
            owner: recipient.publicKey,
            mint: fixture.mint,
        });
        expect(recipientAta.parsed.amount).toBe(1_000n);
    });
});
