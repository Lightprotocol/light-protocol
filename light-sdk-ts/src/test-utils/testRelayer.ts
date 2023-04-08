import { PublicKey } from "@solana/web3.js";
import { BN } from "@coral-xyz/anchor";
import { Relayer } from "../relayer";
import { updateMerkleTreeForTest } from "./updateMerkleTree";
import { Provider } from "../wallet";
import { sendVersionedTransaction } from "../index";
export class TestRelayer extends Relayer {
  constructor(
    relayerPubkey: PublicKey,
    lookUpTable: PublicKey,
    relayerRecipientSol?: PublicKey,
    relayerFee: BN = new BN(0),
    highRelayerFee?: BN,
  ) {
    super(
      relayerPubkey,
      lookUpTable,
      relayerRecipientSol,
      relayerFee,
      highRelayerFee,
    );
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

  async sendTransaction(instruction: any, provider: Provider): Promise<any> {
    try {
      if (!provider.provider) throw new Error("no provider set");
      const response = await sendVersionedTransaction(instruction, provider);
      return response;
    } catch (err) {
      console.error("erorr here =========>", { err });
      throw err;
    }
  }
}
