import * as anchor from "@coral-xyz/anchor";
import { circuitlibjs } from "@lightprotocol/zk.js";
import { Prover } from "@lightprotocol/prover.js";
const { ElGamalUtils } = circuitlibjs;
const { coordinatesToExtPoint } = ElGamalUtils;
import {
  decrypt,
  decode,
  formatSecretKey,
} from "@lightprotocol/circuit-lib.js";

import { BN } from "@coral-xyz/anchor";

export type PublishDecryptedTallyTransactionInput = {
  idl: anchor.Idl;
  proofInputs: {
    publicVoteWeightNoEmphemeralKeyX: BN;
    publicVoteWeightNoEmphemeralKeyY: BN;
    publicVoteWeightYesEmphemeralKeyX: BN;
    publicVoteWeightYesEmphemeralKeyY: BN;
    publicVoteWeightNoX: BN;
    publicVoteWeightNoY: BN;
    publicVoteWeightYesX: BN;
    publicVoteWeightYesY: BN;
  };
  circuitPath: string;
  secretKey: bigint;
};

export const createPublishDecryptedTallyProof = async (
  voteTransactionInput: PublishDecryptedTallyTransactionInput
) => {
  let directoryPath = "../../circuit-lib/circuit-lib.js/build";
  const fs = require("fs");
  const lookupTable19Path = directoryPath + `/lookupTableBBJub19.json`;
  const lookupTable = JSON.parse(fs.readFileSync(lookupTable19Path));

  const { idl, circuitPath, proofInputs, secretKey } = voteTransactionInput;
  const extPointYesEmphemeralKey = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicVoteWeightYesEmphemeralKeyX.toString()),
    BigInt(proofInputs.publicVoteWeightYesEmphemeralKeyY.toString())
  );
  const extPointNoEmphemeralKey = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicVoteWeightNoEmphemeralKeyX.toString()),
    BigInt(proofInputs.publicVoteWeightNoEmphemeralKeyY.toString())
  );
  const extPointYesCiphertext = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicVoteWeightYesX.toString()),
    BigInt(proofInputs.publicVoteWeightYesY.toString())
  );
  const extPointNoCiphertext = coordinatesToExtPoint<BigInt>(
    BigInt(proofInputs.publicVoteWeightNoX.toString()),
    BigInt(proofInputs.publicVoteWeightNoY.toString())
  );
  const decryptedYes = decrypt(
    secretKey,
    extPointYesEmphemeralKey,
    extPointYesCiphertext
  );

  const decodedYes = decode(decryptedYes, 19, lookupTable);
  const decryptedNo = decrypt(
    secretKey,
    extPointNoEmphemeralKey,
    extPointNoCiphertext
  );

  const decodedNo = decode(decryptedNo, 19, lookupTable);
  const completeProofInputs = {
    ...proofInputs,
    xhiYes: new BN(decodedYes.xhi.toString()),
    xloYes: new BN(decodedYes.xlo.toString()),
    xhiNo: new BN(decodedNo.xhi.toString()),
    xloNo: new BN(decodedNo.xlo.toString()),
    publicNoResult: new BN(decodedNo.value.toString()),
    publicYesResult: new BN(decodedYes.value.toString()),
    secretKey: new BN(formatSecretKey(secretKey)),
  };
  const prover = new Prover(idl, circuitPath, "publishDecryptedTally");
  await prover.addProofInputs(completeProofInputs);
  console.time("Publish decrypted tally proof: ");
  const { parsedProof, parsedPublicInputs } = await prover.fullProveAndParse();
  console.timeEnd("Publish decrypted tally proof: ");
  return { proof: parsedProof, publicInputs: parsedPublicInputs };
};
