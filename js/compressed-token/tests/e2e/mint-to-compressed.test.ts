import { describe, it, expect, beforeAll } from 'vitest';
import {
    PublicKey,
    Keypair,
    Signer,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    VERSION,
    featureFlags,
    CTOKEN_PROGRAM_ID,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import { createMintInterface } from '../../src/v3/actions/create-mint-interface';
import { mintToCompressed } from '../../src/v3/actions/mint-to-compressed';
import { getMintInterface } from '../../src/v3/get-mint-interface';
import { findMintAddress } from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('mintToCompressed', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintSigner: Keypair;
    let mintAuthority: Keypair;
    let mint: PublicKey;
    let recipient1: Keypair;
    let recipient2: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintSigner = Keypair.generate();
        mintAuthority = Keypair.generate();
        recipient1 = Keypair.generate();
        recipient2 = Keypair.generate();

        const decimals = 9;
        const result = await createMintInterface(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        mint = result.mint;
    });

    it('should mint tokens to a single recipient', async () => {
        const amount = 1000;

        const txId = await mintToCompressed(rpc, payer, mint, mintAuthority, [
            { recipient: recipient1.publicKey, amount },
        ]);

        await rpc.confirmTransaction(txId, 'confirmed');

        const compressedAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient1.publicKey,
        );

        expect(compressedAccounts.items.length).toBeGreaterThan(0);

        const account = compressedAccounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );

        expect(account).toBeDefined();
        expect(account!.parsed.amount.toNumber()).toBe(amount);
        expect(account!.parsed.owner.toString()).toBe(
            recipient1.publicKey.toString(),
        );

        const mintInfo = await getMintInterface(
            rpc,
            mint,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBe(BigInt(amount));
    });

    it('should mint tokens to multiple recipients', async () => {
        const amount1 = 500;
        const amount2 = 750;

        const txId = await mintToCompressed(rpc, payer, mint, mintAuthority, [
            { recipient: recipient1.publicKey, amount: amount1 },
            { recipient: recipient2.publicKey, amount: amount2 },
        ]);

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts1 = await rpc.getCompressedTokenAccountsByOwner(
            recipient1.publicKey,
        );
        const account1 = accounts1.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account1).toBeDefined();

        const accounts2 = await rpc.getCompressedTokenAccountsByOwner(
            recipient2.publicKey,
        );
        const account2 = accounts2.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account2).toBeDefined();
        expect(account2!.parsed.amount.toNumber()).toBe(amount2);

        const mintInfo = await getMintInterface(
            rpc,
            mint,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBe(BigInt(1000 + amount1 + amount2));
    });

    it('should fail with wrong authority', async () => {
        const wrongAuthority = Keypair.generate();

        await expect(
            mintToCompressed(rpc, payer, mint, wrongAuthority, [
                { recipient: recipient1.publicKey, amount: 100 },
            ]),
        ).rejects.toThrow();
    });

    it('should support bigint amounts', async () => {
        const amount = 1000000000n;

        const txId = await mintToCompressed(rpc, payer, mint, mintAuthority, [
            { recipient: recipient1.publicKey, amount },
        ]);

        await rpc.confirmTransaction(txId, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mint,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBeGreaterThanOrEqual(amount);
    });
});
