import assert from 'node:assert/strict';
import { Given, When, Then } from '@cucumber/cucumber';
import { Keypair } from '@solana/web3.js';
import { LIGHT_TOKEN_PROGRAM_ID } from '@lightprotocol/stateless.js';
import {
    createApproveInstruction,
    createAtaInstruction,
    createFreezeInstruction,
    createRevokeInstruction,
    createThawInstruction,
    createTransferCheckedInstruction,
} from '../../../src/instructions/index.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

Given(
    'random keypairs for {string}, {string}, {string}, {string}, and {string}',
    function (
        this: TokenInterfaceWorld,
        n1: string,
        n2: string,
        n3: string,
        n4: string,
        n5: string,
    ) {
        for (const name of [n1, n2, n3, n4, n5]) {
            this.keypairs[name] = Keypair.generate().publicKey;
        }
    },
);

When(
    'I build a create-ATA instruction for {string}, {string}, and {string}',
    function (
        this: TokenInterfaceWorld,
        payerKey: string,
        ownerKey: string,
        mintKey: string,
    ) {
        this.instruction = createAtaInstruction({
            payer: this.keypairs[payerKey],
            owner: this.keypairs[ownerKey],
            mint: this.keypairs[mintKey],
        });
    },
);

Then(
    'the instruction program ID is the light-token program',
    function (this: TokenInterfaceWorld) {
        assert.ok(this.instruction!.programId.equals(LIGHT_TOKEN_PROGRAM_ID));
    },
);

Then(
    'account key {int} is {string}',
    function (this: TokenInterfaceWorld, index: number, name: string) {
        assert.ok(this.instruction!.keys[index].pubkey.equals(this.keypairs[name]));
    },
);

When(
    'I build a checked transfer instruction for {int} tokens with {int} decimals',
    function (this: TokenInterfaceWorld, amount: number, decimals: number) {
        this.instruction = createTransferCheckedInstruction({
            source: this.keypairs['source'],
            destination: this.keypairs['destination'],
            mint: this.keypairs['mint'],
            authority: this.keypairs['authority'],
            payer: this.keypairs['payer'],
            amount: BigInt(amount),
            decimals,
        });
    },
);

Then(
    'the instruction discriminator byte is {int}',
    function (this: TokenInterfaceWorld, disc: number) {
        assert.strictEqual(this.instruction!.data[0], disc);
    },
);

When(
    'I build approve, revoke, freeze, and thaw instructions',
    function (this: TokenInterfaceWorld) {
        this.builtInstructions['approve'] = createApproveInstruction({
            tokenAccount: this.keypairs['tokenAccount'],
            delegate: this.keypairs['delegate'],
            owner: this.keypairs['owner'],
            amount: 10n,
        });
        this.builtInstructions['revoke'] = createRevokeInstruction({
            tokenAccount: this.keypairs['tokenAccount'],
            owner: this.keypairs['owner'],
        });
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
    'the approve instruction targets the light-token program',
    function (this: TokenInterfaceWorld) {
        assert.ok(
            this.builtInstructions['approve'].programId.equals(
                LIGHT_TOKEN_PROGRAM_ID,
            ),
        );
    },
);

Then(
    'the revoke instruction targets the light-token program',
    function (this: TokenInterfaceWorld) {
        assert.ok(
            this.builtInstructions['revoke'].programId.equals(
                LIGHT_TOKEN_PROGRAM_ID,
            ),
        );
    },
);
