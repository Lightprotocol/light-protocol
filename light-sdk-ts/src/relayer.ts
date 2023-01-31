import { Keypair, PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";

export class Relayer {
  accounts: {
    relayerPubkey: PublicKey; // signs the transaction
    relayerRecipient: PublicKey; // receives the fees
    lookUpTable: PublicKey;
  };
  relayerFee: BN;

  /**
   *
   * @param relayerPubkey Sign the transaction
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
