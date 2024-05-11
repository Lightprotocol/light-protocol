import {PublicKey, Signer, Keypair} from '@solana/web3.js';
import {airdropSol, createRpc, bn, compress, transfer, Rpc} from '@lightprotocol/stateless.js';

const payerKeypair = [
    46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249,
    108, 80, 144, 32, 168, 245, 161, 146, 92, 180, 79,
    231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130,
    60, 106, 251, 24, 224, 192, 108, 70, 59, 111, 251,
    186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158,
    111, 163, 28, 157, 47, 201, 41, 249, 59
]

const bobKeypair = [
    125, 14, 244, 185, 193, 42, 156, 191, 212, 42, 239,
    56, 169, 240, 239, 52, 95, 215, 240, 86, 151, 212,
    245, 230, 198, 148, 12, 230, 83, 57, 56, 244, 191,
    129, 151, 233, 233, 129, 21, 255, 101, 163, 48, 212,
    218, 82, 134, 36, 29, 185, 30, 215, 183, 242, 244,
    222, 8, 10, 158, 214, 99, 237, 126, 9
]

const LAMPORTS = 1e11;
const COMPRESS_AMOUNT = 1e9;
const TOTAL_NUMBER_OF_TRANSFERS = 10;
const NUMBER_OF_CONCURRENT_TRANSFERS = 2;
const TRANSFER_AMOUNT = 10;

async function transferAsync(i: number, rpc: Rpc, payer: Signer, bobPublicKey: PublicKey): Promise<void> {
    const transferSig = await transfer(rpc, payer, TRANSFER_AMOUNT, payer, bobPublicKey);
    console.log(`transfer ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${transferSig}`);
}

async function prefillNullifierQueue() {
    const rpc = createRpc();
    const payer = Keypair.fromSecretKey(Uint8Array.from(payerKeypair));
    const bob = Keypair.fromSecretKey(Uint8Array.from(bobKeypair));

    await airdropSol({connection: rpc, lamports: LAMPORTS, recipientPublicKey: payer.publicKey});
    await airdropSol({connection: rpc, lamports: LAMPORTS, recipientPublicKey: bob.publicKey});

    const compressSig = await compress(rpc, payer, COMPRESS_AMOUNT, payer.publicKey);
    console.log('compress tx sig', compressSig);

    for (let i = 0; i < TOTAL_NUMBER_OF_TRANSFERS; i += NUMBER_OF_CONCURRENT_TRANSFERS) {
        const transferPromises = Array.from(
            {length: NUMBER_OF_CONCURRENT_TRANSFERS},
            (_, j) => transferAsync(i + j, rpc, payer, bob.publicKey)
        );
        await Promise.all(transferPromises);
        // await new Promise(resolve => setTimeout(resolve, 50));
    }
}

prefillNullifierQueue().then(() => {
    console.log('Transfer completed.');
}).catch((error) => {
    console.error('An error occurred:', error);
});



