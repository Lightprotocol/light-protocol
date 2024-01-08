import {
  Connection,
  PublicKey,
  RpcResponseAndContext,
  SignatureResult,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import axios from "axios";
import {
  RpcError,
  RpcErrorCode,
  Provider,
  IndexedTransaction,
  TOKEN_ACCOUNT_FEE,
  SendVersionedTransactionsResult,
  BN_0,
  RpcIndexedTransactionResponse,
  RpcIndexedTransaction,
} from "./index";

export type RpcSendTransactionsResponse = SendVersionedTransactionsResult & {
  transactionStatus: string;
  rpcResponse?: RpcResponseAndContext<SignatureResult>;
};

export class Rpc {
  accounts: {
    rpcPubkey: PublicKey; // signs the transaction
    rpcRecipientSol: PublicKey; // receives the fees
  };
  rpcFee: BN;
  highRpcFee: BN;
  url: string;

  /**
   *
   * @param rpcPubkey Signs the transaction
   * @param rpcRecipientSol Recipient account for SOL fees
   * @param rpcFee Fee amount
   * @param highRpcFee
   * @param url
   */
  constructor(
    rpcPubkey: PublicKey,
    rpcRecipientSol?: PublicKey,
    rpcFee: BN = BN_0,
    highRpcFee: BN = TOKEN_ACCOUNT_FEE,
    url: string = "http://localhost:3332",
  ) {
    if (!rpcPubkey) {
      throw new RpcError(RpcErrorCode.RPC_PUBKEY_UNDEFINED, "constructor");
    }
    if (rpcRecipientSol && rpcFee.eq(BN_0)) {
      throw new RpcError(
        RpcErrorCode.RPC_FEE_UNDEFINED,
        "constructor",
        "If rpcRecipientSol is defined, rpcFee must be defined and non zero.",
      );
    }
    if (rpcFee.toString() !== "0" && !rpcRecipientSol) {
      throw new RpcError(RpcErrorCode.RPC_RECIPIENT_UNDEFINED, "constructor");
    }
    if (rpcRecipientSol) {
      this.accounts = {
        rpcPubkey,
        rpcRecipientSol,
      };
    } else {
      this.accounts = {
        rpcPubkey,
        rpcRecipientSol: rpcPubkey,
      };
    }
    this.highRpcFee = highRpcFee;
    this.rpcFee = rpcFee;
    this.url = url;
  }

  async sendTransactions(
    instructions: any[],
    _provider: Provider,
  ): Promise<RpcSendTransactionsResponse> {
    try {
      const response = await axios.post(this.url + "/relayTransaction", {
        instructions,
      });
      return response.data.data;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }

  getRpcFee(ataCreationFee?: boolean): BN {
    return ataCreationFee ? this.highRpcFee : this.rpcFee;
  }

  async getIndexedTransactions(
    /* We must keep the param for type equality with TestRpc */
    _connection: Connection,
  ): Promise<RpcIndexedTransaction[]> {
    try {
      const response = await axios.get(this.url + "/indexedTransactions");

      const indexedTransactions: RpcIndexedTransaction[] =
        response.data.data.map((trx: IndexedTransaction) => {
          return {
            ...trx,
            signer: new PublicKey(trx.signer),
            to: new PublicKey(trx.to),
            from: new PublicKey(trx.from),
            toSpl: new PublicKey(trx.toSpl),
            fromSpl: new PublicKey(trx.fromSpl),
            verifier: new PublicKey(trx.verifier),
            rpcRecipientSol: new PublicKey(trx.rpcRecipientSol),
            firstLeafIndex: new BN(trx.firstLeafIndex, "hex"),
            publicAmountSol: new BN(trx.publicAmountSol, "hex"),
            publicAmountSpl: new BN(trx.publicAmountSpl, "hex"),
            changeSolAmount: new BN(trx.changeSolAmount, "hex"),
            rpcFee: new BN(trx.rpcFee, "hex"),
          };
        });

      return indexedTransactions;
    } catch (err) {
      console.log({ err });
      throw err;
    }
  }

  async syncRpcInfo(): Promise<void> {
    const response = await axios.get(this.url + "/getRpcInfo");
    const data = response.data.data;
    this.accounts.rpcPubkey = new PublicKey(data.rpcPubkey);
    this.accounts.rpcRecipientSol = new PublicKey(data.rpcRecipientSol);
    this.rpcFee = new BN(data.rpcFee);
    this.highRpcFee = new BN(data.highRpcFee);
  }

  static async initFromUrl(url: string): Promise<Rpc> {
    const response = await axios.get(url + "/getRpcInfo");
    const data = response.data.data;
    return new Rpc(
      new PublicKey(data.rpcPubkey),
      new PublicKey(data.rpcRecipientSol),
      new BN(data.rpcFee),
      new BN(data.highRpcFee),
      url,
    );
  }

  async getEventById(
    merkleTreePdaPublicKey: PublicKey,
    id: string,
    variableNameID: number,
  ): Promise<RpcIndexedTransactionResponse | undefined> {
    try {
      const response = await axios.post(this.url + "/getEventById", {
        id,
        variableNameID,
        merkleTreePdaPublicKey: merkleTreePdaPublicKey.toBase58(),
      });
      return response.data.data;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }

  async getEventsByIdBatch(
    merkleTreePdaPublicKey: PublicKey,
    ids: string[],
    variableNameID: number,
  ): Promise<RpcIndexedTransactionResponse[] | undefined> {
    if (ids.length === 0) return [];
    try {
      const response = await axios.post(this.url + "/getEventsByIdBatch", {
        ids,
        variableNameID,
        merkleTreePdaPublicKey: merkleTreePdaPublicKey.toBase58(),
      });
      return response.data.data;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }

  async getMerkleProofByIndexBatch(
    merkleTreePdaPublicKey: PublicKey,
    indexes: number[],
  ): Promise<
    { merkleProofs: string[][]; root: string; index: number } | undefined
  > {
    try {
      const response = await axios.post(
        this.url + "/getMerkleProofByIndexBatch",
        { indexes, merkleTreePdaPublicKey: merkleTreePdaPublicKey.toBase58() },
      );
      return response.data.data;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }

  async getMerkleRoot(
    merkleTreePdaPublicKey: PublicKey,
  ): Promise<{ root: string; index: number } | undefined> {
    try {
      const response = await axios.post(this.url + "/getMerkleRoot", {
        merkleTreePdaPublicKey: merkleTreePdaPublicKey.toBase58(),
      });
      return response.data.data;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }
}
