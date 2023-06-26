import {
  Connection,
  Keypair,
  PublicKey,
  RpcResponseAndContext,
  SignatureResult,
} from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import axios from "axios";
import {
  RelayerError,
  RelayerErrorCode,
  Provider,
  IndexedTransaction,
  TOKEN_ACCOUNT_FEE,
  LOOK_UP_TABLE,
  ConfirmOptions,
  SendVersionedTransactionsResult,
} from "./index";

export type RelayerSendTransactionsResponse =
  SendVersionedTransactionsResult & {
    transactionStatus: string;
    rpcResponse?: RpcResponseAndContext<SignatureResult>;
  };

export class Relayer {
  accounts: {
    relayerPubkey: PublicKey; // signs the transaction
    relayerRecipientSol: PublicKey; // receives the fees
    lookUpTable: PublicKey;
  };
  relayerFee: BN;
  highRelayerFee: BN;
  indexedTransactions: IndexedTransaction[] = [];
  url: string;

  /**
   *
   * @param relayerPubkey Signs the transaction
   * @param lookUpTable  The relayer's lookuptable - uniformly used currently
   * @param relayerRecipientSol Recipient account for SOL fees
   * @param relayerFee Fee amount
   */
  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipientSol?: PublicKey,
    relayerFee: BN = new BN(0),
    highRelayerFee: BN = new BN(TOKEN_ACCOUNT_FEE),
    url: string = "http://localhost:3331",
  ) {
    if (!relayerPubkey) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED,
        "constructor",
      );
    }
    // if (!lookUpTable) {
    //   throw new RelayerError(
    //     RelayerErrorCode.LOOK_UP_TABLE_UNDEFINED,
    //     "constructor",
    //   );
    // }
    if (relayerRecipientSol && relayerFee.toString() === "0") {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "constructor",
      );
    }
    if (relayerFee.toString() !== "0" && !relayerRecipientSol) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED,
        "constructor",
      );
    }
    if (relayerRecipientSol) {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipientSol,
      };
    } else {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipientSol: relayerPubkey,
      };
    }
    this.highRelayerFee = highRelayerFee;
    this.relayerFee = relayerFee;
    this.url = url;
  }

  async updateMerkleTree(provider: Provider) {
    try {
      const response = await axios.post(this.url + "/updatemerkletree");
      return response;
    } catch (err) {
      console.error({ err });
      throw err;
    }
  }

  async sendTransactions(
    instructions: any[],
    provider: Provider,
  ): Promise<RelayerSendTransactionsResponse> {
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

  getRelayerFee(ataCreationFee?: boolean): BN {
    return ataCreationFee ? this.highRelayerFee : this.relayerFee;
  }

  async getIndexedTransactions(
    connection: Connection,
  ): Promise<IndexedTransaction[]> {
    try {
      const response = await axios.get(this.url + "/indexedTransactions");

      const indexedTransactions: IndexedTransaction[] = response.data.data.map(
        (trx: IndexedTransaction) => {
          return {
            ...trx,
            to: new PublicKey(trx.to),
            from: new PublicKey(trx.from),
            firstLeafIndex: new BN(trx.firstLeafIndex),
          };
        },
      );

      return indexedTransactions.sort(
        (a, b) => a.firstLeafIndex.toNumber() - b.firstLeafIndex.toNumber(),
      );
    } catch (err) {
      console.log({ err });
      throw err;
    }
  }
}
