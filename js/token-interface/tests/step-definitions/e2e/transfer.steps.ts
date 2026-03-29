import assert from 'node:assert/strict';
import { When, Then } from '@cucumber/cucumber';
import { Keypair } from '@solana/web3.js';
import {
    TOKEN_PROGRAM_ID,
    getAssociatedTokenAddressSync,
    unpackAccount,
} from '@solana/spl-token';
import {
    createTransferInstructions,
    getAta,
    getAtaAddress,
} from '../../../src/index.js';
import {
    getCompressedAmounts,
    getHotBalance,
    sendInstructions,
} from '../../e2e/helpers.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

When(
    'the sender transfers {int} tokens to the recipient',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.sender, 'sender must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const instructions = await createTransferInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            mint: this.fixture.mint,
            sourceOwner: this.sender.publicKey,
            authority: this.sender.publicKey,
            recipient: this.recipient.publicKey,
            amount: BigInt(amount),
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.sender],
        );
    },
);

Then(
    'the recipient ATA balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const account = await getAta({
            rpc: this.fixture.rpc,
            owner: this.recipient.publicKey,
            mint: this.fixture.mint,
        });

        assert.strictEqual(account.parsed.amount, BigInt(expected));
    },
);

Then(
    'the sender ATA balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.sender, 'sender must be created first');

        const senderAta = getAtaAddress({
            owner: this.sender.publicKey,
            mint: this.fixture.mint,
        });

        const balance = await getHotBalance(this.fixture.rpc, senderAta);
        assert.strictEqual(balance, BigInt(expected));
    },
);

When(
    'the sender transfers {int} tokens to the recipient SPL ATA',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.sender, 'sender must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const instructions = await createTransferInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            mint: this.fixture.mint,
            sourceOwner: this.sender.publicKey,
            authority: this.sender.publicKey,
            recipient: this.recipient.publicKey,
            tokenProgram: TOKEN_PROGRAM_ID,
            amount: BigInt(amount),
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.sender],
        );
    },
);

Then(
    'the recipient SPL ATA balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const recipientSplAta = getAssociatedTokenAddressSync(
            this.fixture.mint,
            this.recipient.publicKey,
            false,
            TOKEN_PROGRAM_ID,
        );

        const recipientSplInfo = await this.fixture.rpc.getAccountInfo(
            recipientSplAta,
        );
        assert.ok(recipientSplInfo, 'SPL ATA account should exist');

        const recipientSpl = unpackAccount(
            recipientSplAta,
            recipientSplInfo,
            TOKEN_PROGRAM_ID,
        );
        assert.strictEqual(recipientSpl.amount, BigInt(expected));
    },
);

When(
    'the sender attempts to transfer {int} tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.sender, 'sender must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        try {
            const instructions = await createTransferInstructions({
                rpc: this.fixture.rpc,
                payer: this.fixture.payer.publicKey,
                mint: this.fixture.mint,
                sourceOwner: this.sender.publicKey,
                authority: this.sender.publicKey,
                recipient: this.recipient.publicKey,
                amount: BigInt(amount),
            });

            await sendInstructions(
                this.fixture.rpc,
                this.fixture.payer,
                instructions,
                [this.sender],
            );
        } catch (error) {
            this.resultError = error as Error;
        }
    },
);

Then(
    'the transaction fails with {string}',
    function (this: TokenInterfaceWorld, expectedMessage: string) {
        assert.ok(this.resultError, 'an error should have been thrown');
        assert.ok(
            this.resultError.message.includes(expectedMessage),
            `expected error message to contain "${expectedMessage}", got: "${this.resultError.message}"`,
        );
    },
);

Then(
    'the recipient hot balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const recipientAtaAddress = getAtaAddress({
            owner: this.recipient.publicKey,
            mint: this.fixture.mint,
        });

        const balance = await getHotBalance(
            this.fixture.rpc,
            recipientAtaAddress,
        );
        assert.strictEqual(balance, BigInt(expected));
    },
);

Then(
    'the recipient still has {int} in compressed accounts',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const amounts = await getCompressedAmounts(
            this.fixture.rpc,
            this.recipient.publicKey,
            this.fixture.mint,
        );

        assert.deepStrictEqual(amounts, [BigInt(expected)]);
    },
);

Then(
    'the recipient total ATA amount is {int} with {int} compressed',
    async function (
        this: TokenInterfaceWorld,
        totalAmount: number,
        compressedAmount: number,
    ) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.recipient, 'recipient must be created first');

        const recipientAta = await getAta({
            rpc: this.fixture.rpc,
            owner: this.recipient.publicKey,
            mint: this.fixture.mint,
        });

        assert.strictEqual(recipientAta.parsed.amount, BigInt(totalAmount));
        assert.strictEqual(
            recipientAta.compressedAmount,
            BigInt(compressedAmount),
        );
    },
);

When(
    'the delegate transfers {int} tokens to a new recipient',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        assert.ok(this.delegate, 'delegate must be created first');

        this.recipient = Keypair.generate();

        const instructions = await createTransferInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            mint: this.fixture.mint,
            sourceOwner: this.owner.publicKey,
            authority: this.delegate.publicKey,
            recipient: this.recipient.publicKey,
            amount: BigInt(amount),
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.delegate],
        );
    },
);

Then(
    'the owner ATA balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const ownerAta = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const balance = await getHotBalance(this.fixture.rpc, ownerAta);
        assert.strictEqual(balance, BigInt(expected));
    },
);
