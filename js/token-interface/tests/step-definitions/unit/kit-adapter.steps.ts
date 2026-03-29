import assert from 'node:assert/strict';
import { Given, When, Then } from '@cucumber/cucumber';
import { Keypair } from '@solana/web3.js';
import { createAtaInstruction } from '../../../src/instructions/index.js';
import {
    createTransferInstructions,
    createAtaInstructions,
    createTransferInstructionPlan,
    toKitInstructions,
} from '../../../src/kit/index.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

Given(
    'a legacy create-ATA instruction',
    function (this: TokenInterfaceWorld) {
        this.instruction = createAtaInstruction({
            payer: Keypair.generate().publicKey,
            owner: Keypair.generate().publicKey,
            mint: Keypair.generate().publicKey,
        });
    },
);

When(
    'I convert it to kit instructions',
    function (this: TokenInterfaceWorld) {
        this.kitInstructions = toKitInstructions([this.instruction!]);
    },
);

Then(
    'the result is a list of {int} kit instruction object(s)',
    function (this: TokenInterfaceWorld, count: number) {
        assert.strictEqual(this.kitInstructions!.length, count);
        assert.ok(this.kitInstructions![0] !== undefined);
        assert.strictEqual(typeof this.kitInstructions![0], 'object');
    },
);

When(
    'I call the kit createAtaInstructions builder',
    async function (this: TokenInterfaceWorld) {
        this.kitInstructions = await createAtaInstructions({
            payer: this.keypairs['payer'],
            owner: this.keypairs['owner'],
            mint: this.keypairs['mint'],
        });
    },
);

Then(
    'the result is a list of {int} kit instruction(s)',
    function (this: TokenInterfaceWorld, count: number) {
        assert.strictEqual(this.kitInstructions!.length, count);
        assert.ok(this.kitInstructions![0] !== undefined);
    },
);

Then(
    'createTransferInstructions from kit is a function',
    function () {
        assert.strictEqual(typeof createTransferInstructions, 'function');
    },
);

Then(
    'createTransferInstructionPlan from kit is a function',
    function () {
        assert.strictEqual(typeof createTransferInstructionPlan, 'function');
    },
);
