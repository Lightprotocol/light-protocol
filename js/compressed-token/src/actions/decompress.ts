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
} from '@lightprotocol/stateless.js';
import BN from 'bn.js';
import { CompressedTokenProgram } from '../program';
import { selectMinCompressedTokenAccountsForTransfer } from '../utils';
import {
    selectTokenPoolInfosForDecompression,
    TokenPoolInfo,
} from '../utils/get-token-pool-infos';
import { getTokenPoolInfos } from '../utils/get-token-pool-infos';

/**
 * Decompress compressed tokens
 *
 * @param rpc                   Rpc connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens
 * @param toAddress             Destination **uncompressed** token account
 *                              address. (ATA)
 * @param tokenPoolInfos        Optional: Token pool infos.
 * @param confirmOptions        Options for confirming the transaction
 *
 * @return confirmed transaction signature
 */
export async function decompress(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    tokenPoolInfos?: TokenPoolInfo[],
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    amount = bn(amount);

    const compressedTokenAccounts = await rpc.getCompressedTokenAccountsByOwner(
        owner.publicKey,
        {
            mint,
        },
    );

    const [inputAccounts] = selectMinCompressedTokenAccountsForTransfer(
        compressedTokenAccounts.items,
        amount,
    );

    const proof = await rpc.getValidityProofV0(
        inputAccounts.map(account => ({
            hash: account.compressedAccount.hash,
            tree: account.compressedAccount.treeInfo.tree,
            queue: account.compressedAccount.treeInfo.queue,
        })),
    );

    tokenPoolInfos = tokenPoolInfos ?? (await getTokenPoolInfos(rpc, mint));

    const selectedTokenPoolInfos = selectTokenPoolInfosForDecompression(
        tokenPoolInfos,
        amount,
    );

    const ix = await CompressedTokenProgram.decompress({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        tokenPoolInfos: selectedTokenPoolInfos,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 350_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    return await sendAndConfirmTx(rpc, signedTx, confirmOptions);
}
