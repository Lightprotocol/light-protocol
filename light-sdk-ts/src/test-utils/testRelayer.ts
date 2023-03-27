import { PublicKey, Connection } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";

export class TestRelayer extends Relayer {
  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient?: PublicKey,
    relayerFee: BN = new BN(0),
  ) {
    super(relayerPubkey, lookUpTable, relayerRecipient, relayerFee);
  }

  async updateMerkleTree(provider: Provider): Promise<any> {
    try {
      const response = await updateMerkleTreeForTest(
        provider.provider?.connection!,
      );
      return response;
    } catch (e) {
      console.log(e);
      throw e;
    }
  }

  static init(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipient: PublicKey,
    relayerFee: BN,
  ): TestRelayer {
    let testRelayer = new TestRelayer(
      relayerPubkey,
      lookUpTable,
      relayerRecipient,
      relayerFee,
    );
    return testRelayer;
  }
}
