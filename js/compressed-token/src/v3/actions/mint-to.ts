import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import { MAX_TOP_UP } from '../../constants';
import { createMintToInstruction } from '../instructions/mint-to';

/**
 * Mint tokens to a light-token account.
 *
 * This is a simple mint instruction for minting to light-token associated token accounts (hot).
 * The light mint account must exist (mint must be decompressed first).
 *
 * @param rpc - RPC connection
 * @param payer - Fee payer (signer)
 * @param mint - Mint address (light mint account)
 * @param destination - Destination light-token account
 * @param authority - Mint authority (signer)
 * @param amount - Amount to mint
 * @param maxTopUp - Optional maximum lamports for rent top-up
 * @param confirmOptions - Optional confirm options
 * @returns Transaction signature
 */
export async function mintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer,
    amount: number | bigint,
    maxTopUp?: number,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    // Use payer as fee payer for top-ups if authority is different from payer
    const feePayer = authority.publicKey.equals(payer.publicKey)
        ? undefined
        : payer.publicKey;

    const ix = createMintToInstruction({
        mint,
        destination,
        amount,
        authority: authority.publicKey,
        maxTopUp: maxTopUp ?? MAX_TOP_UP,
        feePayer,
    });

    const additionalSigners = authority.publicKey.equals(payer.publicKey)
        ? []
        : [authority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 200_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
