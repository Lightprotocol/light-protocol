import {
  Connection,
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
  SendVersionedTransactionsResult,
  ParsedIndexedTransaction,
  BN_0,
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
  };
  relayerFee: BN;
  highRelayerFee: BN;
  indexedTransactions: ParsedIndexedTransaction[] = [];
  url: string;

  /**
   *
   * @param relayerPubkey Signs the transaction
   * @param relayerRecipientSol Recipient account for SOL fees
   * @param relayerFee Fee amount
   * @param highRelayerFee
   * @param url
   */
  constructor(
    relayerPubkey: PublicKey,
    relayerRecipientSol?: PublicKey,
    relayerFee: BN = BN_0,
    highRelayerFee: BN = TOKEN_ACCOUNT_FEE,
    url: string = "http://localhost:3332",
  ) {
    if (!relayerPubkey) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED,
        "constructor",
      );
    }
    if (relayerRecipientSol && relayerFee.eq(BN_0)) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "constructor",
        "If relayerRecipientSol is defined, relayerFee must be defined and non zero.",
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
        relayerRecipientSol,
      };
    } else {
      this.accounts = {
        relayerPubkey,
        relayerRecipientSol: relayerPubkey,
      };
    }
    this.highRelayerFee = highRelayerFee;
    this.relayerFee = relayerFee;
    this.url = url;
  }

  async sendTransactions(
    instructions: any[],
    _provider: Provider,
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
    /* We must keep the param for type equality with TestRelayer */
    _connection: Connection,
  ): Promise<ParsedIndexedTransaction[]> {
    try {
      const response = await axios.get(this.url + "/indexedTransactions");

      const indexedTransactions: ParsedIndexedTransaction[] =
        response.data.data.map((trx: IndexedTransaction) => {
          return {
            ...trx,
            signer: new PublicKey(trx.signer),
            to: new PublicKey(trx.to),
            from: new PublicKey(trx.from),
            toSpl: new PublicKey(trx.toSpl),
            fromSpl: new PublicKey(trx.fromSpl),
            verifier: new PublicKey(trx.verifier),
            relayerRecipientSol: new PublicKey(trx.relayerRecipientSol),
            firstLeafIndex: new BN(trx.firstLeafIndex, "hex"),
            publicAmountSol: new BN(trx.publicAmountSol, "hex"),
            publicAmountSpl: new BN(trx.publicAmountSpl, "hex"),
            changeSolAmount: new BN(trx.changeSolAmount, "hex"),
            relayerFee: new BN(trx.relayerFee, "hex"),
          };
        });

      return indexedTransactions;
    } catch (err) {
      console.log({ err });
      throw err;
    }
  }

  async syncRelayerInfo(): Promise<void> {
    const response = await axios.get(this.url + "/getRelayerInfo");
    const data = response.data.data;
    this.accounts.relayerPubkey = new PublicKey(data.relayerPubkey);
    this.accounts.relayerRecipientSol = new PublicKey(data.relayerRecipientSol);
    this.relayerFee = new BN(data.relayerFee);
    this.highRelayerFee = new BN(data.highRelayerFee);
  }

  static async initFromUrl(url: string): Promise<Relayer> {
    const response = await axios.get(url + "/getRelayerInfo");
    const data = response.data.data;
    return new Relayer(
      new PublicKey(data.relayerPubkey),
      new PublicKey(data.relayerRecipientSol),
      new BN(data.relayerFee),
      new BN(data.highRelayerFee),
      url,
    );
  }
}
