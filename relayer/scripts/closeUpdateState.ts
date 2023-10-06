import { AnchorProvider, Program, Wallet, utils } from "@coral-xyz/anchor";
import {
  confirmConfig,
  IDL_MERKLE_TREE_PROGRAM,
  merkleTreeProgramId,
  closeMerkleTreeUpdateState,
} from "@lightprotocol/zk.js";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { getKeyPairFromEnv } from "../src/utils/provider";
import { RPC_URL } from "../src/config";

(async () => {
  let wallet = getKeyPairFromEnv("KEY_PAIR");
  let url = RPC_URL;
  const connection = new Connection(url, confirmConfig);

  const anchorProvider = new AnchorProvider(
    connection,
    new Wallet(Keypair.generate()),
    confirmConfig,
  );

  const merkleTreeProgram = new Program(
    IDL_MERKLE_TREE_PROGRAM,
    merkleTreeProgramId,
    anchorProvider,
  );

  let pda = PublicKey.findProgramAddressSync(
    [
      Buffer.from(new Uint8Array(wallet.publicKey.toBytes())),
      utils.bytes.utf8.encode("storage"),
    ],
    merkleTreeProgram.programId,
  )[0];
  console.log("closing merkletreeupdatestate:", pda.toBase58());
  try {
    await closeMerkleTreeUpdateState(merkleTreeProgram, wallet, connection);
  } catch (e) {
    throw e;
  }
})();
