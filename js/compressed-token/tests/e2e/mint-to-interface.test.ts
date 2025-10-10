import { describe, it, expect, beforeAll } from 'vitest';
import { PublicKey, Keypair, Signer } from '@solana/web3.js';
import {
    Rpc,
    newAccountWithLamports,
    createRpc,
    getTestRpc,
    bn,
    VERSION,
    featureFlags,
    CTOKEN_PROGRAM_ID,
    selectStateTreeInfo,
    TreeInfo,
} from '@lightprotocol/stateless.js';
import { WasmFactory } from '@lightprotocol/hasher.rs';
import { createMint } from '../../src/mint/actions/create-mint';
import { mintToInterface } from '../../src/mint/actions/mint-to-interface';
import { createMintSPL } from '../../src/actions/create-mint';
import { getMintInterface } from '../../src/mint/helpers';
import { createAssociatedCTokenAccount } from '../../src/mint/actions/create-associated-ctoken';
import { getAssociatedCTokenAddress } from '../../src/compressible/derivation';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

featureFlags.version = VERSION.V2;

const TEST_TOKEN_DECIMALS = 9;

describe('mintToInterface - SPL Mints', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;
    let recipient1: Keypair;
    let recipient2: Keypair;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();
        recipient1 = Keypair.generate();
        recipient2 = Keypair.generate();

        const mintKeypair = Keypair.generate();
        mint = (
            await createMintSPL(
                rpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                mintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await rpc.getStateTreeInfos());
        tokenPoolInfo = selectTokenPoolInfo(await getTokenPoolInfos(rpc, mint));
    });

    it('should mint SPL tokens to single compressed account', async () => {
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipient1.publicKey,
            mintAuthority,
            amount,
            stateTreeInfo,
            tokenPoolInfo,
        );

        const compressedAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient1.publicKey,
            { mint },
        );

        expect(compressedAccounts.items.length).toBeGreaterThan(0);
        const account = compressedAccounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account).toBeDefined();
        expect(account!.parsed.amount.eq(bn(amount))).toBe(true);
    });

    it('should mint SPL tokens to multiple compressed accounts', async () => {
        const amounts = [500, 750];
        const recipients = [recipient1.publicKey, recipient2.publicKey];

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipients,
            mintAuthority,
            amounts,
            stateTreeInfo,
            tokenPoolInfo,
        );

        for (let i = 0; i < recipients.length; i++) {
            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                recipients[i],
                { mint },
            );
            const account = accounts.items.find(acc =>
                acc.parsed.mint.equals(mint),
            );
            expect(account).toBeDefined();
        }
    });

    it('should auto-resolve stateTreeInfo and tokenPoolInfo if not provided', async () => {
        const amount = 500;
        const recipient = Keypair.generate().publicKey;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipient,
            mintAuthority,
            amount,
        );

        const compressedAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient,
            { mint },
        );

        expect(compressedAccounts.items.length).toBeGreaterThan(0);
        const account = compressedAccounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account).toBeDefined();
        expect(account!.parsed.amount.eq(bn(amount))).toBe(true);
    });

    it('should fail with wrong authority for SPL mint', async () => {
        const wrongAuthority = Keypair.generate();

        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                recipient1.publicKey,
                wrongAuthority,
                100,
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrow();
    });
});

describe('mintToInterface - Compressed Mints', () => {
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
        const result = await createMint(
            rpc,
            payer,
            mintAuthority,
            null,
            decimals,
            mintSigner,
            undefined,
            undefined,
            undefined,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        mint = result.mint;
    });

    it('should mint compressed tokens to single compressed account', async () => {
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            [recipient1.publicKey],
            mintAuthority,
            [amount],
        );

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

        const mintInfo = await getMintInterface(rpc, mint);
        expect(mintInfo.mint.supply).toBe(BigInt(amount));
    });

    it('should mint compressed tokens to multiple compressed accounts', async () => {
        const amounts = [500, 750];
        const recipients = [recipient1.publicKey, recipient2.publicKey];

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipients,
            mintAuthority,
            amounts,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        for (let i = 0; i < recipients.length; i++) {
            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                recipients[i],
            );
            const account = accounts.items.find(acc =>
                acc.parsed.mint.equals(mint),
            );
            expect(account).toBeDefined();
        }

        const mintInfo = await getMintInterface(rpc, mint);
        const previousSupply = 1000n;
        expect(mintInfo.mint.supply).toBe(
            previousSupply + BigInt(amounts[0]) + BigInt(amounts[1]),
        );
    });

    it('should mint compressed tokens to onchain ctoken account', async () => {
        const recipient = Keypair.generate();
        const { transactionSignature } = await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );
        await rpc.confirmTransaction(transactionSignature, 'confirmed');

        const recipientCToken = getAssociatedCTokenAddress(
            recipient.publicKey,
            mint,
        );
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            recipientCToken,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accountInfo = await rpc.getAccountInfo(recipientCToken);
        expect(accountInfo).toBeDefined();
        expect(accountInfo?.owner.toString()).toBe(
            CTOKEN_PROGRAM_ID.toBase58(),
        );
    });

    it('should fail with wrong authority for compressed mint', async () => {
        const wrongAuthority = Keypair.generate();

        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                [recipient1.publicKey],
                wrongAuthority,
                [100],
            ),
        ).rejects.toThrow();
    });

    it('should support bigint amounts for compressed mint', async () => {
        const amount = 1000000000n;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            [recipient1.publicKey],
            mintAuthority,
            [amount],
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const mintInfo = await getMintInterface(rpc, mint);
        expect(mintInfo.mint.supply).toBeGreaterThanOrEqual(amount);
    });

    it('should fail when recipient array length does not match amount array length', async () => {
        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                [recipient1.publicKey, recipient2.publicKey],
                mintAuthority,
                [100],
            ),
        ).rejects.toThrow(
            'Recipient and amount arrays must have the same length',
        );
    });

    it('should fail when recipient is array but amount is not', async () => {
        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                [recipient1.publicKey, recipient2.publicKey],
                mintAuthority,
                100,
            ),
        ).rejects.toThrow('Amount must be an array when recipient is an array');
    });

    it('should fail when recipient is single but amount is array', async () => {
        await expect(
            mintToInterface(
                rpc,
                payer,
                mint,
                recipient1.publicKey,
                mintAuthority,
                [100, 200],
            ),
        ).rejects.toThrow(
            'Amount must be a single value when recipient is a single address',
        );
    });
});

describe('mintToInterface - Mixed Workflow', () => {
    let rpc: Rpc;
    let testRpc: Rpc;
    let payer: Signer;
    let splMint: PublicKey;
    let compressedMint: PublicKey;
    let mintAuthority: Keypair;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = createRpc();
        testRpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const splMintKeypair = Keypair.generate();
        splMint = (
            await createMintSPL(
                testRpc,
                payer,
                mintAuthority.publicKey,
                TEST_TOKEN_DECIMALS,
                splMintKeypair,
            )
        ).mint;

        stateTreeInfo = selectStateTreeInfo(await testRpc.getStateTreeInfos());
        tokenPoolInfo = selectTokenPoolInfo(
            await getTokenPoolInfos(testRpc, splMint),
        );

        const compressedMintSigner = Keypair.generate();
        const result = await createMint(
            rpc,
            payer,
            mintAuthority,
            null,
            TEST_TOKEN_DECIMALS,
            compressedMintSigner,
            undefined,
            undefined,
            undefined,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        compressedMint = result.mint;
    });

    it('should handle both SPL and compressed mints correctly', async () => {
        const recipient = Keypair.generate();
        const amount = 1000;

        const splTxId = await mintToInterface(
            testRpc,
            payer,
            splMint,
            recipient.publicKey,
            mintAuthority,
            amount,
            stateTreeInfo,
            tokenPoolInfo,
        );

        const compressedTxId = await mintToInterface(
            rpc,
            payer,
            compressedMint,
            [recipient.publicKey],
            mintAuthority,
            [amount],
        );

        await rpc.confirmTransaction(compressedTxId, 'confirmed');

        const splAccounts = await testRpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
            { mint: splMint },
        );
        expect(splAccounts.items.length).toBeGreaterThan(0);

        const compressedAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const compressedAccount = compressedAccounts.items.find(acc =>
            acc.parsed.mint.equals(compressedMint),
        );
        expect(compressedAccount).toBeDefined();
        expect(compressedAccount!.parsed.amount.toNumber()).toBe(amount);
    });

    it('should fail with non-existent mint', async () => {
        const fakeMint = Keypair.generate().publicKey;
        const recipient = Keypair.generate().publicKey;

        await expect(
            mintToInterface(
                rpc,
                payer,
                fakeMint,
                recipient,
                mintAuthority,
                100,
            ),
        ).rejects.toThrow('Mint not found');
    });
});

describe('mintToInterface - Edge Cases', () => {
    let rpc: Rpc;
    let payer: Signer;
    let compressedMint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const mintSigner = Keypair.generate();
        const result = await createMint(
            rpc,
            payer,
            mintAuthority,
            null,
            6,
            mintSigner,
            undefined,
            undefined,
            undefined,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        compressedMint = result.mint;
    });

    it('should handle zero amount minting', async () => {
        const recipient = Keypair.generate();

        const txId = await mintToInterface(
            rpc,
            payer,
            compressedMint,
            [recipient.publicKey],
            mintAuthority,
            [0],
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const account = accounts.items.find(acc =>
            acc.parsed.mint.equals(compressedMint),
        );
        expect(account).toBeDefined();
        expect(account!.parsed.amount.toNumber()).toBe(0);
    });

    it('should handle large amounts with bigint', async () => {
        const recipient = Keypair.generate();
        const largeAmount = 1000000000000n;

        const txId = await mintToInterface(
            rpc,
            payer,
            compressedMint,
            [recipient.publicKey],
            mintAuthority,
            [largeAmount],
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const mintInfo = await getMintInterface(rpc, compressedMint);
        expect(mintInfo.mint.supply).toBeGreaterThanOrEqual(largeAmount);
    });

    it('should handle payer as authority', async () => {
        const mintSigner = Keypair.generate();
        const result = await createMint(
            rpc,
            payer,
            payer as Keypair,
            null,
            9,
            mintSigner,
            undefined,
            undefined,
            undefined,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');

        const recipient = Keypair.generate();
        const amount = 1000;

        const txId = await mintToInterface(
            rpc,
            payer,
            result.mint,
            [recipient.publicKey],
            payer as Keypair,
            [amount],
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const account = accounts.items.find(acc =>
            acc.parsed.mint.equals(result.mint),
        );
        expect(account).toBeDefined();
    });

    it('should handle multiple recipients with varying amounts', async () => {
        const recipients = Array.from(
            { length: 5 },
            () => Keypair.generate().publicKey,
        );
        const amounts = [100, 200, 300, 400, 500];

        const txId = await mintToInterface(
            rpc,
            payer,
            compressedMint,
            recipients,
            mintAuthority,
            amounts,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        for (let i = 0; i < recipients.length; i++) {
            const accounts = await rpc.getCompressedTokenAccountsByOwner(
                recipients[i],
            );
            const account = accounts.items.find(acc =>
                acc.parsed.mint.equals(compressedMint),
            );
            expect(account).toBeDefined();
        }
    });
});

describe('mintToInterface - Custom Queues', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintAuthority = Keypair.generate();

        const mintSigner = Keypair.generate();
        const result = await createMint(
            rpc,
            payer,
            mintAuthority,
            null,
            9,
            mintSigner,
            undefined,
            undefined,
            undefined,
        );
        await rpc.confirmTransaction(result.transactionSignature, 'confirmed');
        mint = result.mint;
    });

    it('should use custom outputQueue when provided', async () => {
        const recipient = Keypair.generate();
        const amount = 500;
        const customQueue = (await rpc.getStateTreeInfos())[0].queue;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            [recipient.publicKey],
            mintAuthority,
            [amount],
            undefined,
            undefined,
            customQueue,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const account = accounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account).toBeDefined();
    });

    it('should use custom tokensOutQueue when provided', async () => {
        const recipient = Keypair.generate();
        const amount = 750;
        const customQueue = (await rpc.getStateTreeInfos())[0].queue;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            [recipient.publicKey],
            mintAuthority,
            [amount],
            undefined,
            undefined,
            customQueue,
            customQueue,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const account = accounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account).toBeDefined();
    });

    it('should default tokensOutQueue to outputQueue when only outputQueue provided', async () => {
        const recipient = Keypair.generate();
        const amount = 250;
        const customQueue = (await rpc.getStateTreeInfos())[0].queue;

        const txId = await mintToInterface(
            rpc,
            payer,
            mint,
            [recipient.publicKey],
            mintAuthority,
            [amount],
            undefined,
            undefined,
            customQueue,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const accounts = await rpc.getCompressedTokenAccountsByOwner(
            recipient.publicKey,
        );
        const account = accounts.items.find(acc =>
            acc.parsed.mint.equals(mint),
        );
        expect(account).toBeDefined();
        expect(account!.parsed.amount.toNumber()).toBe(amount);
    });
});
