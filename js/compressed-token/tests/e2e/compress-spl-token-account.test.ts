import { describe, it, expect, beforeAll, assert } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    bn,
    defaultTestStateTreeAccounts,
    newAccountWithLamports,
    getTestRpc,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    createMint,
    decompress,
    mintTo,
    compressSplTokenAccount,
} from '../../src/actions';
import {
    createAssociatedTokenAccount,
    mintToChecked,
    TOKEN_2022_PROGRAM_ID,
} from '@solana/spl-token';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

const TEST_TOKEN_DECIMALS = 2;

describe('compressSplTokenAccount', () => {
    let rpc: Rpc;
    let payer: Signer;
    let alice: Signer;
    let aliceAta: PublicKey;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);

        mintAuthority = Keypair.generate();
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));

        alice = await newAccountWithLamports(rpc, 1e9);
        aliceAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            alice.publicKey,
        );

        // Mint some tokens to alice's ATA
        await mintTo(
            rpc,
            payer,
            mint,
            alice.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
            tokenPoolInfo,
        );

        await decompress(rpc, payer, mint, bn(1000), alice, aliceAta);
    }, 80_000);

    it('should compress entire token balance when remainingAmount is undefined', async () => {
        // Get initial ATA balance
        const ataBalanceBefore = await rpc.getTokenAccountBalance(aliceAta);

        const initialCompressedBalance =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Compress the entire balance
        await compressSplTokenAccount(
            rpc,
            payer,
            mint,
            alice,
            aliceAta,
            undefined,
            stateTreeInfo,
            tokenPoolInfo,
        );

        // Get final balances
        const ataBalanceAfter = await rpc.getTokenAccountBalance(aliceAta);
        const compressedBalanceAfter =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Assert ATA is empty
        expect(bn(ataBalanceAfter.value.amount).eq(bn(0))).toBe(true);

        // Assert compressed balance equals original ATA balance
        const totalCompressedAmount = compressedBalanceAfter.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );
        const initialCompressedAmount = initialCompressedBalance.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );

        expect(
            totalCompressedAmount.eq(
                bn(ataBalanceBefore.value.amount).add(initialCompressedAmount),
            ),
        ).toBe(true);
    });

    it('should fail when trying to compress more than available balance', async () => {
        // Mint new tokens for this test
        const testAmount = bn(100);

        await mintToChecked(
            rpc,
            payer,
            mint,
            aliceAta,
            mintAuthority,
            testAmount.toNumber(),
            TEST_TOKEN_DECIMALS,
        );

        // Try to compress more than available
        await expect(
            compressSplTokenAccount(
                rpc,
                payer,
                mint,
                alice,
                aliceAta,
                bn(testAmount.add(bn(1))), // Try to leave more than available
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow();
    });

    it('should leave specified remaining amount in token account', async () => {
        /// still has 100
        expect(
            Number((await rpc.getTokenAccountBalance(aliceAta)).value.amount),
        ).toBe(100);

        const remainingAmount = bn(10);
        const ataBalanceBefore = await rpc.getTokenAccountBalance(aliceAta);
        const initialCompressedBalance =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Compress tokens while leaving remainingAmount
        await compressSplTokenAccount(
            rpc,
            payer,
            mint,
            alice,
            aliceAta,
            remainingAmount,
            stateTreeInfo,
            tokenPoolInfo,
        );

        // Get final balances
        const ataBalanceAfter = await rpc.getTokenAccountBalance(aliceAta);
        const compressedBalanceAfter =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Assert remaining amount in ATA
        expect(bn(ataBalanceAfter.value.amount).eq(remainingAmount)).toBe(true);

        // Assert compressed amount is correct
        const totalCompressedAmount = compressedBalanceAfter.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );
        const initialCompressedAmount = initialCompressedBalance.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );

        // Assert that the total compressed amount equals:
        // Initial ATA balance - remaining amount + initial compressed amount
        expect(
            totalCompressedAmount.eq(
                bn(ataBalanceBefore.value.amount)
                    .sub(remainingAmount)
                    .add(initialCompressedAmount),
            ),
        ).toBe(true);
    });

    it('should handle remainingAmount = current balance', async () => {
        // Mint some tokens for testing
        const testAmount = bn(100);
        await mintToChecked(
            rpc,
            payer,
            mint,
            aliceAta,
            mintAuthority,
            testAmount.toNumber(),
            TEST_TOKEN_DECIMALS,
        );

        const balanceBefore = await rpc.getTokenAccountBalance(aliceAta);
        const compressedBefore = await rpc.getCompressedTokenAccountsByOwner(
            alice.publicKey,
            { mint },
        );

        await compressSplTokenAccount(
            rpc,
            payer,
            mint,
            alice,
            aliceAta,
            bn(balanceBefore.value.amount),
            stateTreeInfo,
            tokenPoolInfo,
        );

        const balanceAfter = await rpc.getTokenAccountBalance(aliceAta);
        const compressedAfter = await rpc.getCompressedTokenAccountsByOwner(
            alice.publicKey,
            { mint },
        );

        expect(balanceAfter.value.amount).toBe(balanceBefore.value.amount);
        expect(compressedAfter.items.length).toBe(
            compressedBefore.items.length + 1,
        );
        expect(
            compressedAfter.items.some(item => item.parsed.amount.eq(bn(0))),
        ).toBe(true);
    });

    it('should fail when non-owner tries to compress', async () => {
        const nonOwner = await newAccountWithLamports(rpc, 1e9);

        // Mint some tokens to ensure non-zero balance
        await mintToChecked(
            rpc,
            payer,
            mint,
            aliceAta,
            mintAuthority,
            100,
            TEST_TOKEN_DECIMALS,
        );

        await expect(
            compressSplTokenAccount(
                rpc,
                payer,
                mint,
                nonOwner, // wrong signer
                aliceAta,
                undefined,
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow();
    });

    it('should fail with invalid state tree', async () => {
        const invalidTreeInfo = selectStateTreeInfo(
            await rpc.getStateTreeInfos(),
        );
        invalidTreeInfo.tree = Keypair.generate().publicKey;
        invalidTreeInfo.queue = Keypair.generate().publicKey;

        // Mint some tokens to ensure non-zero balance
        await mintToChecked(
            rpc,
            payer,
            mint,
            aliceAta,
            mintAuthority,
            100,
            TEST_TOKEN_DECIMALS,
        );

        await expect(
            compressSplTokenAccount(
                rpc,
                payer,
                mint,
                alice,
                aliceAta,
                undefined,
                invalidTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow();
    });

    it('should compress entire token 2022 account balance when remainingAmount is undefined', async () => {
        const mintKeypair = Keypair.generate();

        mint = (
            await createMint(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
                undefined,
                true,
            )
        ).mint;

        const tokenPoolInfoT22 = selectTokenPoolInfo(
            await getTokenPoolInfos(rpc, mint),
        );
        const mintAccountInfo = await rpc.getAccountInfo(mint);
        assert.equal(
            mintAccountInfo!.owner.toBase58(),
            TOKEN_2022_PROGRAM_ID.toBase58(),
        );

        alice = await newAccountWithLamports(rpc, 1e9);
        aliceAta = await createAssociatedTokenAccount(
            rpc,
            payer,
            mint,
            alice.publicKey,
            undefined,
            TOKEN_2022_PROGRAM_ID,
        );

        // Mint some tokens to alice's ATA
        await mintTo(
            rpc,
            payer,
            mint,
            alice.publicKey,
            mintAuthority,
            bn(1000),
            stateTreeInfo,
        );

        await decompress(rpc, payer, mint, bn(1000), alice, aliceAta);
        // Get initial ATA balance
        const ataBalanceBefore = await rpc.getTokenAccountBalance(aliceAta);

        const initialCompressedBalance =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Compress the entire balance
        await compressSplTokenAccount(
            rpc,
            payer,
            mint,
            alice,
            aliceAta,
            undefined,
            stateTreeInfo,
            tokenPoolInfoT22,
        );

        // Get final balances
        const ataBalanceAfter = await rpc.getTokenAccountBalance(aliceAta);
        const compressedBalanceAfter =
            await rpc.getCompressedTokenAccountsByOwner(alice.publicKey, {
                mint,
            });

        // Assert ATA is empty
        expect(bn(ataBalanceAfter.value.amount).eq(bn(0))).toBe(true);

        // Assert compressed balance equals original ATA balance
        const totalCompressedAmount = compressedBalanceAfter.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );
        const initialCompressedAmount = initialCompressedBalance.items.reduce(
            (sum, item) => sum.add(item.parsed.amount),
            bn(0),
        );

        expect(
            totalCompressedAmount.eq(
                bn(ataBalanceBefore.value.amount).add(initialCompressedAmount),
            ),
        ).toBe(true);
    });
});
