import {PublicKey, Signer, Keypair, LAMPORTS_PER_SOL} from '@solana/web3.js';
import {
    createRpc,
    compress,
    transfer,
    Rpc,
    LightSystemProgram,
    createAccount
} from '@lightprotocol/stateless.js';
import { randomBytes } from 'tweetnacl';
import * as dotenv from "dotenv";
import { BN } from '@coral-xyz/anchor';
dotenv.config();

const RPC_API_KEY = process.env.RPC_API_KEY;
console.log("RPC_API_KEY: ", RPC_API_KEY);

const COMPRESS_AMOUNT = LAMPORTS_PER_SOL * 0.1;
const TRANSFER_AMOUNT = 10;
const MIN_BALANCE_FOR_COMPRESS = COMPRESS_AMOUNT + 0.01 * LAMPORTS_PER_SOL;

const aliceKeypair = new Keypair();

const bobKeypair = new Keypair();

async function getBalances(rpc: Rpc, keypair: Keypair): Promise<{ compressed: number, uncompressed: number }> {
    const compressedBalance = await rpc.getCompressedBalanceByOwner(keypair.publicKey);
    const uncompressedBalance = await rpc.getBalance(keypair.publicKey);
    return {
        compressed: compressedBalance.toNumber(),
        uncompressed: uncompressedBalance,
    };
}

async function printInitialBalances(rpc: Rpc): Promise<void> {
    const aliceBalances = await getBalances(rpc, aliceKeypair);
    const bobBalances = await getBalances(rpc, bobKeypair);

    console.log("Initial Balances:");
    console.log(`Alice (${aliceKeypair.publicKey}):`);
    console.log(` Compressed: ${aliceBalances.compressed / LAMPORTS_PER_SOL} SOL`);
    console.log(` Uncompressed: ${aliceBalances.uncompressed / LAMPORTS_PER_SOL} SOL`);
    console.log(`Bob (${bobKeypair.publicKey}):`);
    console.log(` Compressed: ${bobBalances.compressed / LAMPORTS_PER_SOL} SOL`);
    console.log(` Uncompressed: ${bobBalances.uncompressed / LAMPORTS_PER_SOL} SOL`);
}


async function transferAsync(rpc: Rpc, from: Signer, to: PublicKey): Promise<void> {
    try {
        const transferSig = await transfer(rpc, from, TRANSFER_AMOUNT, from, to);
        console.log(`Transfer from ${from.publicKey.toBase58()} to ${to.toBase58()}: ${transferSig}`);
    } catch (error) {
        console.error(`Transfer failed: ${error}`);
    }
}

async function createAccountAsync(rpc: Rpc, payer: Signer): Promise<void> {
    try {
        const seed = new Uint8Array(randomBytes(32));
        const tx = await createAccount(
            rpc,
            payer,
            seed,
            LightSystemProgram.programId,
        );
        console.log(`Create account by ${payer.publicKey.toBase58()}: ${tx}`);
    } catch (error) {
        console.error(`Create account failed: ${error}`);
    }
}

async function compressIfNeeded(rpc: Rpc, keypair: Keypair): Promise<void> {
    const { compressed: compressedBalance, uncompressed: regularBalance } = await getBalances(rpc, keypair);
    if (compressedBalance <= LAMPORTS_PER_SOL * 0.1 && regularBalance  >= MIN_BALANCE_FOR_COMPRESS) {
        try {
            const compressSig = await compress(rpc, keypair, COMPRESS_AMOUNT, keypair.publicKey);
            console.log(`Compress tx sig for ${keypair.publicKey.toBase58()}:`, compressSig);
        } catch (error) {
            console.error(`Compression failed for ${keypair.publicKey.toBase58()}: ${error}`);
        }
    }
}

function devnetRpc(): Rpc {
    let validatorUrl = 'https://mainnet.helius-rpc.com/?api-key=27c42b89-12ac-41d0-8fa7-6341caa5737d';
    let photonUrl = 'https://mainnet.helius-rpc.com/?api-key=27c42b89-12ac-41d0-8fa7-6341caa5737d';
    let proverUrl = 'https://mainnet.helius-rpc.com/?api-key=27c42b89-12ac-41d0-8fa7-6341caa5737d';
    return createRpc(validatorUrl, photonUrl, proverUrl);
}

async function infiniteStressTest() {
    const rpc = devnetRpc();

    // Print initial balances
    await printInitialBalances(rpc);

    while (true) {
        await compressIfNeeded(rpc, aliceKeypair);
        await compressIfNeeded(rpc, bobKeypair);

        // Transfer from A to B
        await transferAsync(rpc, aliceKeypair, bobKeypair.publicKey);

        // Transfer from B to A
        await transferAsync(rpc, bobKeypair, aliceKeypair.publicKey);

        // Create account using A
        await createAccountAsync(rpc, aliceKeypair);

        // Create account using B
        await createAccountAsync(rpc, bobKeypair);

        // Optional: Add a small delay to prevent overwhelming the server
        await new Promise(resolve => setTimeout(resolve, 1000));
    }
}

infiniteStressTest().catch((error) => {
    console.error('An error occurred:', error);
});

