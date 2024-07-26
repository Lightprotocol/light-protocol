import { PublicKey, Signer, Keypair } from "@solana/web3.js";
import {
  airdropSol,
  createRpc,
  compress,
  transfer,
  createAccount,
  LightSystemProgram,
  Rpc,
} from "@lightprotocol/stateless.js";
import nacl from "tweetnacl";
import * as dotenv from "dotenv";

dotenv.config();
const PHOTON_API_KEY = process.env.PHOTON_API_KEY;

const payerKeypair = [
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
const kp = (kp: number[]) => Keypair.fromSecretKey(Uint8Array.from(kp));

function getRpc(network: string): Rpc {
  switch (network) {
    case "localnet":
      return createRpc(
        "http://0.0.0.0:8899",
        "http://0.0.0.0:8784",
        "http://0.0.0.0:3001"
      );
    case "zktestnet":
      return createRpc(
        "https://zk-testnet.helius.dev:8899",
        "https://zk-testnet.helius.dev:8784"
      );
    case "devnet":
      const url = `https://devnet.helius-rpc.com?api-key=${PHOTON_API_KEY}`;
      return createRpc(url, url);
    default:
      throw new Error(`Unknown network: ${network}`);
  }
}

const LAMPORTS = 1e9;
const COMPRESS_AMOUNT = 1e9 - 50000;
const TOTAL_NUMBER_OF_TRANSFERS = 10;
const NUMBER_OF_CONCURRENT_TRANSFERS = 10;
const TRANSFER_AMOUNT = 10;

async function transferAsync(
  i: number,
  rpc: Rpc,
  payer: Signer,
  bobPublicKey: PublicKey
): Promise<void> {
  const transferSig = await transfer(
    rpc,
    payer,
    TRANSFER_AMOUNT,
    payer,
    bobPublicKey
  );
  console.log(`transfer ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${transferSig}`);
}

async function createAccountAsync(i: number, rpc: Rpc, payer: Signer) {
  const sig = await createAccount(
    rpc,
    payer,
    nacl.randomBytes(32),
    LightSystemProgram.programId
  );
  console.log(`create account ${i} of ${TOTAL_NUMBER_OF_TRANSFERS}: ${sig}`);
}

async function airdropToAccount(rpc: Rpc, account: Keypair, lamports: number) {
  console.log(
    `Airdropping ${lamports} lamports to ${account.publicKey.toBase58()}...`
  );
  const tx = await airdropSol({
    connection: rpc,
    lamports,
    recipientPublicKey: account.publicKey,
  });
  console.log("Airdrop tx:", tx);
}

async function logBalances(rpc: Rpc, payer: Keypair, bob: Keypair) {
  const payerBalance = await rpc.getBalance(payer.publicKey);
  console.log("payer balance", payerBalance);

  const bobBalance = await rpc.getBalance(bob.publicKey);
  console.log("bob balance", bobBalance);
}

async function performTransfers(
  rpc: Rpc,
  payer: Signer,
  bobPublicKey: PublicKey
) {
  for (
    let i = 0;
    i < TOTAL_NUMBER_OF_TRANSFERS;
    i += NUMBER_OF_CONCURRENT_TRANSFERS
  ) {
    const transferPromises = Array.from(
      { length: NUMBER_OF_CONCURRENT_TRANSFERS },
      (_, j) => transferAsync(i + j, rpc, payer, bobPublicKey)
    );
    await Promise.all(transferPromises);
  }
}

async function performCreateAddresses(rpc: Rpc, payer: Signer) {
  for (
    let i = 0;
    i < TOTAL_NUMBER_OF_TRANSFERS;
    i += NUMBER_OF_CONCURRENT_TRANSFERS
  ) {
    const createAccountPromises = Array.from(
      { length: NUMBER_OF_CONCURRENT_TRANSFERS },
      (_, j) => createAccountAsync(i + j, rpc, payer)
    );
    await Promise.all(createAccountPromises);
  }
}

async function compressSol(rpc: Rpc, payer: Signer, amount: number) {
  const compressSig = await compress(rpc, payer, amount, payer.publicKey);
  console.log("compress tx sig", compressSig);
}

async function executeOperations(operations: string[]) {
  const rpc = getRpc("devnet");
  const payer = kp(payerKeypair);
  const bob = kp(bobKeypair);

  console.log("payer:", payer.publicKey.toBase58());
  console.log("bob:", bob.publicKey.toBase58());

  for (const operation of operations) {
    switch (operation) {
      case "airdrop":
        await airdropToAccount(rpc, payer, LAMPORTS);
        await airdropToAccount(rpc, bob, LAMPORTS);
        await logBalances(rpc, payer, bob);
        break;
      case "compress":
        await compressSol(rpc, payer, COMPRESS_AMOUNT);
        break;
      case "transfer":
        await performTransfers(rpc, payer, bob.publicKey);
        break;
      case "createAddresses":
        await performCreateAddresses(rpc, payer);
        break;
      default:
        console.log(`Unknown operation: ${operation}`);
    }
  }
}

async function executeInfiniteLoop(rpc: Rpc, payer: Signer, bob: PublicKey) {
  while (true) {
    try {
      await Promise.all([
        performTransfers(rpc, payer, bob),
        performCreateAddresses(rpc, payer),
      ]);

      await new Promise((resolve) => setTimeout(resolve, 100));
    } catch (error) {
      console.error("Error in infinite loop:", error);
      break;
    }
  }
}

async function main() {
  const rpc = getRpc("devnet");
  const payer = kp(payerKeypair);
  const bob = kp(bobKeypair);
  // Example usage:
  // await executeOperations(["airdrop", "compress"]);

  await executeInfiniteLoop(rpc, payer, bob.publicKey);
}

main().catch(console.error);
