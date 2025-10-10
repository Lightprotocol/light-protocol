import { describe, it, expect, beforeAll } from 'vitest';
import {
    PublicKey,
    Signer,
    Keypair,
    ComputeBudgetProgram,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    createMint,
    createTokenProgramLookupTable,
    mintTo,
} from '../../src/actions';
import {
    bn,
    Rpc,
    sendAndConfirmTx,
    buildAndSignTx,
    dedupeSigner,
    TreeInfo,
    selectStateTreeInfo,
} from '@lightprotocol/stateless.js';
import {
    createLiteSVMRpc,
    newAccountWithLamports,
} from '@lightprotocol/program-test';

import { CompressedTokenProgram } from '../../src/program';
import { NobleHasherFactory } from '@lightprotocol/program-test';
import {
    getTokenPoolInfos,
    selectTokenPoolInfo,
    TokenPoolInfo,
} from '../../src/utils/get-token-pool-infos';

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
        rpc = await createLiteSVMRpc(lightWasm);
        payer = await newAccountWithLamports(rpc, 1e9);
        bob = await newAccountWithLamports(rpc, 1e9);
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

    it('should mint to bob', async () => {
        const amount = bn(1000);
        const txId = await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            stateTreeInfo,
            tokenPoolInfo,
        );

        await assertMintTo(rpc, mint, amount, bob.publicKey);

        /// wrong authority
        /// is not checked in cToken program, so it throws invalid owner inside spl token program.
        await expect(
            mintTo(rpc, payer, mint, bob.publicKey, Keypair.generate(), amount),
        ).rejects.toThrowError(/code: 4/);

        /// with output state merkle tree defined
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            stateTreeInfo,
            tokenPoolInfo,
        );
    });

    // const maxRecipients = 18;
    const maxRecipients = 22;
    const recipients = Array.from(
        { length: maxRecipients },
        () => Keypair.generate().publicKey,
    );
    const amounts = Array.from({ length: maxRecipients }, (_, i) => bn(i + 1));

    it('should mint to multiple recipients', async () => {
        /// mint to three recipients
        await mintTo(
            rpc,
            payer,
            mint,
            recipients.slice(0, 3),
            mintAuthority,
            amounts.slice(0, 3),
            stateTreeInfo,
            tokenPoolInfo,
        );

        /// Mint to 10 recipients
        const tx = await mintTo(
            rpc,
            payer,
            mint,
            recipients.slice(0, 10),
            mintAuthority,
            amounts.slice(0, 10),
            stateTreeInfo,
            tokenPoolInfo,
        );

        // Uneven amounts
        await expect(
            mintTo(
                rpc,
                payer,
                mint,
                recipients,
                mintAuthority,
                amounts.slice(0, 2),
                stateTreeInfo,
                tokenPoolInfo,
            ),
        ).rejects.toThrowError(
            /Amount and toPubkey arrays must have the same length/,
        );
    });
});
