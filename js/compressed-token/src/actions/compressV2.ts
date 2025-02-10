import {
    AddressLookupTableAccount,
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import BN from 'bn.js';
import {
    sendAndConfirmTx,
    buildAndSignTx,
    Rpc,
    dedupeSigner,
    pickRandomTreeAndQueue,
} from '@lightprotocol/stateless.js';
import { CompressedTokenProgram } from '../program';

/**
 * Distribute SPL tokens to multiple recipients as compressed tokens
 *
 * @param rpc                   Rpc to use
 * @param payer                 Payer of the transaction fees
 * @param mint                  Mint for the account
 * @param recipients            Addresses to mint to.
 * @param authority             Minting authority
 * @param sourceTokenAccount    Source token account to compress tokens from.
 * @param amount                Amount per recipient.
 * @param merkleTree            State tree account that the compressed tokens
 *                              should be part of. Defaults to the default state
 *                              tree account.
 * @param confirmOptions        Options for confirming the transaction
 * @param tokenProgramId        Optional: The token program ID. Default: SPL
 *                              Token Program ID
 * @return                      Signature of the confirmed transaction
 */
export async function compressV2(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    recipients: PublicKey[],
    authority: Signer,
    sourceTokenAccount: PublicKey,
    amount: number | BN,
    merkleTree?: PublicKey,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
    lookupTable?: PublicKey,
): Promise<TransactionSignature> {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.get_mint_program_id(mint, rpc);

    const additionalSigners = dedupeSigner(payer, [authority]);

    if (!merkleTree) {
        const stateTreeInfo = await rpc.getCachedActiveStateTreeInfo();
        const { tree } = pickRandomTreeAndQueue(stateTreeInfo);
        merkleTree = tree;
    }

    let lookupTableAccountValue: AddressLookupTableAccount | null = null;
    if (lookupTable) {
        lookupTableAccountValue = (await rpc.getAddressLookupTable(lookupTable))
            .value;
        if (!lookupTableAccountValue) {
            throw new Error('Lookup table not found');
        }
    }

    const ix = await CompressedTokenProgram.compressV2({
        feePayer: payer.publicKey,
        mint,
        authority: authority.publicKey,
        sourceTokenAccount,
        amount: amount,
        recipients,
        merkleTree,
        tokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(
        [
            ComputeBudgetProgram.setComputeUnitLimit({
                units:
                    recipients.length <= 10
                        ? 200_000
                        : recipients.length <= 15
                          ? 250_000
                          : recipients.length <= 20
                            ? 300_000
                            : 350_000,
            }),
            ix,
        ],
        payer,
        blockhash,
        additionalSigners,
        lookupTableAccountValue ? [lookupTableAccountValue] : [],
    );

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
