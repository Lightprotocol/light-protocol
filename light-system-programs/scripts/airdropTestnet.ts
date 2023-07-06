import { ADMIN_AUTH_KEYPAIR, airdropSol, confirmConfig, createTestAccounts, setUpMerkleTree, sleep } from '@lightprotocol/zk.js';
import * as anchor from "@coral-xyz/anchor";
import { Connection, PublicKey, sendAndConfirmTransaction, SystemProgram, Transaction } from '@solana/web3.js';

process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";
process.env.ANCHOR_PROVIDER_URL = "https://api.testnet.solana.com";

const recipient = "CLEuMG7pzJX9xAuKCFzBP154uiG1GaNo4Fq7x6KAcAfG";

async function main() {
    console.log("airdropping 100 testnet sol to ", recipient, " in 100 transaftions");
    
    // Replace this with your user's Solana wallet
    const deployerSolanaWallet = ADMIN_AUTH_KEYPAIR;
    const connection = new Connection(process.env.ANCHOR_PROVIDER_URL, confirmConfig);
    const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(deployerSolanaWallet), confirmConfig);
        for (var i = 0; i < 100; i++) {      
            const tmpSolanaWallet = anchor.web3.Keypair.generate();
            
            await airdropSol({provider, lamports: 1e9, recipientPublicKey: tmpSolanaWallet.publicKey})
            await sleep(1000);
            const balance = await provider.connection.getBalance(tmpSolanaWallet.publicKey);
            
            let tx = new Transaction().add(SystemProgram.transfer({fromPubkey: tmpSolanaWallet.publicKey, toPubkey: new PublicKey(recipient), lamports: balance-5000}))
            await sendAndConfirmTransaction(provider.connection, tx, [tmpSolanaWallet], confirmConfig);
            console.log(`${recipient} balance ${await provider.connection.getBalance(new PublicKey(recipient))}`);

            await sleep(10000);
    }
}

main()