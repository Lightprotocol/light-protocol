import assert from 'node:assert/strict';
import { Given } from '@cucumber/cucumber';
import { Keypair } from '@solana/web3.js';
import { newAccountWithLamports } from '@lightprotocol/stateless.js';
import {
    createAtaInstructions,
    createApproveInstructions,
    getAtaAddress,
} from '../../../src/index.js';
import {
    createMintFixture,
    mintCompressedToOwner,
    sendInstructions,
} from '../../e2e/helpers.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

Given(
    'a fresh mint fixture',
    async function (this: TokenInterfaceWorld) {
        this.fixture = await createMintFixture();
    },
);

Given(
    'a fresh mint fixture with freeze authority',
    async function (this: TokenInterfaceWorld) {
        this.fixture = await createMintFixture({ withFreezeAuthority: true });
    },
);

Given(
    'a new owner',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.owner = await newAccountWithLamports(this.fixture.rpc, 1e9);
    },
);

Given(
    'a new recipient',
    function (this: TokenInterfaceWorld) {
        this.recipient = Keypair.generate();
    },
);

Given(
    'a new delegate',
    function (this: TokenInterfaceWorld) {
        this.delegate = Keypair.generate();
    },
);

Given(
    'a funded sender with {int} compressed tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.sender = await newAccountWithLamports(this.fixture.rpc, 1e9);
        await mintCompressedToOwner(
            this.fixture,
            this.sender.publicKey,
            BigInt(amount),
        );
    },
);

Given(
    'a funded recipient with SOL',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.recipient = await newAccountWithLamports(this.fixture.rpc, 1e9);
    },
);

Given(
    'a funded recipient with {int} compressed tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.recipient = await newAccountWithLamports(this.fixture.rpc, 1e9);
        await mintCompressedToOwner(
            this.fixture,
            this.recipient.publicKey,
            BigInt(amount),
        );
    },
);

Given(
    'an owner with {int} compressed tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.owner = await newAccountWithLamports(this.fixture.rpc, 1e9);
        await mintCompressedToOwner(
            this.fixture,
            this.owner.publicKey,
            BigInt(amount),
        );
    },
);

Given(
    'an owner with a created ATA and {int} compressed tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.owner = await newAccountWithLamports(this.fixture.rpc, 1e9);

        const ataIxs = await createAtaInstructions({
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });
        await sendInstructions(this.fixture.rpc, this.fixture.payer, ataIxs);

        await mintCompressedToOwner(
            this.fixture,
            this.owner.publicKey,
            BigInt(amount),
        );
    },
);

Given(
    'an owner with compressed mints of {int}, {int}, and {int} tokens',
    async function (
        this: TokenInterfaceWorld,
        amount1: number,
        amount2: number,
        amount3: number,
    ) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.owner = await newAccountWithLamports(this.fixture.rpc, 1e9);
        await mintCompressedToOwner(
            this.fixture,
            this.owner.publicKey,
            BigInt(amount1),
        );
        await mintCompressedToOwner(
            this.fixture,
            this.owner.publicKey,
            BigInt(amount2),
        );
        await mintCompressedToOwner(
            this.fixture,
            this.owner.publicKey,
            BigInt(amount3),
        );
    },
);

Given(
    'a sender with compressed mints of {int}, {int}, and {int} tokens',
    async function (
        this: TokenInterfaceWorld,
        amount1: number,
        amount2: number,
        amount3: number,
    ) {
        assert.ok(this.fixture, 'fixture must be created first');
        this.sender = await newAccountWithLamports(this.fixture.rpc, 1e9);
        await mintCompressedToOwner(
            this.fixture,
            this.sender.publicKey,
            BigInt(amount1),
        );
        await mintCompressedToOwner(
            this.fixture,
            this.sender.publicKey,
            BigInt(amount2),
        );
        await mintCompressedToOwner(
            this.fixture,
            this.sender.publicKey,
            BigInt(amount3),
        );
    },
);

Given(
    'a delegate approved for {int} tokens',
    async function (this: TokenInterfaceWorld, amount: number) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        this.delegate = await newAccountWithLamports(this.fixture.rpc, 1e9);

        const approveIxs = await createApproveInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
            delegate: this.delegate.publicKey,
            amount: BigInt(amount),
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            approveIxs,
            [this.owner],
        );
    },
);
