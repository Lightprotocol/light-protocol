import {
  Connection,
  Keypair,
  PublicKey,
  Signer,
  SystemProgram,
} from "@solana/web3.js";
import { getPayer, getSolanaRpcUrl, getIndexerUrl, getProverUrl } from "../../src/utils/utils";
import {
  Rpc,
  buildAndSignTx,
  confirmTx,
  createRpc,
  dedupeSigner,
  sendAndConfirmTx,
} from "@lightprotocol/stateless.js";
import {
  createMintInterface,
  createSplInterface,
  mintToInterface,
  createAtaInterfaceIdempotent,
  getAssociatedTokenAddressInterface,
} from "@lightprotocol/compressed-token";
import {
  MINT_SIZE,
  TOKEN_PROGRAM_ID,
  createInitializeMint2Instruction,
  mintTo as splMintTo,
  createAssociatedTokenAccountIdempotent,
} from "@solana/spl-token";

export async function requestAirdrop(address: PublicKey, amount = 3e9) {
  const rpc = createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());
  const connection = new Connection(getSolanaRpcUrl(), "finalized");
  let sig = await connection.requestAirdrop(address, amount);
  await confirmTx(rpc, sig);
}

export async function createTestMint(mintKeypair: Keypair) {
  const rpc = createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());

  const payer = await getPayer();
  const { mint, transactionSignature } = await createMintInterface(
    rpc,
    payer,
    payer,
    null,
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
  const rpc = createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());

  await createAtaInterfaceIdempotent(rpc, payer, mintAddress, mintDestination);
  const destination = getAssociatedTokenAddressInterface(mintAddress, mintDestination);
  const txId = await mintToInterface(
    rpc,
    payer,
    mintAddress,
    destination,
    mintAuthority,
    mintAmount,
  );
  return txId;
}

/**
 * Create a standard SPL mint, register it with the Light Token program,
 * and mint SPL tokens to the destination's ATA. Used by wrap/unwrap tests.
 */
export async function createTestSplMintWithPool(
  mintKeypair: Keypair,
  mintAuthority: Keypair,
  mintAmount: number,
  mintDestination: PublicKey,
) {
  const rpc = createRpc(getSolanaRpcUrl(), getIndexerUrl(), getProverUrl());
  const payer = await getPayer();

  await createTestSplMint(rpc, payer, mintKeypair, mintAuthority);
  await createSplInterface(rpc, payer, mintKeypair.publicKey);

  const ata = await createAssociatedTokenAccountIdempotent(
    rpc,
    payer,
    mintKeypair.publicKey,
    mintDestination,
  );
  await splMintTo(
    rpc,
    payer,
    mintKeypair.publicKey,
    ata,
    mintAuthority,
    mintAmount,
  );
  return mintKeypair.publicKey;
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
