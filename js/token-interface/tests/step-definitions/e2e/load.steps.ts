import assert from 'node:assert/strict';
import { When, Then } from '@cucumber/cucumber';
import {
    createLoadInstructions,
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
    'I read the owner ATA',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        this.resultAccount = await getAta({
            rpc: this.fixture.rpc,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });
    },
);

Then(
    'the ATA amount is {int}',
    function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.resultAccount, 'resultAccount must be set');
        assert.strictEqual(this.resultAccount.parsed.amount, BigInt(expected));
    },
);

Then(
    'the ATA compressed amount is {int}',
    function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.resultAccount, 'resultAccount must be set');
        assert.strictEqual(
            this.resultAccount.compressedAmount,
            BigInt(expected),
        );
    },
);

Then(
    'the ATA requires load',
    function (this: TokenInterfaceWorld) {
        assert.ok(this.resultAccount, 'resultAccount must be set');
        assert.strictEqual(this.resultAccount.requiresLoad, true);
    },
);

Then(
    'there are {int} ignored compressed accounts totaling {int}',
    function (
        this: TokenInterfaceWorld,
        count: number,
        totalIgnored: number,
    ) {
        assert.ok(this.resultAccount, 'resultAccount must be set');
        assert.strictEqual(
            this.resultAccount.ignoredCompressedAccounts.length,
            count,
        );
        assert.strictEqual(
            this.resultAccount.ignoredCompressedAmount,
            BigInt(totalIgnored),
        );
    },
);

When(
    'I load the first compressed balance',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const instructions = await createLoadInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.owner],
        );
    },
);

Then(
    'the hot balance is {int}',
    async function (this: TokenInterfaceWorld, expected: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const tokenAccount = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const balance = await getHotBalance(this.fixture.rpc, tokenAccount);
        assert.strictEqual(balance, BigInt(expected));
    },
);

Then(
    'compressed accounts are {int} and {int}',
    async function (
        this: TokenInterfaceWorld,
        amount1: number,
        amount2: number,
    ) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const amounts = await getCompressedAmounts(
            this.fixture.rpc,
            this.owner.publicKey,
            this.fixture.mint,
        );

        assert.deepStrictEqual(amounts, [BigInt(amount1), BigInt(amount2)]);
    },
);

When(
    'I load the next compressed balance',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const instructions = await createLoadInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.owner],
        );
    },
);

Then(
    'compressed accounts are {int}',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const amounts = await getCompressedAmounts(
            this.fixture.rpc,
            this.owner.publicKey,
            this.fixture.mint,
        );

        assert.deepStrictEqual(amounts, [BigInt(amount)]);
    },
);
