import assert from 'node:assert/strict';
import { Given, When, Then } from '@cucumber/cucumber';
import { Keypair } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import { getAssociatedTokenAddress } from '../../../src/read/index.js';
import {
    createTransferInstructions,
    MultiTransactionNotSupportedError,
    createAtaInstructions,
    createFreezeInstruction,
    createThawInstruction,
    getAtaAddress,
} from '../../../src/index.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

Given(
    'random keypairs for {string} and {string}',
    function (this: TokenInterfaceWorld, name1: string, name2: string) {
        this.keypairs[name1] = Keypair.generate().publicKey;
        this.keypairs[name2] = Keypair.generate().publicKey;
    },
);

Given(
    'random keypairs for {string}, {string}, and {string}',
    function (
        this: TokenInterfaceWorld,
        name1: string,
        name2: string,
        name3: string,
    ) {
        this.keypairs[name1] = Keypair.generate().publicKey;
        this.keypairs[name2] = Keypair.generate().publicKey;
        this.keypairs[name3] = Keypair.generate().publicKey;
    },
);

When(
    'I derive the ATA address for {string} and {string}',
    function (this: TokenInterfaceWorld, ownerKey: string, mintKey: string) {
        const derived = getAtaAddress({
            owner: this.keypairs[ownerKey],
            mint: this.keypairs[mintKey],
        });
        this.keypairs['derivedAta'] = derived;
    },
);

Then(
    'it matches the low-level getAssociatedTokenAddress result',
    function (this: TokenInterfaceWorld) {
        const expected = getAssociatedTokenAddress(
            this.keypairs['mint'],
            this.keypairs['owner'],
        );
        assert.ok(this.keypairs['derivedAta'].equals(expected));
    },
);

When(
    'I build an ATA instruction list for {string}, {string}, and {string}',
    async function (
        this: TokenInterfaceWorld,
        payerKey: string,
        ownerKey: string,
        mintKey: string,
    ) {
        this.instructions = await createAtaInstructions({
            payer: this.keypairs[payerKey],
            owner: this.keypairs[ownerKey],
            mint: this.keypairs[mintKey],
        });
    },
);

Then(
    'the result is a list of {int} instruction(s)',
    function (this: TokenInterfaceWorld, count: number) {
        assert.strictEqual(this.instructions.length, count);
    },
);

Then(
    'the first instruction program ID is the light-token program',
    function (this: TokenInterfaceWorld) {
        assert.ok(this.instructions[0].programId.equals(LIGHT_TOKEN_PROGRAM_ID));
    },
);

When(
    'I build raw freeze and thaw instructions',
    function (this: TokenInterfaceWorld) {
        this.builtInstructions['freeze'] = createFreezeInstruction({
            tokenAccount: this.keypairs['tokenAccount'],
            mint: this.keypairs['mint'],
            freezeAuthority: this.keypairs['freezeAuthority'],
        });
        this.builtInstructions['thaw'] = createThawInstruction({
            tokenAccount: this.keypairs['tokenAccount'],
            mint: this.keypairs['mint'],
            freezeAuthority: this.keypairs['freezeAuthority'],
        });
    },
);

Then(
    'the freeze discriminator byte is {int}',
    function (this: TokenInterfaceWorld, disc: number) {
        assert.strictEqual(this.builtInstructions['freeze'].data[0], disc);
    },
);

Then(
    'the thaw discriminator byte is {int}',
    function (this: TokenInterfaceWorld, disc: number) {
        assert.strictEqual(this.builtInstructions['thaw'].data[0], disc);
    },
);

When(
    'I create a MultiTransactionNotSupportedError for {string} with batch count {int}',
    function (this: TokenInterfaceWorld, funcName: string, count: number) {
        this.errorInstance = new MultiTransactionNotSupportedError(
            funcName,
            count,
        );
    },
);

Then(
    'the error name is {string}',
    function (this: TokenInterfaceWorld, name: string) {
        assert.strictEqual(this.errorInstance!.name, name);
    },
);

Then(
    'the error message contains {string}',
    function (this: TokenInterfaceWorld, substring: string) {
        assert.ok(this.errorInstance!.message.includes(substring));
    },
);

Then('createTransferInstructions is a function', function () {
    assert.strictEqual(typeof createTransferInstructions, 'function');
});
