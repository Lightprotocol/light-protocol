import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { RelayerError, RelayerErrorCode } from "./errors";

export class Relayer {
  accounts: {
    relayerPubkey: PublicKey; // signs the transaction
    relayerRecipient: PublicKey; // receives the fees
    lookUpTable: PublicKey;
  };
  relayerFee: BN;

  /**
   *
   * @param relayerPubkey Signs the transaction
   * @param lookUpTable  The relayer's lookuptable - uniformly used currently
   * @param relayerRecipient Recipient account for SOL fees
   * @param relayerFee Fee amount
   */
  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient?: PublicKey,
    relayerFee: BN = new BN(0),
  ) {
    if (!relayerPubkey) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_PUBKEY_UNDEFINED,
        "constructor",
      );
    }
    if (!lookUpTable) {
      throw new RelayerError(
        RelayerErrorCode.LOOK_UP_TABLE_UNDEFINED,
        "constructor",
      );
    }
    if (relayerRecipient && relayerFee.toString() === "0") {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_FEE_UNDEFINED,
        "constructor",
      );
    }
    if (relayerFee.toString() !== "0" && !relayerRecipient) {
      throw new RelayerError(
        RelayerErrorCode.RELAYER_RECIPIENT_UNDEFINED,
        "constructor",
      );
    }
    if (relayerRecipient) {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipient,
      };
    } else {
      this.accounts = {
        relayerPubkey,
        lookUpTable,
        relayerRecipient: relayerPubkey,
      };
    }
    this.relayerFee = relayerFee;
  }
}
