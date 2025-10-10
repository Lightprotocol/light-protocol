import { describe, it, expect, beforeAll } from 'vitest';
import {
    PublicKey,
    Signer,
    Keypair,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import BN from 'bn.js';
import { createMint, createTokenProgramLookupTable } from '../../src/actions';
import {
    getTestKeypair,
    newAccountWithLamports,
    bn,
    Rpc,
    sendAndConfirmTx,
    buildAndSignTx,
    dedupeSigner,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';

import { CompressedTokenProgram } from '../../src/program';
import { NobleHasherFactory } from '@lightprotocol/program-test';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';
import { getTestRpc } from '@lightprotocol/program-test';

/**
 * Asserts that mintTo() creates a new compressed token account for the
 * recipient
 */
async function assertMintTo(
    rpc: Rpc,
    refMint: PublicKey,
    refAmount: BN,
    refTo: PublicKey,
) {
    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        refTo,
        {
            mint: refMint,
        },
    );

    const compressedTokenAccount = compressedTokenAccounts.items[0];
    expect(compressedTokenAccount.parsed.mint.toBase58()).toBe(
        refMint.toBase58(),
    );
    expect(compressedTokenAccount.parsed.amount.eq(refAmount)).toBe(true);
    expect(compressedTokenAccount.parsed.owner.equals(refTo)).toBe(true);
    expect(compressedTokenAccount.parsed.delegate).toBe(null);
}

const TEST_TOKEN_DECIMALS = 2;

describe('mintTo', () => {
    let rpc: Rpc;
    let payer: Signer;
    let bob: Signer;
    let mint: PublicKey;
    let mintAuthority: Keypair;
    let lut: PublicKey;
    let stateTreeInfo: TreeInfo;
    let tokenPoolInfo: TokenPoolInfo;

    beforeAll(async () => {
        const lightWasm = await NobleHasherFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc);
        bob = getTestKeypair();
        mintAuthority = payer as Keypair;
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

        /// Setup LUT.
        const { address } = await createTokenProgramLookupTable(
            rpc,
            payer,
            payer,
            [mint, payer.publicKey],
        );
        lut = address;
    }, 80_000);

    // const maxRecipients = 18;
    const maxRecipients = 22;
    const recipients = Array.from(
        { length: maxRecipients },
        () => Keypair.generate().publicKey,
    );
    const amounts = Array.from({ length: maxRecipients }, (_, i) => bn(i + 1));

    it(`should mint to ${recipients.length} recipients optimized with LUT`, async () => {
        const lookupTableAccount = (await rpc.getAddressLookupTable(lut))
            .value!;

        const ix = await CompressedTokenProgram.mintTo({
            feePayer: payer.publicKey,
            mint,
            authority: mintAuthority.publicKey,
            amount: amounts,
            toPubkey: recipients,
            outputStateTreeInfo: stateTreeInfo,
            tokenPoolInfo,
        });

        const { blockhash } = await rpc.getLatestBlockhash();
        const additionalSigners = dedupeSigner(payer, [mintAuthority]);

        const tx = buildAndSignTx(
            [ComputeBudgetProgram.setComputeUnitLimit({ units: 600_000 }), ix],
            payer,
            blockhash,
            additionalSigners,
            [lookupTableAccount],
        );
        const txId = await sendAndConfirmTx(rpc, tx);
    });
});
