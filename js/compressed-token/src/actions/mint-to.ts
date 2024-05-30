import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';

/**
 * Mint compressed tokens to a solana address
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint for the account
 * @param destination    Address of the account to mint to
 * @param authority      Minting authority
 * @param amount         Amount to mint
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       part of. Defaults to the default state tree account.
 * @param confirmOptions Options for confirming the transaction
 *
 * @return Signature of the confirmed transaction
 */
export async function mintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer,
    amount: number | BN,
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const additionalSigners = dedupeSigner(payer, [authority]);

    const ix = await CompressedTokenProgram.mintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        amount: amount,
        toPubkey: destination,
        merkleTree,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
