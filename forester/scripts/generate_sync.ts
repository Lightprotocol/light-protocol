import { PublicKey, Signer, Keypair, LAMPORTS_PER_SOL } from '@solana/web3.js';
import {
  airdropSol,
  createRpc,
  compress,
  transfer,
  Rpc,
  LightSystemProgram,
  createAccount
} from '@lightprotocol/stateless.js';
import { randomBytes } from 'tweetnacl';

// Constants
const LAMPORTS = 1e11;
const COMPRESS_AMOUNT = 1e8;
const TOTAL_NUMBER_OF_TRANSFERS = 10;
const NUMBER_OF_CONCURRENT_TRANSFERS = 1;
const TRANSFER_AMOUNT = 10;

// Keypairs (consider moving these to a separate config file)
const payerKeypair = new Uint8Array([
  46, 239, 29, 58, 196, 181, 39, 77, 196, 54, 249, 108, 80, 144, 32, 168, 245, 161, 146, 92, 180, 79,
  231, 37, 50, 88, 220, 48, 9, 146, 249, 82, 130, 60, 106, 251, 24, 224, 192, 108, 70, 59, 111, 251,
  186, 50, 23, 103, 106, 233, 113, 148, 57, 190, 158, 111, 163, 28, 157, 47, 201, 41, 249, 59
]);

const bobKeypair = new Uint8Array([
  125, 14, 244, 185, 193, 42, 156, 191, 212, 42, 239, 56, 169, 240, 239, 52, 95, 215, 240, 86, 151, 212,
  245, 230, 198, 148, 12, 230, 83, 57, 56, 244, 191, 129, 151, 233, 233, 129, 21, 255, 101, 163, 48, 212,
  218, 82, 134, 36, 29, 185, 30, 215, 183, 242, 244, 222, 8, 10, 158, 214, 99, 237, 126, 9
]);

// RPC configuration functions
function localRpc(): Rpc {
  return createRpc('http://0.0.0.0:8899', 'http://0.0.0.0:8784', 'http://0.0.0.0:3001');
}

function zkTestnetRpc(): Rpc {
  return createRpc(
    'https://zk-testnet.helius.dev:8899',
    'https://zk-testnet.helius.dev:8784',
    'https://zk-testnet.helius.dev:3001'
  );
}

function devnetRpc(): Rpc {
  const apiKey = '<HELIUS_API_KEY>';
  return createRpc(
    `https://devnet.helius-rpc.com/?api-key=${apiKey}`,
    `https://devnet.helius-rpc.com/?api-key=${apiKey}`,
    `https://devnet.helius-rpc.com:3001/?api-key=${apiKey}`
  );
}

// Helper functions
async function transferAsync(i: number, rpc: Rpc, payer: Signer, recipient: PublicKey): Promise<void> {
  const transferSig = await transfer(rpc, payer, TRANSFER_AMOUNT, payer, recipient);
  console.log(`Transfer ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${transferSig}`);
}

async function createAccountAsync(i: number, rpc: Rpc, payer: Signer): Promise<void> {
  const seed = randomBytes(32);
  const accountSig = await createAccount(rpc, payer, seed, LightSystemProgram.programId);
  console.log(`Account ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${accountSig}`);
}

async function airdropIfNeeded(payer: Keypair, bob: Keypair, rpc: Rpc): Promise<void> {
  console.log("Payer:", payer.publicKey.toBase58());
  console.log("Bob:", bob.publicKey.toBase58());

  const [bobBalance, payerBalance] = await Promise.all([
    rpc.getBalance(bob.publicKey),
    rpc.getBalance(payer.publicKey)
  ]);

  console.log('Bob balance:', bobBalance);
  console.log('Payer balance:', payerBalance);

  if (payerBalance < LAMPORTS_PER_SOL) {
    console.log('Airdropping SOL to payer');
    await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: payer.publicKey });
    console.log('New payer balance:', await rpc.getBalance(payer.publicKey));
  }

  if (bobBalance < LAMPORTS_PER_SOL) {
    console.log('Airdropping SOL to Bob');
    await airdropSol({ connection: rpc, lamports: LAMPORTS, recipientPublicKey: bob.publicKey });
    console.log('New Bob balance:', await rpc.getBalance(bob.publicKey));
  }
}

// Main function
async function prefillNullifierQueue(): Promise<void> {
  const rpc = devnetRpc();
  const payer = Keypair.fromSecretKey(payerKeypair);
  const bob = Keypair.fromSecretKey(bobKeypair);

  await airdropIfNeeded(payer, bob, rpc);
  const compressSig = await compress(rpc, payer, COMPRESS_AMOUNT, payer.publicKey);
  console.log('Compress transaction signature:', compressSig);

  for (let i = 0; i < TOTAL_NUMBER_OF_TRANSFERS; i += NUMBER_OF_CONCURRENT_TRANSFERS) {
    await Promise.all(
      Array.from({ length: NUMBER_OF_CONCURRENT_TRANSFERS }, (_, j) =>
        transferAsync(i + j, rpc, payer, bob.publicKey)
      )
    );
  }

  for (let i = 0; i < TOTAL_NUMBER_OF_TRANSFERS; i += NUMBER_OF_CONCURRENT_TRANSFERS) {
    await Promise.all(
      Array.from({ length: NUMBER_OF_CONCURRENT_TRANSFERS }, (_, j) =>
        createAccountAsync(i + j, rpc, payer)
      )
    );
  }
}

// Run the script
prefillNullifierQueue()
  .then(() => console.log('Transfer completed successfully.'))
  .catch((error) => console.error('An error occurred:', error));