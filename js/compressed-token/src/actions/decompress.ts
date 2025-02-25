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
    StateTreeContext,
    pickRandomStateTreeContext,
} from '@lightprotocol/stateless.js';

import BN from 'bn.js';

import { CompressedTokenProgram } from '../program';
import { selectMinCompressedTokenAccountsForTransfer } from '../utils';

/**
 * Decompress compressed tokens
 *
 * @param rpc                       Rpc to use
 * @param payer                     Payer of the transaction fees
 * @param mint                      Mint of the compressed token
 * @param amount                    Number of tokens to transfer
 * @param owner                     Owner of the compressed tokens
 * @param toAddress                 Destination **uncompressed** (associated) token account
 *                                  address.
 * @param outputStateTreeContext    State tree context that any changes to
 *                                  compressed tokens should be inserted into.
 *                                  Defaults to the default state tree context.
 * @param confirmOptions            Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    outputStateTreeContext?: StateTreeContext,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    amount = bn(amount);

    if (!outputStateTreeContext) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        outputStateTreeContext = pickRandomStateTreeContext(stateTreeInfo);
    }

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        {
            mint,
        },
    );

    /// TODO: consider using a different selection algorithm
    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ix = await CompressedTokenProgram.decompress({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress, // TODO: add explicit check that it is a token account
        amount,
        outputStateTreeContext: outputStateTreeContext,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(rpc, signedTx, confirmOptions);
    return txId;
}
