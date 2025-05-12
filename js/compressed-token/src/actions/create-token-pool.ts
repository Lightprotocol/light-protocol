import {
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionInstruction,
    TransactionSignature,
} from '@solana/web3.js';
import { CompressedTokenProgram } from '../program';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
} from '@lightprotocol/stateless.js';
import { getTokenPoolInfos } from '../utils/get-token-pool-infos';

/**
 * Register an existing mint with the CompressedToken program
 *
 * @param rpc             RPC connection to use
 * @param payer           Fee payer
 * @param mint            SPL Mint address
 * @param confirmOptions  Options for confirming the transaction
 * @param tokenProgramId  Optional: Address of the token program. Default:
 *                        TOKEN_PROGRAM_ID
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
        : await CompressedTokenProgram.getMintProgramId(mint, rpc);

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

/**
 * Create additional token pools for an existing mint
 *
 * @param rpc                   RPC connection to use
 * @param payer                 Fee payer
 * @param mint                  SPL Mint address
 * @param numMaxAdditionalPools Number of additional token pools to create. Max
 *                              3.
 * @param confirmOptions        Optional: Options for confirming the transaction
 * @param tokenProgramId        Optional: Address of the token program. Default:
 *                              TOKEN_PROGRAM_ID
 *
 * @return transaction signature
 */
export async function addTokenPools(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    numMaxAdditionalPools: number,
    confirmOptions?: ConfirmOptions,
    tokenProgramId?: PublicKey,
) {
    tokenProgramId = tokenProgramId
        ? tokenProgramId
        : await CompressedTokenProgram.getMintProgramId(mint, rpc);
    const instructions: TransactionInstruction[] = [];

    const infos = (await getTokenPoolInfos(rpc, mint)).slice(0, 4);

    // Get indices of uninitialized pools
    const uninitializedIndices = [];
    for (let i = 0; i < infos.length; i++) {
        if (!infos[i].isInitialized) {
            uninitializedIndices.push(i);
        }
    }

    // Create instructions for requested number of pools
    for (let i = 0; i < numMaxAdditionalPools; i++) {
        if (i >= uninitializedIndices.length) {
            break;
        }

        instructions.push(
            await CompressedTokenProgram.addTokenPool({
                mint,
                feePayer: payer.publicKey,
                tokenProgramId,
                poolIndex: uninitializedIndices[i],
            }),
        );
    }
    const { blockhash } = await rpc.getLatestBlockhash();

    const tx = buildAndSignTx(instructions, payer, blockhash);

    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    return txId;
}
