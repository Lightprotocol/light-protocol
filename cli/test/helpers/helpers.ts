import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getSolanaRpcUrl } from "../../src";
import { confirmTx, getTestRpc } from "@lightprotocol/stateless.js";
import { createMint, mintTo } from "@lightprotocol/compressed-token";

export async function requestAirdrop(address: PublicKey, amount = 3e9) {
  const rpc = await getTestRpc(getSolanaRpcUrl());
  const connection = new Connection(getSolanaRpcUrl(), "finalized");
  let sig = await connection.requestAirdrop(address, amount);
  await confirmTx(rpc, sig);
}

export async function createTestMint(payer: Keypair) {
  const rpc = await getTestRpc(getSolanaRpcUrl());
  const { mint } = await createMint(rpc, payer, payer, 9, undefined, {
    commitment: "finalized",
  });
  return mint;
}

export async function testMintTo(
  payer: Keypair,
  mintAddress: PublicKey,
  mintDestination: PublicKey,
  mintAuthority: Keypair,
  mintAmount: number,
) {
  const rpc = await getTestRpc(getSolanaRpcUrl());
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
