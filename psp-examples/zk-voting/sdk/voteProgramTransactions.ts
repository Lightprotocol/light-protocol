import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  TestRelayer,
  Provider,
  circuitlibjs,
  Account,
} from "@lightprotocol/zk.js";
import { Prover } from "@lightprotocol/prover.js";
const { MerkleTree, ElGamalUtils } = circuitlibjs;
const { pointToStringArray, coordinatesToExtPoint } = ElGamalUtils;
import { PublicKey, Keypair } from "@solana/web3.js";
import {
  encrypt,
  PublicKey as ElGamalPublicKey,
  generateKeypair,
  generateRandomSalt,
} from "@lightprotocol/circuit-lib.js";

import { buildPoseidonOpt } from "circomlibjs";
import { BN } from "@coral-xyz/anchor";
import { IDL, PrivateVoting } from "../target/types/private_voting";
import { utils } from "@project-serum/anchor";
const path = require("path");

export type InitVoteTransactionInput = {
  idl: anchor.Idl;
  elGamalPublicKey: ElGamalPublicKey;
  circuitPath: string;
};

export const createInitVoteProof = async (
  voteTransactionInput: InitVoteTransactionInput
) => {
  const { idl, circuitPath, elGamalPublicKey } = voteTransactionInput;
  const yesZeroNonce = generateRandomSalt();
  const { ephemeralKey: zeroYesEmphemeralKey, ciphertext: zeroYesCiphertext } =
    encrypt(elGamalPublicKey, BigInt(0), yesZeroNonce);

  const zeroCiphertextString = pointToStringArray(zeroYesCiphertext);
  const zeroEmphemeralKeyString = pointToStringArray(zeroYesEmphemeralKey);
  const elGamalPublicKeyString = pointToStringArray(elGamalPublicKey);

  const publicInputs = {
    publicElGamalPublicKeyX: new BN(elGamalPublicKeyString[0]),
    publicElGamalPublicKeyY: new BN(elGamalPublicKeyString[1]),
    publicZeroYesEmphemeralKeyX: new BN(zeroEmphemeralKeyString[0]),
    publicZeroYesEmphemeralKeyY: new BN(zeroEmphemeralKeyString[1]),
    publicZeroYesCiphertextX: new BN(zeroCiphertextString[0]),
    publicZeroYesCiphertextY: new BN(zeroCiphertextString[1]),
  };
  const proofInputs = {
    ...publicInputs,
    nonce: new BN(yesZeroNonce.toString()),
  };
  const prover = new Prover(idl, circuitPath, "initVote");
  await prover.addProofInputs(proofInputs);
  console.time("Init vote proof: ");
  const { parsedProof, parsedPublicInputs } = await prover.fullProveAndParse();
  console.timeEnd("Init vote proof: ");
  return { proof: parsedProof, publicInputs: parsedPublicInputs };
};
