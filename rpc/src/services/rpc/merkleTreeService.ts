import {
  IDL_LIGHT_MERKLE_TREE_PROGRAM,
  Provider,
  SolMerkleTree,
  getRootIndex,
  merkleTreeProgramId,
} from "@lightprotocol/zk.js";
import { getLightProvider } from "../../utils/provider";
import { getIndexedTransactionsJob } from "./eventService";
import { Program } from "@coral-xyz/anchor";
import { PublicKey } from "@solana/web3.js";

export const getMerkleRoot = async (req: any, res: any) => {
  try {
    console.log("@getMerkleRoot! ", req.body);
    const { merkleTreePdaPublicKey: merkleTreePdaPublicKeyString } = req.body;

    const merkleTreePdaPublicKey = new PublicKey(merkleTreePdaPublicKeyString);
    const indexedTransactions = await getIndexedTransactionsJob(
      merkleTreePdaPublicKey,
    );
    if (!indexedTransactions) {
      return res
        .status(404)
        .json({ status: "error", message: "No indexed transactions found" });
    }

    const provider: Provider = await getLightProvider();
    const merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider.provider,
    );
    const merkleTree = await SolMerkleTree.build({
      pubkey: merkleTreePdaPublicKey,
      lightWasm: provider.lightWasm,
      indexedTransactions: indexedTransactions.map((trx) => trx.transaction),
      provider: provider.provider,
    });
    // TODO: avoid rpc calls to accounts, stream the data of accounts and cache it instead
    const index = await getRootIndex(
      merkleTreeProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
    );

    return res.status(200).json({
      data: { root: merkleTree.merkleTree.root(), index: index.toNumber() },
    });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
};

export const getMerkleProofByIndexBatch = async (req: any, res: any) => {
  try {
    console.log("@getMerkleProofByIndexBatch! ", req.body);

    const { merkleTreePdaPublicKey: merkleTreePdaPublicKeyString, indexes } =
      req.body;
    const merkleTreePdaPublicKey = new PublicKey(merkleTreePdaPublicKeyString);
    let indexedTransactions = await getIndexedTransactionsJob(
      merkleTreePdaPublicKey,
    );
    if (!indexedTransactions) {
      // TODO: return error but right it would break the client implementation
      indexedTransactions = [];
    }

    const provider: Provider = await getLightProvider();
    const merkleTreeProgram = new Program(
      IDL_LIGHT_MERKLE_TREE_PROGRAM,
      merkleTreeProgramId,
      provider.provider,
    );
    const merkleTree = await SolMerkleTree.build({
      pubkey: merkleTreePdaPublicKey,
      lightWasm: provider.lightWasm,
      indexedTransactions: indexedTransactions.map((trx) => trx.transaction),
      provider: provider.provider,
    });
    // TODO: avoid rpc calls to accounts, stream the data of accounts and cache it instead
    const index = await getRootIndex(
      merkleTreeProgram,
      merkleTree.pubkey,
      merkleTree.merkleTree.root(),
    );
    // issue is that zk js expects proofs to returned but rn an error and no proofs are returned
    const merkleProofs = indexes.map(
      (index: string) => merkleTree.merkleTree.path(Number(index)).pathElements,
    );
    return res.status(200).json({
      data: {
        root: merkleTree.merkleTree.root(),
        index: index.toNumber(),
        merkleProofs,
      },
    });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
};
