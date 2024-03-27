import { Connection, PublicKey } from "@solana/web3.js";
import { getSolanaRpcUrl } from "../../src";
import { confirmTx, getTestRpc } from "@lightprotocol/stateless.js";

export async function requestAirdrop(address: PublicKey, amount = 3e9) {
  const rpc = await getTestRpc(getSolanaRpcUrl());
  const connection = new Connection(getSolanaRpcUrl(), "finalized");
  let sig = await connection.requestAirdrop(address, amount);
  await confirmTx(rpc, sig);
}
