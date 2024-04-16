import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { BN } from '@coral-xyz/anchor';
import {
    defaultTestStateTreeAccounts,
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';
import { getOrCreateAssociatedTokenAccount } from '@solana/spl-token';

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
export async function approveAndMintTo(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    destination: PublicKey,
    authority: Signer,
    amount: number | BN,
    merkleTree: PublicKey = defaultTestStateTreeAccounts().merkleTree, // DEFAULT IF NOT PROVIDED
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    const authorityTokenAccount = await getOrCreateAssociatedTokenAccount(
        rpc,
        payer,
        authority.publicKey,
        mint,
    );

    const ixs = await CompressedTokenProgram.approveAndMintTo({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        authorityTokenAccount: authorityTokenAccount.address,
        amount,
        toPubkey: destination,
        merkleTree,
    });

    const { blockhash } = await rpc.getLatestBlockhash();
    const additionalSigners = dedupeSigner(payer, [authority]);

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({ units: 1_000_000 }),
            ...ixs,
        ],
        payer,
        blockhash,
        additionalSigners,
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
