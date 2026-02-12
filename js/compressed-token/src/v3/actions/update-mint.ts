import {
    ComputeBudgetProgram,
    ConfirmOptions,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    sendAndConfirmTx,
    DerivationMode,
    bn,
    CTOKEN_PROGRAM_ID,
    assertBetaEnabled,
} from '@lightprotocol/stateless.js';
import {
    createUpdateMintAuthorityInstruction,
    createUpdateFreezeAuthorityInstruction,
} from '../instructions/update-mint';
import { getMintInterface } from '../get-mint-interface';

/**
 * Update the mint authority of a compressed token mint.
 * Works for both compressed and decompressed mints.
 *
 * @param rpc                    RPC connection
 * @param payer                  Fee payer (signer)
 * @param mint                   Mint address
 * @param currentMintAuthority   Current mint authority (signer)
 * @param newMintAuthority       New mint authority (or null to revoke)
 * @param confirmOptions         Optional confirm options
 */
export async function updateMintAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    currentMintAuthority: Signer,
    newMintAuthority: PublicKey | null,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInterface.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
    }

    // When mint is decompressed, no validity proof needed - program reads from CMint account
    const isDecompressed =
        mintInterface.mintContext?.cmintDecompressed ?? false;
    const validityProof = isDecompressed
        ? null
        : await rpc.getValidityProofV2(
              [
                  {
                      hash: bn(mintInterface.merkleContext.hash),
                      leafIndex: mintInterface.merkleContext.leafIndex,
                      treeInfo: mintInterface.merkleContext.treeInfo,
                      proveByIndex: mintInterface.merkleContext.proveByIndex,
                  },
              ],
              [],
              DerivationMode.compressible,
          );

    const ix = createUpdateMintAuthorityInstruction(
        mintInterface,
        currentMintAuthority.publicKey,
        newMintAuthority,
        payer.publicKey,
        validityProof,
    );

    const additionalSigners = currentMintAuthority.publicKey.equals(
        payer.publicKey,
    )
        ? []
        : [currentMintAuthority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}

/**
 * Update the freeze authority of a compressed token mint.
 * Works for both compressed and decompressed mints.
 *
 * @param rpc                      RPC connection
 * @param payer                    Fee payer (signer)
 * @param mint                     Mint address
 * @param currentFreezeAuthority   Current freeze authority (signer)
 * @param newFreezeAuthority       New freeze authority (or null to revoke)
 * @param confirmOptions           Optional confirm options
 */
export async function updateFreezeAuthority(
    rpc: Rpc,
    payer: Signer,
    mint: PublicKey,
    currentFreezeAuthority: Signer,
    newFreezeAuthority: PublicKey | null,
    confirmOptions?: ConfirmOptions,
): Promise<TransactionSignature> {
    assertBetaEnabled();

    const mintInterface = await getMintInterface(
        rpc,
        mint,
        confirmOptions?.commitment,
        CTOKEN_PROGRAM_ID,
    );

    if (!mintInterface.merkleContext) {
        throw new Error('Mint does not have MerkleContext');
    }

    // When mint is decompressed, no validity proof needed - program reads from CMint account
    const isDecompressed =
        mintInterface.mintContext?.cmintDecompressed ?? false;
    const validityProof = isDecompressed
        ? null
        : await rpc.getValidityProofV2(
              [
                  {
                      hash: bn(mintInterface.merkleContext.hash),
                      leafIndex: mintInterface.merkleContext.leafIndex,
                      treeInfo: mintInterface.merkleContext.treeInfo,
                      proveByIndex: mintInterface.merkleContext.proveByIndex,
                  },
              ],
              [],
              DerivationMode.compressible,
          );

    const ix = createUpdateFreezeAuthorityInstruction(
        mintInterface,
        currentFreezeAuthority.publicKey,
        newFreezeAuthority,
        payer.publicKey,
        validityProof,
    );

    const additionalSigners = currentFreezeAuthority.publicKey.equals(
        payer.publicKey,
    )
        ? []
        : [currentFreezeAuthority];

    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );

    return await sendAndConfirmTx(rpc, tx, confirmOptions);
}
