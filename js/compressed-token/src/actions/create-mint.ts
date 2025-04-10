import {
    ConfirmOptions,
    Keypair,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
    MINT_SIZE,
    TOKEN_2022_PROGRAM_ID,
    TOKEN_PROGRAM_ID,
} from '@solana/spl-token';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    dedupeSigner,
} from '@lightprotocol/stateless.js';

/**
 * Create and initialize a new compressed token mint
 *
 * @param rpc               RPC to use
 * @param payer             Payer of the transaction and initialization fees
 * @param mintAuthority     Account or multisig that will control minting
 * @param decimals          Location of the decimal place
 * @param keypair           Optional keypair, defaulting to a new random one
 * @param confirmOptions    Options for confirming the transaction
 * @param tokenProgramId    Program ID for the token. Defaults to
 *                          TOKEN_PROGRAM_ID. You can pass in a boolean to
 *                          automatically resolve to TOKEN_2022_PROGRAM_ID if
 *                          true, or TOKEN_PROGRAM_ID if false.
 * @param freezeAuthority   Account that will control freeze and thaw. Defaults
 *                          to null.
 *
 * @return Address of the new mint and the transaction signature
 */
export async function createMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: PublicKey,
    decimals: number,
    keypair = Keypair.generate(),
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey | boolean,
    freezeAuthority?: PublicKey,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    const rentExemptBalance =
        await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

    // If true, uses TOKEN_2022_PROGRAM_ID.
    // If false or undefined, defaults to TOKEN_PROGRAM_ID.
    // Otherwise, uses the provided tokenProgramId.
    const resolvedTokenProgramId =
        tokenProgramId === true
            ? TOKEN_2022_PROGRAM_ID
            : tokenProgramId || TOKEN_PROGRAM_ID;

    const ixs = await CompressedTokenProgram.createMint({
        feePayer: payer.publicKey,
        mint: keypair.publicKey,
        decimals,
        authority: mintAuthority,
        freezeAuthority: freezeAuthority || null,
        rentExemptBalance,
        tokenProgramId: resolvedTokenProgramId,
    });

    const { blockhash } = await rpc.getLatestBlockhash();

    const additionalSigners = dedupeSigner(payer, [keypair]);

    const tx = buildAndSignTx(ixs, payer, blockhash, additionalSigners);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return { mint: keypair.publicKey, transactionSignature: txId };
}
