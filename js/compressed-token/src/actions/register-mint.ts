import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

/**
 * Register an existing mint with the CompressedToken program
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param mintAuthority   Account or multisig that will control minting. Is signer.
 * @param mintAddress     Address of the existing mint
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return transaction signature
 */
export async function registerMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: Signer,
    mintAddress: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const ixs = await CompressedTokenProgram.registerMint({
        feePayer: payer.publicKey,
        mint: mintAddress,
        authority: mintAuthority.publicKey,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [mintAuthority]);
    console.log('additionalSigners', additionalSigners);
    const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
