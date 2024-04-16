import {
    ConfirmOptions,
    Keypair,
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
 * Create and initialize a new compressed token mint
 *
 * @param rpc             RPC to use
 * @param payer           Payer of the transaction and initialization fees
 * @param mintAuthority   Account or multisig that will control minting. Is signer.
 * @param decimals        Location of the decimal place
 * @param keypair         Optional keypair, defaulting to a new random one
 * @param confirmOptions  Options for confirming the transaction
 *
 * @return Address of the new mint and the transaction signature
 */
export async function registerMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: Signer,
    decimals: number,
    keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    const ixs = await CompressedTokenProgram.registerMint({
        feePayer: payer.publicKey,
        mint: keypair.publicKey,
        decimals,
        authority: mintAuthority.publicKey,
        freezeAuthority: null, // TODO: add feature
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [mintAuthority, keypair]);

    const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { mint: keypair.publicKey, transactionSignature: txId };
}
