import {
  Connection,
  Keypair,
  PublicKey,
  Signer,
  SystemProgram,
} from "@solana/web3.js";
import { getPayer, getSolanaRpcUrl } from "../../src";
import {
  Rpc,
  buildAndSignTx,
  confirmTx,
  dedupeSigner,
  getTestRpc,
  sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
} from "@solana/spl-token";
import { WasmFactory } from "@lightprotocol/hasher.rs";

export async function requestAirdrop(address: PublicKey, amount = 3e9) {
  const lightWasm = await WasmFactory.getInstance();
  const rpc = await getTestRpc(lightWasm);
  const connection = new Connection(getSolanaRpcUrl(), "finalized");
  let sig = await connection.requestAirdrop(address, amount);
  await confirmTx(rpc, sig);
}

export async function createTestMint(mintKeypair: Keypair) {
  const lightWasm = await WasmFactory.getInstance();
  const rpc = await getTestRpc(lightWasm);

  const { mint, transactionSignature } = await createMint(
    rpc,
    await getPayer(),
    (await getPayer()).publicKey,
    9,
    mintKeypair,
  );
  await confirmTx(rpc, transactionSignature);
  return mint;
}

export async function testMintTo(
  payer: Keypair,
  mintAddress: PublicKey,
  mintDestination: PublicKey,
  mintAuthority: Keypair,
  mintAmount: number,
) {
  const lightWasm = await WasmFactory.getInstance();
  const rpc = await getTestRpc(lightWasm);

  const txId = await mintTo(
    rpc,
    payer,
    mintAddress,
    mintDestination,
    mintAuthority,
    mintAmount,
  );
  return txId;
}

export const TEST_TOKEN_DECIMALS = 2;

export async function createTestSplMint(
  rpc: Rpc,
  payer: Signer,
  mintKeypair: Signer,
  mintAuthority: Keypair,
) {
  const rentExemptBalance =
    await rpc.getMinimumBalanceForRentExemption(MINT_SIZE);

  const createMintAccountInstruction = SystemProgram.createAccount({
    fromPubkey: payer.publicKey,
    lamports: rentExemptBalance,
    newAccountPubkey: mintKeypair.publicKey,
    programId: TOKEN_PROGRAM_ID,
    space: MINT_SIZE,
  });
  const initializeMintInstruction = createInitializeMint2Instruction(
    mintKeypair.publicKey,
    TEST_TOKEN_DECIMALS,
    mintAuthority.publicKey,
    null,
    TOKEN_PROGRAM_ID,
  );
  const { blockhash } = await rpc.getLatestBlockhash();

  const tx = buildAndSignTx(
    [createMintAccountInstruction, initializeMintInstruction],
    payer,
    blockhash,
    dedupeSigner(payer, [mintKeypair]),
  );
  await sendAndConfirmTx(rpc, tx);
}
