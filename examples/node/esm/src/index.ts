import { airdropSol, compress, createRpc } from "@lightprotocol/stateless.js";
import { Keypair } from "@solana/web3.js";

const rpc = createRpc();
const keypair = new Keypair();

(async () => {
  await airdropSol({
    connection: rpc,
    lamports: 1e11,
    recipientPublicKey: keypair.publicKey,
  });
  const tx = await compress(rpc, keypair, 1e9, keypair.publicKey);
  console.log("compress tx", tx);
})();
