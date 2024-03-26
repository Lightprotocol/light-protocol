import { MerkleTree } from "../src";
import { WasmFactory } from "@lightprotocol/hasher.rs";
import axios from "axios";
import { BN } from "@coral-xyz/anchor";
import { assert } from "chai";

describe("Tests", () => {
  const MAX_RETRIES = 20;
  const DELAY_MS = 5000;
  const SERVER_URL = "http://localhost:3001";
  const INCLUSION_PROOF_URL = `${SERVER_URL}/inclusion`;
  const NON_INCLUSION_PROOF_URL = `${SERVER_URL}/noninclusion`;
  const HEALTH_CHECK_URL = `${SERVER_URL}/health`;

  async function pingServer(serverUrl: string) {
    for (let attempt = 0; attempt < MAX_RETRIES; attempt++) {
      try {
        const response = await axios.get(serverUrl);
        if (response.status === 200) {
          console.log("Server is up!");
          break;
        }
      } catch (error) {
        console.log(
          `Attempt ${attempt + 1} failed. Retrying in ${
            DELAY_MS / 1000
          } seconds...`,
        );
      }
      await new Promise((res) => setTimeout(res, DELAY_MS)); // Wait before retrying
    }
  }

  it("inclusion proof", async () => {
    await pingServer(HEALTH_CHECK_URL);

    const hasher = await WasmFactory.getInstance();
    const merkleHeights = [26];
    const utxos = [1, 2, 3, 4, 8];
    for (let i = 0; i < merkleHeights.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        const leaf = hasher.poseidonHashString(["1"]);
        const merkleTree = new MerkleTree(merkleHeights[i], hasher, [leaf]);

        const pathElements: string[] = merkleTree.path(
          merkleTree.indexOf(leaf),
        ).pathElements;
        const hexPathElements = pathElements.map((value) => toHex(value));
        let inputs = {
          root: new Array(utxos[j]).fill(toHex(merkleTree.root())),
          inPathIndices: new Array(utxos[j]).fill(merkleTree.indexOf(leaf)),
          inPathElements: new Array(utxos[j]).fill(hexPathElements),
          leaf: new Array(utxos[j]).fill(toHex(leaf)),
        };
        const inputsData = JSON.stringify(inputs);
        console.time(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);
        const response = await axios.post(INCLUSION_PROOF_URL, inputsData);
        console.timeEnd(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);

        assert.equal(response.status, 200);
        assert.isNotEmpty(response.data.toString());
      }
    }
  });

  function toHex(bnString: string) {
    return "0x" + new BN(bnString).toString(16);
  }
});
