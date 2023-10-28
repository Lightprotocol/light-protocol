import { twistedEdwards } from "@noble/curves/abstract/edwards";
import { Field } from "@noble/curves/abstract/modular";
import { sha512 } from "@noble/hashes/sha512";
import { randomBytes } from "@noble/hashes/utils";

const Fp =
  Field(
    21888242871839275222246405745257275088548364400416034343698204186575808495617n,
  );
export const babyjubjub = twistedEdwards({
  a: Fp.create(168700n),
  d: Fp.create(168696n),
  Fp: Fp,
  n: 21888242871839275222246405745257275088614511777268538073601725287587578984328n,
  h: 8n,
  Gx: 5299619240641551281634865583518297030282874472190772894086521144482721001553n,
  Gy: 16950150798460657717958625567821834550301663161624707787222815936182638968203n,
  hash: sha512,
  randomBytes,
} as const);
