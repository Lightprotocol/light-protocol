import {
    ComputeBudgetProgram,
    ConfirmOptions,
    Keypair,
    PublicKey,
    Signer,
    TransactionSignature,
} from '@solana/web3.js';
import {
    Rpc,
    buildAndSignTx,
    dedupeSigner,
    sendAndConfirmTx,
    TreeInfo,
    AddressTreeInfo,
    selectStateTreeInfo,
    getDefaultAddressTreeInfo,
    DerivationMode,
} from '@lightprotocol/stateless.js';
import {
    createMintInstruction,
    TokenMetadataInstructionData,
} from '../instructions/create-mint';
import { findMintAddress } from '../../compressible';

export async function createMint(
    rpc: Rpc,
    payer: Signer,
    mintAuthority: Signer,
    freezeAuthority: null | PublicKey,
    decimals: number,
    keypair: Keypair = Keypair.generate(),
    metadata?: TokenMetadataInstructionData,
    addressTreeInfo?: AddressTreeInfo,
    outputStateTreeInfo?: TreeInfo,
    confirmOptions?: ConfirmOptions,
): Promise<{ mint: PublicKey; transactionSignature: TransactionSignature }> {
    addressTreeInfo = addressTreeInfo ?? getDefaultAddressTreeInfo();
    outputStateTreeInfo =
        outputStateTreeInfo ??
        selectStateTreeInfo(await rpc.getStateTreeInfos());

    const validityProof = await rpc.getValidityProofV2(
        [],
        [
            {
                address: findMintAddress(keypair.publicKey)[0].toBytes(),
                treeInfo: addressTreeInfo,
            },
        ],
        DerivationMode.compressible,
    );

    const ix = createMintInstruction(
        keypair.publicKey,
        decimals,
        mintAuthority.publicKey,
        freezeAuthority,
        payer.publicKey,
        validityProof,
        metadata,
        addressTreeInfo,
        outputStateTreeInfo,
    );

    const additionalSigners = dedupeSigner(payer, [keypair, mintAuthority]);
    const { blockhash } = await rpc.getLatestBlockhash();
    const tx = buildAndSignTx(
        [ComputeBudgetProgram.setComputeUnitLimit({ units: 500_000 }), ix],
        payer,
        blockhash,
        additionalSigners,
    );
    const txId = await sendAndConfirmTx(rpc, tx, confirmOptions);

    const mint = findMintAddress(keypair.publicKey);
    return { mint: mint[0], transactionSignature: txId };
}
