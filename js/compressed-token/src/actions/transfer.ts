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
    ParsedTokenAccount,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

import { BN } from '@coral-xyz/anchor';

import { CompressedTokenProgram } from '../program';

/**
 * Transfer compressed tokens from one owner to another
 *
 * @param rpc            Rpc to use
 * @param payer          Payer of the transaction fees
 * @param mint           Mint of the compressed token
 * @param amount         Number of tokens to transfer
 * @param owner          Owner of the compressed tokens
 * @param toAddress      Destination address of the recipient
 * @param merkleTree     State tree account that the compressed tokens should be
 *                       inserted into. Defaults to the default state tree
 *                       account.
 * @param confirmOptions Options for confirming the transaction
 *
 *
 * @return Signature of the confirmed transaction
 */
export async function transfer(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    amount: number | BN,
    owner: Signer,
    toAddress: PublicKey,
    /// TODO: allow multiple
    merkleTree?: PublicKey,
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
        compressedTokenAccounts,
        amount,
    );

    const proof = await rpc.getValidityProof(
        inputAccounts.map(account => bn(account.compressedAccount.hash)),
    );

    const ix = await CompressedTokenProgram.transfer({
        payer: payer.publicKey,
        inputCompressedTokenAccounts: inputAccounts,
        toAddress,
        amount,
        recentInputStateRootIndices: proof.rootIndices,
        recentValidityProof: proof.compressedProof,
        outputStateTrees: merkleTree,
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

/**
 * Selects the minimal number of compressed token accounts for a transfer.
 *
 * 1. Sorts the accounts by amount in descending order
 * 2. Accumulates the amount until it is greater than or equal to the transfer
 *    amount
 */
export function selectMinCompressedTokenAccountsForTransfer(
    accounts: ParsedTokenAccount[],
    transferAmount: BN,
): [
    selectedAccounts: ParsedTokenAccount[],
    total: BN,
    totalLamports: BN | null,
] {
    let accumulatedAmount = bn(0);
    let accumulatedLamports = bn(0);

    const selectedAccounts: ParsedTokenAccount[] = [];

    accounts.sort((a, b) => b.parsed.amount.cmp(a.parsed.amount));

    for (const account of accounts) {
        if (accumulatedAmount.gte(bn(transferAmount))) break;
        accumulatedAmount = accumulatedAmount.add(account.parsed.amount);
        accumulatedLamports = accumulatedLamports.add(
            account.compressedAccount.lamports,
        );
        selectedAccounts.push(account);
    }

    if (accumulatedAmount.lt(bn(transferAmount))) {
        throw new Error(
            `Not enough balance for transfer. Required: ${transferAmount.toString()}, available: ${accumulatedAmount.toString()}`,
        );
    }

    return [
        selectedAccounts,
        accumulatedAmount,
        accumulatedLamports.lt(bn(0)) ? accumulatedLamports : null,
    ];
}
