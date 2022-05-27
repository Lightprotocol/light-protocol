import * as fs from "fs";
const { wtns, groth16, zKey } = require("snarkjs");
const { utils } = require("ffjavascript");

const util = require("util");
// const fs = require("fs");
async function prove(input, keyBasePath) {
  console.time("Proof generation");
  // console.log("inNullifier:", utils.stringifyBigInts(input).inputNullifier);
  // console.log("root:", utils.stringifyBigInts(input).root);
  // console.log(
  //   "outCommitments:",
  //   utils.stringifyBigInts(input).outputCommitment,
  // );

  const { proof, publicSignals } = await groth16.fullProve(
    utils.stringifyBigInts(input),
    `${keyBasePath}.wasm`,
    `${keyBasePath}.zkey`,
  );
  let publicInputsJson = JSON.stringify(publicSignals, null, 1);
  console.timeEnd("Proof generation");

  let vKey = await zKey.exportVerificationKey(`./final_transaction2.zkey`);
  const res = await groth16.verify(vKey, publicSignals, proof);
  // const vKey = JSON.parse(fs.readFileSync("./verification_key.json"));
  // const res = await groth16.verify(vKey, testSigs, testProof);

  if (res === true) {
    console.log("Verification OK");
  } else {
    console.log("Invalid proof");
    throw new Error("Invalid Proof");
  }

  let proofJson = JSON.stringify(proof, null, 1);

  return { proofJson, publicInputsJson };
}

// function proveZkutil(input, keyBasePath) {
//   input = utils.stringifyBigInts(input);
//   return tmp.dir().then(async (dir) => {
//     dir = dir.path;
//     let out;

//     try {
//       await wtns.debug(
//         utils.unstringifyBigInts(input),
//         `${keyBasePath}.wasm`,
//         `${dir}/witness.wtns`,
//         `${keyBasePath}.sym`,
//         {}
//       );
//       const witness = utils.stringifyBigInts(
//         await wtns.exportJson(`${dir}/witness.wtns`)
//       );
//       fs.writeFileSync(`${dir}/witness.json`, JSON.stringify(witness, null, 2));
//       // console.log(`${dir}/witness.json`);
//       out = await exec(
//         `zkutil prove -c ${keyBasePath}.r1cs -p ${keyBasePath}.params -w ${dir}/witness.json -r ${dir}/proof.json -o ${dir}/public.json`
//       );
//       // TODO: catch inconsistent input during witness generation
//       await exec(
//         `zkutil verify -p ${keyBasePath}.params -r ${dir}/proof.json -i ${dir}/public.json`
//       );
//     } catch (e) {
//       console.log(out, e);
//       throw e;
//     }
//     // console.log(`${dir}/proof.json`);
//     return (
//       "0x" + JSON.parse(fs.readFileSync(`${dir}/proof.json`).toString()).proof
//     );
//   });
// }

module.exports = {
  prove,
  //proveZkutil
};
