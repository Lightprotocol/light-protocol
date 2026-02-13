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
} from '@lightprotocol/stateless.js';
import { createMintInterface } from '../../src/v3/actions';
import { mintTo } from '../../src/v3/actions/mint-to';
import { getMintInterface } from '../../src/v3/get-mint-interface';
import { createAssociatedCTokenAccount } from '../../src/v3/actions/create-associated-ctoken';
import {
    getAssociatedCTokenAddress,
    findMintAddress,
} from '../../src/v3/derivation';

featureFlags.version = VERSION.V2;

describe('mintTo (MintToCToken)', () => {
    let rpc: Rpc;
    let payer: Signer;
    let mintSigner: Keypair;
    let mintAuthority: Keypair;
    let mint: PublicKey;
    let recipient: Keypair;
    let recipientCToken: PublicKey;

    beforeAll(async () => {
        rpc = createRpc();
        payer = await newAccountWithLamports(rpc, 10e9);
        mintSigner = Keypair.generate();
        mintAuthority = Keypair.generate();
        recipient = Keypair.generate();

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

        await createAssociatedCTokenAccount(
            rpc,
            payer,
            recipient.publicKey,
            mint,
        );
        recipientCToken = getAssociatedCTokenAddress(recipient.publicKey, mint);
    });

    it('should mint tokens to onchain ctoken account', async () => {
        const amount = 1000;

        const txId = await mintTo(
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

        const mintInfo = await getMintInterface(
            rpc,
            mint,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBe(BigInt(amount));
    });

    it('should fail with wrong authority', async () => {
        const wrongAuthority = Keypair.generate();

        await expect(
            mintTo(rpc, payer, mint, recipientCToken, wrongAuthority, 100),
        ).rejects.toThrow();
    });

    it('should support bigint amounts', async () => {
        const amount = 500n;

        const txId = await mintTo(
            rpc,
            payer,
            mint,
            recipientCToken,
            mintAuthority,
            amount,
        );

        await rpc.confirmTransaction(txId, 'confirmed');

        const mintInfo = await getMintInterface(
            rpc,
            mint,
            undefined,
            CTOKEN_PROGRAM_ID,
        );
        expect(mintInfo.mint.supply).toBeGreaterThanOrEqual(1000n + amount);
    });
});
