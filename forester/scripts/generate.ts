import {PublicKey, Signer, Keypair, LAMPORTS_PER_SOL} from '@solana/web3.js';
import {
    airdropSol,
    createRpc,
    compress,
    transfer,
    Rpc,
    LightSystemProgram, createAccount
} from '@lightprotocol/stateless.js';
import { randomBytes } from 'tweetnacl';
import * as dotenv from "dotenv";
dotenv.config();

const RPC_API_KEY = process.env.PHOTON_API_KEY;
console.log("RPC_API_KEY: ", RPC_API_KEY);

const LAMPORTS = 0.9 * LAMPORTS_PER_SOL;
const COMPRESS_AMOUNT = 0.1 * LAMPORTS_PER_SOL;
const TOTAL_TX = 10;
const CONCURRENT_TX = 1;
const TRANSFER_AMOUNT = 10;

const aliceKeypair = [
    46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245,
    161, 146, 92, 180, 79, 231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60,
    106, 251, 24, 224, 192, 108, 70, 59, 111, 251, 186, 50, 23, 103, 106, 233,
    113, 148, 57, 190, 158, 111, 163, 28, 157, 47, 201, 41, 249, 59,
];

const bobKeypair = [
    125, 14, 244, 185, 193, 42, 156, 191, 212, 42, 239, 56, 169, 240, 239, 52, 95,
    215, 240, 86, 151, 212, 245, 230, 198, 148, 12, 230, 83, 57, 56, 244, 191,
    129, 151, 233, 233, 129, 21, 255, 101, 163, 48, 212, 218, 82, 134, 36, 29,
    185, 30, 215, 183, 242, 244, 222, 8, 10, 158, 214, 99, 237, 126, 9,
];


function generateKeypairs(count: number): Keypair[] {
    const keypairs = [];
    for (let i = 0; i < count; i++) {
        keypairs.push(Keypair.generate());
    }
    return keypairs;
}

const payerKeypairs = [Keypair.fromSecretKey(Uint8Array.from(aliceKeypair))]; //generateKeypairs(NUMBER_OF_CONCURRENT_TRANSFERS);
const receiverKeypairs = [Keypair.fromSecretKey(Uint8Array.from(bobKeypair))]; //generateKeypairs(NUMBER_OF_CONCURRENT_TRANSFERS);

async function transferAsync(i: number, rpc: Rpc, payer: Signer, receiverPublicKey: PublicKey): Promise<void> {
    const transferSig = await transfer(rpc, payer, TRANSFER_AMOUNT, payer, receiverPublicKey);
    console.log(`transfer ${i} of ${TOTAL_TX}: ${transferSig}`);
}

async function createAccountAsync(i: number, rpc: Rpc, payer: Signer): Promise<void> {
    const seed = new Uint8Array(randomBytes(32));
    const tx = await createAccount(
        rpc,
        payer,
        seed,
        // TRANSFER_AMOUNT,
        LightSystemProgram.programId,
    );
    console.log(`create account ${i} of ${TOTAL_TX}: ${tx}`);
}

function localRpc(): Rpc {
    let validatorUrl = 'http://0.0.0.0:8899';
    let photonUrl = 'http://0.0.0.0:8784';
    let proverUrl = 'http://0.0.0.0:3001';
    return createRpc(validatorUrl, photonUrl, proverUrl);
}

function devnetRpc(): Rpc {
    const url = `https://devnet.helius-rpc.com?api-key=${RPC_API_KEY}`;
    return createRpc(url, url);
}

async function prefillNullifierQueue() {
    const rpc = devnetRpc();


    const isAirdropNeeded = false;
    if (isAirdropNeeded) {
        await Promise.all([
            ...payerKeypairs.map(async payer => await airdropSol({
                connection: rpc,
                lamports: LAMPORTS,
                recipientPublicKey: payer.publicKey
            })),
            ...receiverKeypairs.map(async receiver => await airdropSol({
                connection: rpc,
                lamports: LAMPORTS,
                recipientPublicKey: receiver.publicKey
            }))
        ]);
    }

    await Promise.all(
        payerKeypairs.map(async (payer) => {
            const balance = await rpc.getBalance(payer.publicKey);
            console.log(`Payer ${payer.publicKey.toBase58()} balance:`, balance);
        })
    );

    const isCompressNeeded = false;
    if (isCompressNeeded) {
        await Promise.all(
            payerKeypairs.map(async (payer) => {
                const compressSig = await compress(rpc, payer, COMPRESS_AMOUNT, payer.publicKey);
                console.log(`Compress tx sig for payer ${payer.publicKey.toBase58()}:`, compressSig);
            })
        );
    }

    const isTransferNeeded = false;
    if (isTransferNeeded) {
        for (let i = 0; i < TOTAL_TX; i += CONCURRENT_TX) {
            const promises = [];
            for (let j = 0; j < CONCURRENT_TX; j++) {
                promises.push(transferAsync(i + j, rpc, payerKeypairs[j], receiverKeypairs[j].publicKey));
            }
            await Promise.all(promises);
        }
    }

    const isCreateAccountNeeded = true;
    if (isCreateAccountNeeded) {
        for (let i = 0; i < TOTAL_TX; i += CONCURRENT_TX) {
            const promises = [];
            for (let j = 0; j < CONCURRENT_TX; j++) {
                promises.push(createAccountAsync(i + j, rpc, payerKeypairs[j]));
            }
            await Promise.all(promises);
        }
    }

}

prefillNullifierQueue().then(() => {
    console.log('Transfer completed.');
}).catch((error) => {
    console.error('An error occurred:', error);
});
