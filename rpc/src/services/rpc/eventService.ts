import {
  RpcIndexedTransaction,
  SolMerkleTree,
  createRpcIndexedTransaction,
  Provider,
} from "@lightprotocol/zk.js";
import { DB_VERSION } from "../../config";
import { indexQueue } from "../../db/redis";
import { getLightProvider } from "../../utils/provider";
import { PublicKey } from "@solana/web3.js";

export async function getEventById(
  req: any,
  res: any,
): Promise<RpcIndexedTransaction | undefined> {
  try {
    console.log("@getEventById! ", req.body);
    const { id, merkleTreePdaPublicKey: merkleTreePdaPublicKeyString } =
      req.body;
    const merkleTreePdaPublicKey = new PublicKey(merkleTreePdaPublicKeyString);

    let indexedTransactions = await getIndexedTransactionsJob(
      merkleTreePdaPublicKey,
    );
    if (!indexedTransactions) {
      // TODO: return different status code currently the client expects 200
      indexedTransactions = [];
    }

    const indexedTransaction = indexedTransactions.find((trx) =>
      trx.IDs.includes(id),
    )?.transaction;
    if (!indexedTransaction) return undefined;
    const provider: Provider = await getLightProvider();
    const merkleTree = await SolMerkleTree.build({
      pubkey: merkleTreePdaPublicKey,
      lightWasm: provider.lightWasm,
      indexedTransactions: indexedTransactions.map((trx) => trx.transaction),
      provider: provider.provider,
    });

    return res.status(200).json({
      data: createRpcIndexedTransaction(indexedTransaction, merkleTree),
    });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}

// TODO: make ids an array which can be indexed in with variableNameID
export async function getEventsByIdBatch(
  req: any,
  res: any,
): Promise<RpcIndexedTransaction[] | undefined> {
  try {
    console.log("@getEventsByIdBatch! ", req.body);

    const { ids, merkleTreePdaPublicKey: merkleTreePdaPublicKeyString } =
      req.body;
    const merkleTreePdaPublicKey = new PublicKey(merkleTreePdaPublicKeyString);

    if (!ids || ids.length === 0)
      return res.status(200).json({ status: "No ids provided", data: [] });
    const indexedTransactions = await getIndexedTransactionsJob(
      merkleTreePdaPublicKey,
    );
    if (!indexedTransactions) {
      // TODO: don't return empty data array but it would break the current client implementation
      return res
        .status(200)
        .json({ status: "No indexed transactions found", data: [] });
    }

    const provider: Provider = await getLightProvider();
    const merkleTree = await SolMerkleTree.build({
      pubkey: merkleTreePdaPublicKey,
      lightWasm: provider.lightWasm,
      indexedTransactions: indexedTransactions.map((trx) => trx.transaction),
      provider: provider.provider,
    });
    const indexedTransactionsById = indexedTransactions.filter((trx) =>
      trx.IDs.some((id: string) => ids.includes(id)),
    );
    const rpcIndexedTransactions = await indexedTransactionsById.map((trx) =>
      createRpcIndexedTransaction(trx.transaction, merkleTree),
    );
    return res.status(200).json({ data: rpcIndexedTransactions });
  } catch (e) {
    console.log(e);
    return res.status(500).json({ status: "error", message: e.message });
  }
}

// TODO: implement support for multiple Merkle trees
// TODO: use merkle tree publickey to get indexed transactions of the respective merkle tree once we have multiple merkle trees
export async function getIndexedTransactionsJob(
  _merkleTreePdaPublicKey: PublicKey,
): Promise<RpcIndexedTransaction[] | undefined> {
  console.log("@getIndexedTransactions!");
  const version = DB_VERSION;
  const job = (await indexQueue.getWaiting())[version];
  if (!job) {
    console.log("No indexed transctions found");
    return undefined;
  }
  return job.data.transactions;
}
