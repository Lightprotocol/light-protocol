import { PublicKey } from "@solana/web3.js";
import * as anchor from "@coral-xyz/anchor";
import { Program } from "@coral-xyz/anchor";
import { Prover } from "@lightprotocol/prover.js";
import { IDL } from "../target/types/{{rust-name}}";
const circomlibjs = require("circomlibjs");


const RPC_URL = "http://127.0.0.1:8899";

describe("Test {{project-name}}", () => {
  process.env.ANCHOR_PROVIDER_URL = RPC_URL;
  process.env.ANCHOR_WALLET = process.env.HOME + "/.config/solana/id.json";

  // Configure the client to use the local cluster.
  const provider = anchor.AnchorProvider.local(RPC_URL);
  anchor.setProvider(provider);
  const program = new Program(IDL, getProgramId(IDL));

  it("Prover example", async () => {
    const poseidon = await circomlibjs.buildPoseidon();
    const hash = poseidon.F.toString(poseidon([123]));

    const circuitsPath: string = "build-circuit";
    const proofInputs: any = {
      x: 123,
      hash: hash,
    };

    const prover = new Prover(IDL, circuitsPath, "{{circom-name-camel-case}}");

    await prover.addProofInputs(proofInputs);
    await prover.fullProve();
    await prover.getVkey();

    console.time("Proof generation + Parsing");
    let { parsedProof: proof, parsedPublicInputs: publicInputs } =
      await prover.fullProveAndParse();
    console.timeEnd("Proof generation + Parsing");

    console.log("public inputs: ", publicInputs);
    console.dir(proof, { maxArrayLength: null });

    try {
      const tx = await program.methods
        .verifyProof(
          publicInputs,
          proof.proofA,
          proof.proofB.flat(),
          proof.proofC
        )
        .rpc();
      console.log("Transaction has been sent, awaiting confirmation...");
      console.log("Your transaction signature", tx);
    } catch (e) {
      console.log("Error: ", e);
    } 
  });
});

function getProgramId(idl: anchor.Idl): PublicKey {
  const programIdObj = idl.constants!.find(
    (constant) => constant.name === "PROGRAM_ID"
  );
  if (!programIdObj || typeof programIdObj.value !== "string") {
    throw new Error(
      'PROGRAM_ID constant not found in idl. Example: pub const PROGRAM_ID: &str = "Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS";'
    );
  }
  const programIdStr = programIdObj.value.slice(1, -1);
  return new PublicKey(programIdStr);
}