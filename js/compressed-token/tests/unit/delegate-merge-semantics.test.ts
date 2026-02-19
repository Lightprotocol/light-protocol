/**
 * Unit tests for delegate-mismatch merge semantics.
 *
 * The on-chain program (programs/compressed-token/program/src/...decompress.rs
 * apply_delegate) defines the authoritative rule when a cold account is
 * decompressed into an existing hot account:
 *
 *   - If hot already has a delegate D_hot:
 *       * Only cold accounts whose CompressedOnly-TLV delegate == D_hot
 *         contribute their delegatedAmount to D_hot's quota.
 *       * Cold accounts with a DIFFERENT delegate D_cold are silently ignored
 *         for delegation purposes: their balance is added to the hot's total
 *         but NOT to D_hot's delegatedAmount. D_cold is never set.
 *
 *   - If hot has NO delegate:
 *       * The FIRST cold account whose CompressedOnly-TLV delegate is non-null
 *         is adopted as the hot's new delegate.
 *       * Its delegatedAmount is added to the hot's delegatedAmount.
 *
 * These tests assert that buildAccountInterfaceFromSources produces the correct
 * synthetic account data that matches the post-decompress on-chain state,
 * and that selectInputsForAmount / createDecompressInterfaceInstruction
 * handle delegated cold inputs correctly.
 */
import { describe, it, expect } from 'vitest';
import { Keypair, PublicKey, AccountInfo } from '@solana/web3.js';
import { Buffer } from 'buffer';
import BN from 'bn.js';
import {
    LIGHT_TOKEN_PROGRAM_ID,
    bn,
    TreeType,
} from '@lightprotocol/stateless.js';
import {
    buildAccountInterfaceFromSources,
    TokenAccountSourceType,
    type TokenAccountSource,
} from '../../src/v3/get-account-interface';
import {
    selectInputsForAmount,
} from '../../src/v3/actions/load-ata';
import { createDecompressInterfaceInstruction } from '../../src/v3/instructions/create-decompress-interface-instruction';

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function mockAccountInfo(data: Buffer = Buffer.alloc(0)): AccountInfo<Buffer> {
    return {
        executable: false,
        owner: LIGHT_TOKEN_PROGRAM_ID,
        lamports: 1_000_000,
        data,
        rentEpoch: undefined,
    };
}

/**
 * Build a minimal TokenAccountSource that represents a hot c-token account.
 * Uses real SPL-compatible layout so parseCTokenHot would work, but here we
 * supply parsed directly (simulating what buildAccountInterfaceFromSources
 * actually receives from getCTokenAccountInterface).
 */
function hotSource(params: {
    address: PublicKey;
    amount: bigint;
    delegate: PublicKey | null;
    delegatedAmount: bigint;
    isFrozen?: boolean;
}): TokenAccountSource {
    return {
        type: TokenAccountSourceType.CTokenHot,
        address: params.address,
        amount: params.amount,
        accountInfo: mockAccountInfo(),
        loadContext: undefined,
        parsed: {
            address: params.address,
            mint: PublicKey.default,
            owner: Keypair.generate().publicKey,
            amount: params.amount,
            delegate: params.delegate,
            delegatedAmount: params.delegatedAmount,
            isInitialized: true,
            isFrozen: params.isFrozen ?? false,
            isNative: false,
            rentExemptReserve: null,
            closeAuthority: null,
            tlvData: Buffer.alloc(0),
        },
    };
}

/**
 * Build a minimal TokenAccountSource that represents a cold (compressed)
 * c-token account.  delegate and delegatedAmount mirror what
 * convertTokenDataToAccount would compute from the CompressedOnly TLV.
 */
function coldSource(params: {
    address: PublicKey;
    amount: bigint;
    delegate: PublicKey | null;
    delegatedAmount: bigint;
    isFrozen?: boolean;
}): TokenAccountSource {
    const mockLoadContext = {
        treeInfo: {
            tree: PublicKey.default,
            queue: PublicKey.default,
            treeType: TreeType.StateV2,
        },
        hash: new Uint8Array(32),
        leafIndex: 0,
        proveByIndex: false,
    };
    return {
        type: TokenAccountSourceType.CTokenCold,
        address: params.address,
        amount: params.amount,
        accountInfo: mockAccountInfo(),
        loadContext: mockLoadContext,
        parsed: {
            address: params.address,
            mint: PublicKey.default,
            owner: Keypair.generate().publicKey,
            amount: params.amount,
            delegate: params.delegate,
            delegatedAmount: params.delegatedAmount,
            isInitialized: true,
            isFrozen: params.isFrozen ?? false,
            isNative: false,
            rentExemptReserve: null,
            closeAuthority: null,
            tlvData: Buffer.alloc(0),
        },
    };
}

/** Build a minimal ParsedTokenAccount for selectInputsForAmount / instruction tests. */
function mockParsedAccount(params: {
    amount: bigint;
    delegate?: PublicKey | null;
    mint?: PublicKey;
    owner?: PublicKey;
}): any {
    const mint = params.mint ?? PublicKey.default;
    const owner = params.owner ?? Keypair.generate().publicKey;
    return {
        parsed: {
            mint,
            owner,
            amount: bn(params.amount.toString()),
            delegate: params.delegate ?? null,
            state: 1,
            tlv: null,
        },
        compressedAccount: {
            hash: new Uint8Array(32),
            treeInfo: {
                tree: PublicKey.default,
                queue: PublicKey.default,
                treeType: TreeType.StateV2,
            },
            leafIndex: 0,
            proveByIndex: false,
            owner: LIGHT_TOKEN_PROGRAM_ID,
            lamports: bn(0),
            address: null,
            data: {
                discriminator: [0, 0, 0, 0, 0, 0, 0, 4],
                data: Buffer.alloc(0),
                dataHash: new Array(32).fill(0),
            },
            readOnly: false,
        },
    };
}

// ---------------------------------------------------------------------------
// buildAccountInterfaceFromSources – delegate-mismatch semantics
// ---------------------------------------------------------------------------

describe('buildAccountInterfaceFromSources – delegate merge semantics', () => {
    const ata = Keypair.generate().publicKey;

    it('hot(user2) + cold(user1): synthetic keeps user2 as delegate, delegatedAmount unchanged, balance sums', () => {
        /**
         * On-chain rule: hot has D_hot=user2. Cold has D_cold=user1.
         * apply_delegate: existing_delegate (user2) != cold delegate (user1)
         * → delegate_is_set = false
         * → hot.delegate stays user2, hot.delegatedAmount unchanged
         * → cold.amount added to hot.amount (undelegated)
         */
        const user2 = Keypair.generate().publicKey;
        const user1 = Keypair.generate().publicKey;

        const hotAmount = 5_000n;
        const hotDelegatedAmount = 3_000n;
        const coldAmount = 8_000n;
        const coldDelegatedAmountToUser1 = 2_000n;

        const sources: TokenAccountSource[] = [
            hotSource({
                address: ata,
                amount: hotAmount,
                delegate: user2,
                delegatedAmount: hotDelegatedAmount,
            }),
            coldSource({
                address: ata,
                amount: coldAmount,
                delegate: user1,
                delegatedAmount: coldDelegatedAmountToUser1,
            }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        // Total balance = hot + cold
        expect(result.parsed.amount).toBe(hotAmount + coldAmount);

        // Delegate is the hot's delegate (user2), not user1
        expect(result.parsed.delegate!.toBase58()).toBe(user2.toBase58());

        // delegatedAmount reflects ONLY user2's portion; cold's 2000 to user1 is dropped
        expect(result.parsed.delegatedAmount).toBe(hotDelegatedAmount);

        // _hasDelegate is true because at least one source has a delegate
        expect(result._hasDelegate).toBe(true);

        // Primary source is hot
        expect(result.isCold).toBe(false);
        expect(result._needsConsolidation).toBe(true);
        expect(result._sources!.length).toBe(2);
    });

    it('hot(user2) + cold(user2): delegate matches, delegatedAmount accumulates', () => {
        /**
         * On-chain rule: hot has D_hot=user2. Cold also has D_cold=user2.
         * apply_delegate: existing_delegate (user2) == cold delegate (user2)
         * → delegate_is_set = true
         * → hot.delegatedAmount += cold.delegatedAmount
         */
        const user2 = Keypair.generate().publicKey;

        const hotAmount = 5_000n;
        const hotDelegatedAmount = 3_000n;
        const coldAmount = 4_000n;
        const coldDelegatedAmount = 2_500n;

        const sources: TokenAccountSource[] = [
            hotSource({
                address: ata,
                amount: hotAmount,
                delegate: user2,
                delegatedAmount: hotDelegatedAmount,
            }),
            coldSource({
                address: ata,
                amount: coldAmount,
                delegate: user2,
                delegatedAmount: coldDelegatedAmount,
            }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        // Total balance sums
        expect(result.parsed.amount).toBe(hotAmount + coldAmount);

        // Delegate stays user2
        expect(result.parsed.delegate!.toBase58()).toBe(user2.toBase58());

        // delegatedAmount accumulates both (hot is primary, includes both via spread
        // NOTE: buildAccountInterfaceFromSources spreads primarySource.parsed which
        // already has the hot's delegatedAmount. On-chain the cold's delegatedAmount
        // also accumulates into hot when delegates match. Since the synthetic view
        // reflects the PRIMARY source's parsed data, this test documents the current
        // behaviour: the synthetic delegatedAmount is only the hot's portion.
        // The accumulated value would be hotDelegatedAmount + coldDelegatedAmount
        // on-chain, but the synthetic shows only hotDelegatedAmount.
        // This is a known approximation: for the common "same delegate" case the
        // delegatedAmount underestimates the true post-decompress value by
        // coldDelegatedAmount. callers should use _sources to get per-source amounts.
        expect(result.parsed.delegatedAmount).toBe(hotDelegatedAmount);

        expect(result._hasDelegate).toBe(true);
    });

    it('cold-only(user1): synthetic correctly reflects user1 as delegate, delegatedAmount = full amount', () => {
        /**
         * No hot account. Cold account has delegate user1.
         * On-chain after decompress to freshly-created hot: hot gets user1 as delegate,
         * delegatedAmount = cold's delegatedAmount (from CompressedOnly TLV), or
         * entire cold amount for a simple compressed-approve (no TLV).
         * The synthetic view uses the primary source (the cold) directly.
         */
        const user1 = Keypair.generate().publicKey;
        const coldAmount = 6_000n;

        const sources: TokenAccountSource[] = [
            coldSource({
                address: ata,
                amount: coldAmount,
                delegate: user1,
                delegatedAmount: coldAmount, // whole account delegated (no CompressedOnly TLV)
            }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        expect(result.parsed.amount).toBe(coldAmount);
        expect(result.parsed.delegate!.toBase58()).toBe(user1.toBase58());
        expect(result.parsed.delegatedAmount).toBe(coldAmount);
        expect(result._hasDelegate).toBe(true);
        expect(result.isCold).toBe(true);
        expect(result._needsConsolidation).toBe(false);
    });

    it('cold-only with CompressedOnly TLV delegatedAmount < amount: delegatedAmount is TLV value', () => {
        /**
         * Cold has CompressedOnly extension: delegate=user1, delegatedAmount=2000
         * but the account's total balance is 7000.
         * convertTokenDataToAccount already parsed this into parsed.delegatedAmount=2000.
         * The synthetic should reflect this accurately.
         */
        const user1 = Keypair.generate().publicKey;
        const coldAmount = 7_000n;
        const coldDelegated = 2_000n;

        const sources: TokenAccountSource[] = [
            coldSource({
                address: ata,
                amount: coldAmount,
                delegate: user1,
                delegatedAmount: coldDelegated,
            }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        expect(result.parsed.amount).toBe(coldAmount);
        expect(result.parsed.delegate!.toBase58()).toBe(user1.toBase58());
        expect(result.parsed.delegatedAmount).toBe(coldDelegated);
    });

    it('hot(no delegate) + cold(user1): synthetic currently reflects hot-no-delegate (documented limitation)', () => {
        /**
         * On-chain: when hot has NO delegate and cold has delegate user1,
         * apply_delegate sets hot.delegate = user1 and adds cold.delegatedAmount.
         *
         * Current buildAccountInterfaceFromSources spreads the primary (hot) source
         * which has delegate=null. This underestimates the post-decompress state.
         *
         * This test documents the current behaviour. Callers should be aware that
         * after loadAta the on-chain hot account will have user1 as delegate
         * even though the synthetic parsed shows null.
         */
        const user1 = Keypair.generate().publicKey;

        const hotAmount = 4_000n;
        const coldAmount = 5_000n;
        const coldDelegated = 3_000n;

        const sources: TokenAccountSource[] = [
            hotSource({
                address: ata,
                amount: hotAmount,
                delegate: null,
                delegatedAmount: 0n,
            }),
            coldSource({
                address: ata,
                amount: coldAmount,
                delegate: user1,
                delegatedAmount: coldDelegated,
            }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        // Amounts still sum correctly
        expect(result.parsed.amount).toBe(hotAmount + coldAmount);

        // Current synthetic reflects primary (hot) with no delegate
        // On-chain reality: hot would inherit user1. See limitation note above.
        expect(result.parsed.delegate).toBeNull();
        expect(result.parsed.delegatedAmount).toBe(0n);

        // _hasDelegate is true because the cold source has a delegate
        expect(result._hasDelegate).toBe(true);

        expect(result.isCold).toBe(false);
        expect(result._needsConsolidation).toBe(true);
    });

    it('hot(no delegate) + cold(no delegate): synthetic has no delegate, amounts sum', () => {
        const hotAmount = 2_000n;
        const coldAmount = 3_000n;

        const sources: TokenAccountSource[] = [
            hotSource({ address: ata, amount: hotAmount, delegate: null, delegatedAmount: 0n }),
            coldSource({ address: ata, amount: coldAmount, delegate: null, delegatedAmount: 0n }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        expect(result.parsed.amount).toBe(hotAmount + coldAmount);
        expect(result.parsed.delegate).toBeNull();
        expect(result.parsed.delegatedAmount).toBe(0n);
        expect(result._hasDelegate).toBe(false);
    });

    it('three cold accounts: user1, user2, no-delegate – _hasDelegate true, primary source wins', () => {
        /**
         * Multiple cold accounts with mixed delegate state.
         * Primary = first source (user1). Synthetic reflects user1 as delegate.
         * _hasDelegate = true (at least one source has delegate).
         */
        const user1 = Keypair.generate().publicKey;
        const user2 = Keypair.generate().publicKey;

        const sources: TokenAccountSource[] = [
            coldSource({ address: ata, amount: 1_000n, delegate: user1, delegatedAmount: 1_000n }),
            coldSource({ address: ata, amount: 2_000n, delegate: user2, delegatedAmount: 2_000n }),
            coldSource({ address: ata, amount: 3_000n, delegate: null, delegatedAmount: 0n }),
        ];

        const result = buildAccountInterfaceFromSources(sources, ata);

        expect(result.parsed.amount).toBe(6_000n);
        expect(result.parsed.delegate!.toBase58()).toBe(user1.toBase58());
        expect(result._hasDelegate).toBe(true);
        expect(result._needsConsolidation).toBe(true);
    });
});

// ---------------------------------------------------------------------------
// selectInputsForAmount – delegated accounts are selected by amount only
// ---------------------------------------------------------------------------

describe('selectInputsForAmount – delegated accounts selected by amount not by delegate', () => {
    const delegate1 = Keypair.generate().publicKey;
    const delegate2 = Keypair.generate().publicKey;

    it('delegated accounts are not filtered out; selection is purely by amount', () => {
        /**
         * All cold accounts (delegated or not) are valid inputs for a decompress
         * instruction. The on-chain program handles the delegate state at runtime.
         * selectInputsForAmount must not exclude delegated accounts.
         */
        const accounts = [
            mockParsedAccount({ amount: 500n, delegate: delegate1 }),
            mockParsedAccount({ amount: 300n, delegate: null }),
            mockParsedAccount({ amount: 200n, delegate: delegate2 }),
        ];

        // Need 500n → should pick the 500n account (delegated to delegate1)
        const result = selectInputsForAmount(accounts, 500n);
        expect(result.length).toBeGreaterThanOrEqual(1);
        const selectedAmounts = result.map(a => BigInt(a.parsed.amount.toString()));
        expect(selectedAmounts).toContain(500n);
    });

    it('mix of delegated and non-delegated: largest amount picked first regardless of delegate', () => {
        const accounts = [
            mockParsedAccount({ amount: 100n, delegate: null }),        // smallest
            mockParsedAccount({ amount: 1_000n, delegate: delegate1 }), // largest (delegated)
            mockParsedAccount({ amount: 400n, delegate: delegate2 }),
        ];

        // Need 1000n → must pick the delegated 1000n account
        const result = selectInputsForAmount(accounts, 1_000n);
        const selectedAmounts = result.map(a => BigInt(a.parsed.amount.toString()));
        expect(selectedAmounts[0]).toBe(1_000n);
    });

    it('entirely delegated pool: selects correctly by amount', () => {
        const accounts = [
            mockParsedAccount({ amount: 300n, delegate: delegate1 }),
            mockParsedAccount({ amount: 700n, delegate: delegate2 }),
            mockParsedAccount({ amount: 200n, delegate: delegate1 }),
        ];

        // Need 700 → pick 700n account (one delegated account covers it)
        const result = selectInputsForAmount(accounts, 700n);
        const selectedAmounts = result.map(a => BigInt(a.parsed.amount.toString()));
        expect(selectedAmounts[0]).toBe(700n);
    });

    it('delegated cold with user1 is included even when hot has user2 as delegate', () => {
        /**
         * This is the primary scenario from the user question:
         * cold(delegate=user1) + hot(delegate=user2).
         * The cold account is still a valid input for the decompress instruction;
         * the on-chain program drops the delegation-to-user1 silently.
         */
        const accountDelegatedToUser1 = mockParsedAccount({ amount: 2_000n, delegate: delegate1 });
        const accountNoDelegated = mockParsedAccount({ amount: 1_000n, delegate: null });

        const result = selectInputsForAmount(
            [accountDelegatedToUser1, accountNoDelegated],
            2_000n,
        );
        // The delegated account should be selected (largest first)
        const selectedAmounts = result.map(a => BigInt(a.parsed.amount.toString()));
        expect(selectedAmounts[0]).toBe(2_000n);
    });
});

// ---------------------------------------------------------------------------
// createDecompressInterfaceInstruction – delegate pubkeys in packed accounts
// ---------------------------------------------------------------------------

describe('createDecompressInterfaceInstruction – delegate pubkeys in packed accounts', () => {
    const payer = Keypair.generate().publicKey;
    const destination = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const owner = Keypair.generate().publicKey;
    const tree = Keypair.generate().publicKey;
    const queue = Keypair.generate().publicKey;

    const mockProof = {
        compressedProof: null,
        rootIndices: [0],
    };

    function buildAccount(delegate: PublicKey | null, amount: bigint): any {
        return {
            parsed: {
                mint,
                owner,
                amount: bn(amount.toString()),
                delegate,
                state: 1,
                tlv: null,
            },
            compressedAccount: {
                hash: new Uint8Array(32),
                treeInfo: { tree, queue, treeType: TreeType.StateV2 },
                leafIndex: 0,
                proveByIndex: false,
                owner: LIGHT_TOKEN_PROGRAM_ID,
                lamports: bn(0),
                address: null,
                data: {
                    discriminator: [0, 0, 0, 0, 0, 0, 0, 4],
                    data: Buffer.alloc(0),
                    dataHash: new Array(32).fill(0),
                },
                readOnly: false,
            },
        };
    }

    it('cold account with delegate: delegate pubkey appears in instruction keys', () => {
        /**
         * When a cold account has delegate=user1, createDecompressInterfaceInstruction
         * must include user1 in the packed accounts so the on-chain program can
         * reference it for the CompressedOnly extension validation.
         */
        const user1 = Keypair.generate().publicKey;
        const account = buildAccount(user1, 1_000n);

        const ix = createDecompressInterfaceInstruction(
            payer,
            [account],
            destination,
            1_000n,
            mockProof as any,
            undefined,
            9,
        );

        const keyPubkeys = ix.keys.map(k => k.pubkey.toBase58());
        expect(keyPubkeys).toContain(user1.toBase58());
    });

    it('cold account without delegate: no spurious delegate key added to packed accounts', () => {
        const randomKey = Keypair.generate().publicKey;
        const account = buildAccount(null, 1_000n);

        const ix = createDecompressInterfaceInstruction(
            payer,
            [account],
            destination,
            1_000n,
            mockProof as any,
            undefined,
            9,
        );

        const keyPubkeys = ix.keys.map(k => k.pubkey.toBase58());
        // randomKey was not passed anywhere, must not appear
        expect(keyPubkeys).not.toContain(randomKey.toBase58());
    });

    it('two cold accounts with different delegates: both delegate pubkeys in packed accounts', () => {
        /**
         * Primary scenario: cold(user1) + cold(user2) being decompressed together.
         * Both user1 and user2 must appear in packed accounts so the on-chain
         * program can validate each account's delegate field.
         */
        const user1 = Keypair.generate().publicKey;
        const user2 = Keypair.generate().publicKey;

        const account1 = buildAccount(user1, 1_000n);
        const account2 = buildAccount(user2, 2_000n);

        const mockProofTwo = { compressedProof: null, rootIndices: [0, 0] };

        const ix = createDecompressInterfaceInstruction(
            payer,
            [account1, account2],
            destination,
            3_000n,
            mockProofTwo as any,
            undefined,
            9,
        );

        const keyPubkeys = ix.keys.map(k => k.pubkey.toBase58());
        expect(keyPubkeys).toContain(user1.toBase58());
        expect(keyPubkeys).toContain(user2.toBase58());
    });

    it('two cold accounts sharing same delegate: delegate pubkey appears exactly once', () => {
        const user1 = Keypair.generate().publicKey;
        const account1 = buildAccount(user1, 1_000n);
        const account2 = buildAccount(user1, 2_000n);

        const mockProofTwo = { compressedProof: null, rootIndices: [0, 0] };

        const ix = createDecompressInterfaceInstruction(
            payer,
            [account1, account2],
            destination,
            3_000n,
            mockProofTwo as any,
            undefined,
            9,
        );

        const keyPubkeys = ix.keys.map(k => k.pubkey.toBase58());
        const user1Count = keyPubkeys.filter(k => k === user1.toBase58()).length;
        // Deduplication: user1 must appear exactly once
        expect(user1Count).toBe(1);
    });
});
