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
    getTestKeypair,
    newAccountWithLamports,
    bn,
    defaultTestStateTreeAccounts,
    Rpc,
    sendAndConfirmTx,
    buildAndSignTx,
    dedupeSigner,
    getTestRpc,
} from '@lightprotocol/stateless.js';

import { CompressedTokenProgram } from '../../src/program';
import { WasmFactory } from '@lightprotocol/hasher.rs';

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

    const { merkleTree } = defaultTestStateTreeAccounts();

    beforeAll(async () => {
        const lightWasm = await WasmFactory.getInstance();
        rpc = await getTestRpc(lightWasm);
        payer = await newAccountWithLamports(rpc);
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
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            defaultTestStateTreeAccounts().merkleTree,
        );

        await assertMintTo(rpc, mint, amount, bob.publicKey);

        /// wrong authority
        await expect(
            mintTo(rpc, payer, mint, bob.publicKey, Keypair.generate(), amount),
        ).rejects.toThrowError(/custom program error: 0x1782/);

        /// with output state merkle tree defined
        await mintTo(
            rpc,
            payer,
            mint,
            bob.publicKey,
            mintAuthority,
            amount,
            defaultTestStateTreeAccounts().merkleTree,
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
            defaultTestStateTreeAccounts().merkleTree,
        );

        /// Mint to 10 recipients
        const tx = await mintTo(
            rpc,
            payer,
            mint,
            recipients.slice(0, 10),
            mintAuthority,
            amounts.slice(0, 10),
            defaultTestStateTreeAccounts().merkleTree,
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
                defaultTestStateTreeAccounts().merkleTree,
            ),
        ).rejects.toThrowError(
            /Amount and toPubkey arrays must have the same length/,
        );
    });

    it(`should mint to ${recipients.length} recipients optimized with LUT`, async () => {
        const lookupTableAccount = (await rpc.getAddressLookupTable(lut))
            .value!;

        const ix = await CompressedTokenProgram.mintTo({
            feePayer: payer.publicKey,
            mint,
            authority: mintAuthority.publicKey,
            amount: amounts,
            toPubkey: recipients,
            merkleTree: defaultTestStateTreeAccounts().merkleTree,
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

        return await sendAndConfirmTx(rpc, tx);
    });
});
