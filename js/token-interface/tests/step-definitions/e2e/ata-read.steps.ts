import assert from 'node:assert/strict';
import { When, Then } from '@cucumber/cucumber';
import {
    createAtaInstructions,
    getAta,
    getAtaAddress,
} from '../../../src/index.js';
import { sendInstructions } from '../../e2e/helpers.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

When(
    'the owner creates an ATA',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const instructions = await createAtaInstructions({
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
        );
    },
);

Then(
    'reading the ATA returns the correct address, owner, mint, and zero balance',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const expectedAddress = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const account = await getAta({
            rpc: this.fixture.rpc,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        assert.strictEqual(
            account.parsed.address.toBase58(),
            expectedAddress.toBase58(),
        );
        assert.strictEqual(
            account.parsed.owner.toBase58(),
            this.owner.publicKey.toBase58(),
        );
        assert.strictEqual(
            account.parsed.mint.toBase58(),
            this.fixture.mint.toBase58(),
        );
        assert.strictEqual(account.parsed.amount, BigInt(0));
    },
);
