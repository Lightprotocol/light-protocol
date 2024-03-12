import { MerkleTree } from "../src";
import { readFileSync, writeFileSync } from "fs";
import { BN } from "@coral-xyz/anchor";
import { WasmFactory } from "@lightprotocol/account.rs";

const snarkjs = require("snarkjs");

describe("Tests", () => {
  function zk(prefix: string, tree_height: number, num_utxos: number): String {
    return `../circuitlib-rs/test-data/${prefix}_${tree_height}_${num_utxos}/${prefix}_${tree_height}_${num_utxos}.zkey`;
  }

  function wasm(
    prefix: string,
    tree_height: number,
    num_utxos: number,
  ): Buffer {
    let path = `../circuitlib-rs/test-data/${prefix}_${tree_height}_${num_utxos}/${prefix}_${tree_height}_${num_utxos}.wasm`;
    return readFileSync(path);
  }

  function witnessGenerator(): any {
    const path = "./utils/witness_calculator.js";
    return require(path);
  }

  it("inclusion merkle proof", async () => {
    const hasher = await WasmFactory.getInstance();
    const merkleHeights = [26];
    const utxos = [1, 2, 3, 4, 8];
    const outPath = "/tmp";
    for (let i = 0; i < merkleHeights.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        const completePathZkey = zk("i", merkleHeights[i], utxos[j]);
        const buffer = wasm("i", merkleHeights[i], utxos[j]);
        const leaf = hasher.poseidonHashString(["1"]);
        const merkleTree = new MerkleTree(merkleHeights[i], hasher, [leaf]);

        let inputs = {
          root: new Array(utxos[j]).fill(merkleTree.root()),
          inPathIndices: new Array(utxos[j]).fill(merkleTree.indexOf(leaf)),
          inPathElements: new Array(utxos[j]).fill(
            merkleTree.path(merkleTree.indexOf(leaf)).pathElements,
          ),
          leaf: new Array(utxos[j]).fill(leaf),
        };

        const inputs_json = JSON.stringify(inputs);
        console.log("inclusion, inputs: ", inputs_json);
        writeFileSync(
          `${outPath}/inputs${merkleHeights[i]}_${utxos[j]}.json`,
          inputs_json,
        );

        let generator = witnessGenerator();
        let witnessCalculator = await generator(buffer);

        console.time(`Witness generation for ${merkleHeights[i]} ${utxos[j]}`);
        let wtns = await witnessCalculator.calculateWTNSBin(inputs, 0);
        console.timeEnd(
          `Witness generation for ${merkleHeights[i]} ${utxos[j]}`,
        );

        console.time(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);
        const { proof, publicSignals } = await snarkjs.groth16.prove(
          completePathZkey,
          wtns,
        );
        console.timeEnd(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);

        // write publicSignals to json file
        const json = JSON.stringify(publicSignals);
        writeFileSync(
          `${outPath}/public_inputs_merkle${merkleHeights[i]}_${utxos[j]}.json`,
          json,
        );

        const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
        const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
        if (res === true) {
          console.log("Verification OK");
        } else {
          console.log("Invalid proof");
          throw new Error("Invalid Proof");
        }
      }
    }
  });

  function hexPoseidonHashString(hasher: any, input: string[]): string {
    return String.prototype.concat(
      "0x",
      new BN(hasher.poseidonHashString(input)).toString(16),
    );
  }

  it("non-inclusion merkle proof", async () => {
    const inputs13 = {
      root: new BN(
        "17028464443381783825701736776944655616686180135429268021720746920802776315116",
      ),
      value: 2,
      leafLowerRangeValue: 1,
      leafHigherRangeValue: 3,
      leafIndex: 2,

      merkleProofHashedIndexedElementLeaf: [
        new BN(
          "11053679896827771114580198143772188067284553252963177433523924515385729065733",
        ),
        new BN(
          "19416991593757494280293241182306682833623412433910034999629278928521307384158",
        ),
        new BN(
          "7423237065226347324353380772367382631490014989348495481811164164159255474657",
        ),
        new BN(
          "11286972368698509976183087595462810875513684078608517520839298933882497716792",
        ),
        new BN(
          "3607627140608796879659380071776844901612302623152076817094415224584923813162",
        ),
        new BN(
          "19712377064642672829441595136074946683621277828620209496774504837737984048981",
        ),
        new BN(
          "20775607673010627194014556968476266066927294572720319469184847051418138353016",
        ),
        new BN(
          "3396914609616007258851405644437304192397291162432396347162513310381425243293",
        ),
        new BN(
          "21551820661461729022865262380882070649935529853313286572328683688269863701601",
        ),
        new BN(
          "6573136701248752079028194407151022595060682063033565181951145966236778420039",
        ),
        new BN(
          "12413880268183407374852357075976609371175688755676981206018884971008854919922",
        ),
        new BN(
          "14271763308400718165336499097156975241954733520325982997864342600795471836726",
        ),
        new BN(
          "20066985985293572387227381049700832219069292839614107140851619262827735677018",
        ),
        new BN(
          "9394776414966240069580838672673694685292165040808226440647796406499139370960",
        ),
        new BN(
          "11331146992410411304059858900317123658895005918277453009197229807340014528524",
        ),
        new BN(
          "15819538789928229930262697811477882737253464456578333862691129291651619515538",
        ),
        new BN(
          "19217088683336594659449020493828377907203207941212636669271704950158751593251",
        ),
        new BN(
          "21035245323335827719745544373081896983162834604456827698288649288827293579666",
        ),
        new BN(
          "6939770416153240137322503476966641397417391950902474480970945462551409848591",
        ),
        new BN(
          "10941962436777715901943463195175331263348098796018438960955633645115732864202",
        ),
        new BN(
          "15019797232609675441998260052101280400536945603062888308240081994073687793470",
        ),
        new BN(
          "11702828337982203149177882813338547876343922920234831094975924378932809409969",
        ),
        new BN(
          "11217067736778784455593535811108456786943573747466706329920902520905755780395",
        ),
        new BN(
          "16072238744996205792852194127671441602062027943016727953216607508365787157389",
        ),
        new BN(
          "17681057402012993898104192736393849603097507831571622013521167331642182653248",
        ),
        new BN(
          "21694045479371014653083846597424257852691458318143380497809004364947786214945",
        ),
      ],
      indexHashedIndexedElementLeaf: 1,
    };
    const merkleHeights = [26];
    const utxos = [1, 2, 3, 4, 8];
    const outPath = "/tmp";
    for (let i = 0; i < merkleHeights.length; i++) {
      for (let j = 0; j < utxos.length; j++) {
        const completePathZkey = zk("ni", merkleHeights[i], utxos[j]);
        const buffer = wasm("ni", merkleHeights[i], utxos[j]);
        const inputs_json = JSON.stringify(inputs13);
        writeFileSync(
          `${outPath}/inputs${merkleHeights[i]}_${utxos[j]}.json`,
          inputs_json,
        );

        let generator = witnessGenerator();
        let witnessCalculator = await generator(buffer);

        console.time(`Witness generation for ${merkleHeights[i]} ${utxos[j]}`);
        let wtns = await witnessCalculator.calculateWTNSBin(inputs13, 0);
        console.timeEnd(
          `Witness generation for ${merkleHeights[i]} ${utxos[j]}`,
        );

        console.time(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);
        const { proof, publicSignals } = await snarkjs.groth16.prove(
          completePathZkey,
          wtns,
        );
        console.timeEnd(`Proof generation for ${merkleHeights[i]} ${utxos[j]}`);

        const json = JSON.stringify(publicSignals);
        writeFileSync(
          `${outPath}/public_inputs_merkle${merkleHeights[i]}_${utxos[j]}.json`,
          json,
        );

        const vKey = await snarkjs.zKey.exportVerificationKey(completePathZkey);
        const res = await snarkjs.groth16.verify(vKey, publicSignals, proof);
        if (res === true) {
          console.log("Verification OK");
        } else {
          console.log("Invalid proof");
          throw new Error("Invalid Proof");
        }
      }
    }
  });
});
