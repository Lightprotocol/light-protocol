import { describe, expect, it } from 'vitest';
import { Keypair, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';
import {
    DerivationMode,
    LIGHT_TOKEN_PROGRAM_ID,
    VERSION,
    bn,
    createRpc,
    featureFlags,
    newAccountWithLamports,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    createMintInstructions,
    createMintToCompressedInstruction,
    getMint,
} from '../../src';
import { sendInstructions } from './helpers';

featureFlags.version = VERSION.V2;
const COMPRESSED_MINT_SEED = Buffer.from('compressed_mint');

describe('mint-to-compressed instruction', () => {
    it('mints directly to compressed recipients for a light mint', async () => {
        const rpc = createRpc();
        const payer = await newAccountWithLamports(rpc, 20e9);
        const mintAuthority = Keypair.generate();
        const mintSigner = Keypair.generate();
        const recipientA = Keypair.generate();
        const recipientB = Keypair.generate();
        const outputStateTreeInfo = selectStateTreeInfo(
            await rpc.getStateTreeInfos(),
        );

        await sendInstructions(
            rpc,
            payer,
            await createMintInstructions({
                rpc,
                payer: payer.publicKey,
                keypair: mintSigner,
                decimals: 9,
                mintAuthority: mintAuthority.publicKey,
                tokenProgramId: LIGHT_TOKEN_PROGRAM_ID,
                outputStateTreeInfo,
            }),
            [mintSigner, mintAuthority],
        );

        const [mint] = PublicKey.findProgramAddressSync(
            [COMPRESSED_MINT_SEED, mintSigner.publicKey.toBuffer()],
            LIGHT_TOKEN_PROGRAM_ID,
        );
        const mintInfo = await getMint(rpc, mint, undefined, LIGHT_TOKEN_PROGRAM_ID);
        if (!mintInfo.merkleContext || !mintInfo.mintContext) {
            throw new Error('Light mint context missing.');
        }

        const validityProof = await rpc.getValidityProofV2(
            [
                {
                    hash: bn(mintInfo.merkleContext.hash),
                    leafIndex: mintInfo.merkleContext.leafIndex,
                    treeInfo: mintInfo.merkleContext.treeInfo,
                    proveByIndex: mintInfo.merkleContext.proveByIndex,
                },
            ],
            [],
            DerivationMode.compressible,
        );

        const ix = createMintToCompressedInstruction({
            authority: mintAuthority.publicKey,
            payer: payer.publicKey,
            validityProof,
            merkleContext: mintInfo.merkleContext,
            mintData: {
                supply: mintInfo.mint.supply,
                decimals: mintInfo.mint.decimals,
                mintAuthority: mintInfo.mint.mintAuthority,
                freezeAuthority: mintInfo.mint.freezeAuthority,
                splMint: mintInfo.mintContext.splMint,
                mintDecompressed: mintInfo.mintContext.cmintDecompressed,
                version: mintInfo.mintContext.version,
                mintSigner: mintInfo.mintContext.mintSigner,
                bump: mintInfo.mintContext.bump,
            },
            recipients: [
                { recipient: recipientA.publicKey, amount: 500n },
                { recipient: recipientB.publicKey, amount: 700n },
            ],
            outputStateTreeInfo,
        });

        await sendInstructions(rpc, payer, [ix], [mintAuthority]);

        const aAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipientA.publicKey,
            { mint },
        );
        const bAccounts = await rpc.getCompressedTokenAccountsByOwner(
            recipientB.publicKey,
            { mint },
        );
        const mintAfter = await getMint(rpc, mint, undefined, LIGHT_TOKEN_PROGRAM_ID);

        const amountA = aAccounts.items.reduce(
            (sum, account) => sum + BigInt(account.parsed.amount.toString()),
            0n,
        );
        const amountB = bAccounts.items.reduce(
            (sum, account) => sum + BigInt(account.parsed.amount.toString()),
            0n,
        );

        expect(amountA).toBe(500n);
        expect(amountB).toBe(700n);
        expect(mintAfter.mint.supply).toBe(1_200n);
    });
});
