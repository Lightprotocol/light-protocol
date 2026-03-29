import assert from 'node:assert/strict';
import { When, Then } from '@cucumber/cucumber';
import { AccountState } from '@solana/spl-token';
import {
    createFreezeInstructions,
    createThawInstructions,
    getAtaAddress,
} from '../../../src/index.js';
import { getHotState, sendInstructions } from '../../e2e/helpers.js';
import type { TokenInterfaceWorld } from '../../support/world.js';

When(
    'the freeze authority freezes the account',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        assert.ok(
            this.fixture.freezeAuthority,
            'freeze authority must exist on fixture',
        );

        const instructions = await createFreezeInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
            freezeAuthority: this.fixture.freezeAuthority.publicKey,
        });

        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.owner, this.fixture.freezeAuthority],
        );
    },
);

Then(
    'the account state is {string}',
    async function (this: TokenInterfaceWorld, expectedState: string) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');

        const tokenAccount = getAtaAddress({
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
        });

        const state = await getHotState(this.fixture.rpc, tokenAccount);

        const stateMap: Record<string, AccountState> = {
            Frozen: AccountState.Frozen,
            Initialized: AccountState.Initialized,
            Uninitialized: AccountState.Uninitialized,
        };

        assert.strictEqual(state, stateMap[expectedState]);
    },
);

When(
    'the freeze authority thaws the account',
    async function (this: TokenInterfaceWorld) {
        assert.ok(this.fixture, 'fixture must be created first');
        assert.ok(this.owner, 'owner must be created first');
        assert.ok(
            this.fixture.freezeAuthority,
            'freeze authority must exist on fixture',
        );

        const instructions = await createThawInstructions({
            rpc: this.fixture.rpc,
            payer: this.fixture.payer.publicKey,
            owner: this.owner.publicKey,
            mint: this.fixture.mint,
            freezeAuthority: this.fixture.freezeAuthority.publicKey,
        });

        // Thaw only needs freezeAuthority as signer. The account is frozen
        // so createLoadInstructions inside createThawInstructions produces
        // no load instructions (nothing to load into a frozen account).
        await sendInstructions(
            this.fixture.rpc,
            this.fixture.payer,
            instructions,
            [this.fixture.freezeAuthority],
        );
    },
);
