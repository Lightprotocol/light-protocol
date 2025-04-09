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
    selectStateTreeInfo,
    StateTreeInfo,
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
 * @param rpc                   Rpc to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint of the compressed token
 * @param amount                Number of tokens to transfer
 * @param owner                 Owner of the compressed tokens
 * @param toAddress             Destination **uncompressed** (associated) token
 *                              account address.
 * @param outputStateTreeInfo   State tree account that any change compressed
 *                              tokens should be inserted into. Defaults to a
 *                              default state tree account.
 * @param tokenPoolInfos        Token pool infos
 * @param confirmOptions        Options for confirming the transaction

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
    outputStateTreeInfo?: StateTreeInfo,
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

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getCachedActiveStateTreeInfos());

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
        outputStateTreeInfo,
        tokenPoolInfos: selectedTokenPoolInfos,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [owner]);
    const signedTx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    return await sendAndConfirmTx(rpc, signedTx, confirmOptions);
}
