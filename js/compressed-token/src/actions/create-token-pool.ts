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
export async function createTokenPool(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    const ix = await CompressedTokenProgram.createTokenPool({
        feePayer: payer.publicKey,
        mint,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx([ix], payer, blockhash);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
