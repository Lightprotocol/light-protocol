import {PublicKey, Signer, Keypair} from '@solana/web3.js';
import {airdropSol, createRpc, compress, transfer, Rpc} from '@lightprotocol/stateless.js';

const LAMPORTS = 1e13;
const COMPRESS_AMOUNT = 1e9;
const TOTAL_NUMBER_OF_TRANSFERS = 1e3;
const NUMBER_OF_CONCURRENT_TRANSFERS = 18;
const TRANSFER_AMOUNT = 10;

const payerKeypairs = generateKeypairs(NUMBER_OF_CONCURRENT_TRANSFERS);
const receiverKeypairs = generateKeypairs(NUMBER_OF_CONCURRENT_TRANSFERS);

async function transferAsync(i: number, rpc: Rpc, payer: Signer, bobPublicKey: PublicKey): Promise<void> {
    const transferSig = await transfer(rpc, payer, TRANSFER_AMOUNT, payer, bobPublicKey);
    console.log(`transfer ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${transferSig}`);
}

async function prefillNullifierQueue() {
    const rpc = createRpc();

    await Promise.all([
        ...payerKeypairs.map(async payer => await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: payer.publicKey })),
        ...receiverKeypairs.map(async receiver => await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: receiver.publicKey }))
    ]);

    await Promise.all(
        payerKeypairs.map(async (payer) => {
            const balance = await rpc.getBalance(payer.publicKey);
            console.log(`Payer ${payer.publicKey.toBase58()} balance:`, balance);
        })
    );

    await Promise.all(
        payerKeypairs.map(async (payer) => {
            const compressSig = await compress(rpc, payer, COMPRESS_AMOUNT, payer.publicKey);
            console.log(`Compress tx sig for payer ${payer.publicKey.toBase58()}:`, compressSig);
        })
    );

    for (let i = 0; i < TOTAL_NUMBER_OF_TRANSFERS; i += NUMBER_OF_CONCURRENT_TRANSFERS) {
        const transferPromises = [];
        for (let j = 0; j < NUMBER_OF_CONCURRENT_TRANSFERS; j++) {
            transferPromises.push(transferAsync(i + j, rpc, payerKeypairs[j], receiverKeypairs[j].publicKey));
        }
        await Promise.all(transferPromises);
    }

}

function generateKeypairs(count: number): Keypair[] {
    const keypairs = [];
    for (let i = 0; i < count; i++) {
        keypairs.push(Keypair.generate());
    }
    return keypairs;
}


prefillNullifierQueue().then(() => {
    console.log('Transfer completed.');
}).catch((error) => {
    console.error('An error occurred:', error);
});
