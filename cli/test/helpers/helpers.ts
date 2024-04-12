import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getPayer, getSolanaRpcUrl } from "../../src";
import { confirmTx, createRpc } from "@lightprotocol/stateless.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";

export async function requestAirdrop(address: PublicKey, amount = 3e9) {
  const rpc = createRpc(getSolanaRpcUrl());
  const connection = new Connection(getSolanaRpcUrl(), "finalized");
  let sig = await connection.requestAirdrop(address, amount);
  await confirmTx(rpc, sig);
}

export async function createTestMint(mintKeypair: Keypair) {
  const rpc = createRpc(getSolanaRpcUrl());

  const { mint, transactionSignature } = await createMint(
    rpc,
    await getPayer(),
    await getPayer(),
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
  const rpc = createRpc(getSolanaRpcUrl());

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
