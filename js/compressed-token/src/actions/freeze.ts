import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    bn,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
    ParsedTokenAccount,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram, parseTokenData } from '../program';

/**
 * Freeze compressed token accounts
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param accounts       Compressed token accounts to freeze
 * @param mint           Mint of the compressed token
 * @param authority      Authority of the compressed token account
 * @param merkleTree     State tree account that any change compressed tokens should be
 *                       inserted into.
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function freeze(
    rpc: Rpc,
    payer: Signer,
    accounts: ParsedTokenAccount[],
    mint: PublicKey,
    freezeAuthority: Signer,
    merkleTree: PublicKey,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const proof = await rpc.getValidityProof(
        accounts.map(account => bn(account.compressedAccount.hash)),
    );
    console.log('proof', proof);

    const ix = await CompressedTokenProgram.freeze({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: accounts,
        freezeAuthority: freezeAuthority.publicKey,
        outputStateTree: merkleTree,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        mint,
    });

    console.log('ix', ix);

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [freezeAuthority]);

    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, signedTx, confirmOptions);
}
