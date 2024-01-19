import {
  ConfirmOptions,
  Connection,
  PublicKey,
  RpcResponseAndContext,
  SignatureResult,
  TransactionInstruction,
  TransactionSignature,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

import {
  RpcError,
  RpcErrorCode,
  Provider,
  TOKEN_ACCOUNT_FEE,
  SendVersionedTransactionsResult,
  BN_0,
  RpcIndexedTransactionResponse,
  RpcIndexedTransaction,
  PrioritizationFee,
  SignaturesWithBlockhashInfo,
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

  /**
   * Convenience function for sending and confirming instructions via Light RPC node.
   * Routes instructions to Light RPC node and confirms the last transaction signature.
   */
  async sendAndConfirmSolanaInstructions(
    _ixs: TransactionInstruction[],
    _connection: Connection,
    _confirmOptions?: ConfirmOptions,
    _prioritizationFee?: PrioritizationFee,
    _provider?: Provider,
  ): Promise<TransactionSignature[]> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "sendAndConfirmSolanaInstructions",
      "Kept for compatibility with testRpc.",
    );
  }

  /**
   * Convenience function for sending instructions via Light RPC node.
   * Routes instructions to Light RPC node and returns tx metadata.
   */
  async sendSolanaInstructions(
    _ixs: TransactionInstruction[],
    _prioritizationFee?: bigint,
  ): Promise<SignaturesWithBlockhashInfo> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "sendSolanaInstructions",
      "Kept for compatibility with testRpc.",
    );
  }

  /** Not extended by TestRpc */
  getRpcFee(ataCreationFee?: boolean): BN {
    return ataCreationFee ? this.highRpcFee : this.rpcFee;
  }

  async getIndexedTransactions(
    /* We must keep the param for type equality with TestRpc */
    _connection: Connection,
  ): Promise<RpcIndexedTransaction[]> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "getIndexedTransactions",
      "Kept for compatibility with testRpc.",
    );
  }

  /** Not extended by TestRpc */
  static async initFromUrl(_url: string): Promise<Rpc> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "initFromUrl",
      "Kept for compatibility with testRpc.",
    );
  }

  async getEventById(
    _merkleTreePdaPublicKey: PublicKey,
    _id: string,
    _variableNameID: number,
  ): Promise<RpcIndexedTransactionResponse | undefined> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "getEventById",
      "Kept for compatibility with testRpc.",
    );
  }

  async getEventsByIdBatch(
    _merkleTreePdaPublicKey: PublicKey,
    _ids: string[],
    _variableNameID: number,
  ): Promise<RpcIndexedTransactionResponse[] | undefined> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "getEventsByIdBatch",
      "Kept for compatibility with testRpc.",
    );
  }

  async getMerkleProofByIndexBatch(
    _merkleTreePdaPublicKey: PublicKey,
    _indexes: number[],
  ): Promise<
    { merkleProofs: string[][]; root: string; index: number } | undefined
  > {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "getMerkleProofByIndexBatch",
      "Kept for compatibility with testRpc.",
    );
  }

  async getMerkleRoot(
    _merkleTreePdaPublicKey: PublicKey,
  ): Promise<{ root: string; index: number } | undefined> {
    throw new RpcError(
      RpcErrorCode.RPC_METHOD_NOT_IMPLEMENTED,
      "getMerkleRoot",
      "Kept for compatibility with testRpc.",
    );
  }
}
