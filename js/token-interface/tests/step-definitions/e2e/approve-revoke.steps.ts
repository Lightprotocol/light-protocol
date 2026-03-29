import assert from 'node:assert/strict';
import { When, Then } from '@cucumber/cucumber';
import { ComputeBudgetProgram } from '@solana/web3.js';
import {
    createApproveInstructions,
    createRevokeInstructions,
    getAtaAddress,
} from '../../../src/index.js';
import { getHotDelegate, sendInstructions } from '../../e2e/helpers.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

When(
    'the owner approves the delegate for {int} tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        assert.ok(this.delegate, 'delegate must be created first');

        const instructions = await createApproveInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
            delegate: this.delegate.publicKey,
            amount: BigInt(amount),
        });

        this.lastApproveInstructions = instructions;

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.owner],
        );
    },
);

Then(
    'the delegate is set on the token account with amount {int}',
    async function (this: TokenInterfaceWorld, expectedAmount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        assert.ok(this.delegate, 'delegate must be created first');

        const tokenAccount = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const delegateInfo = await getHotDelegate(
            this.fixture.rpc,
            tokenAccount,
        );

        assert.strictEqual(
            delegateInfo.delegate?.toBase58(),
            this.delegate.publicKey.toBase58(),
        );
        assert.strictEqual(delegateInfo.delegatedAmount, BigInt(expectedAmount));
    },
);

Then(
    'no compute budget instructions were included',
    function (this: TokenInterfaceWorld) {
        const hasComputeBudget = this.lastApproveInstructions.some((ix) =>
            ix.programId.equals(ComputeBudgetProgram.programId),
        );
        assert.strictEqual(hasComputeBudget, false);
    },
);

When(
    'the owner revokes the delegation',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const instructions = await createRevokeInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        this.lastRevokeInstructions = instructions;

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.owner],
        );
    },
);

Then(
    'the delegate is cleared and delegated amount is 0',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const tokenAccount = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const delegateInfo = await getHotDelegate(
            this.fixture.rpc,
            tokenAccount,
        );

        assert.strictEqual(delegateInfo.delegate, null);
        assert.strictEqual(delegateInfo.delegatedAmount, BigInt(0));
    },
);

Then(
    'no compute budget instructions were included in revoke',
    function (this: TokenInterfaceWorld) {
        const hasComputeBudget = this.lastRevokeInstructions.some((ix) =>
            ix.programId.equals(ComputeBudgetProgram.programId),
        );
        assert.strictEqual(hasComputeBudget, false);
    },
);
