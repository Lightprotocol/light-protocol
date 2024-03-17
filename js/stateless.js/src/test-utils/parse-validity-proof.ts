import { BN } from '@coral-xyz/anchor';
import { FIELD_SIZE } from '../constants';
import { CompressedProof_IdlType } from '../state';

interface GnarkProofJson {
  ar: string[];
  bs: string[][];
  krs: string[];
}

type ProofABC = {
  a: Uint8Array;
  b: Uint8Array;
  c: Uint8Array;
};

export function proofFromJsonStruct(json: GnarkProofJson): ProofABC {
  const proofAX = deserializeHexStringToBeBytes(json.ar[0]);
  const proofAY = deserializeHexStringToBeBytes(json.ar[1]);
  const proofA: Uint8Array = new Uint8Array([...proofAX, ...proofAY]);

  const proofBX0 = deserializeHexStringToBeBytes(json.bs[0][0]);
  const proofBX1 = deserializeHexStringToBeBytes(json.bs[0][1]);
  const proofBY0 = deserializeHexStringToBeBytes(json.bs[1][0]);
  const proofBY1 = deserializeHexStringToBeBytes(json.bs[1][1]);
  const proofB: Uint8Array = new Uint8Array([
    ...proofBX0,
    ...proofBX1,
    ...proofBY0,
    ...proofBY1,
  ]);

  const proofCX = deserializeHexStringToBeBytes(json.krs[0]);
  const proofCY = deserializeHexStringToBeBytes(json.krs[1]);
  const proofC: Uint8Array = new Uint8Array([...proofCX, ...proofCY]);

  const proofABC: ProofABC = { a: proofA, b: proofB, c: proofC };
  return proofABC;
}

// TODO: test if LE BE issue. unit test
export function negateAndCompressProof(
  proof: ProofABC,
): CompressedProof_IdlType {
  const proofA = proof.a;
  const proofB = proof.b;
  const proofC = proof.c;

  const aXElement = proofA.slice(0, 32);
  const aYElement = new BN(proofA.slice(32, 64));

  /// Negate
  const proofAIsPositive = yElementIsPositiveG1(aYElement) ? false : true;
  /// First byte of proofA is the bitmask
  proofA[0] = addBitmaskToByte(proofA[0], proofAIsPositive);

  const bXElement = proofB.slice(0, 64);
  const bYElement = proofB.slice(64, 128);

  // const proofB = mydata.pi_b[0].flat().reverse();
  // const proofBY = mydata.pi_b[1].flat().reverse();
  const proofBIsPositive = yElementIsPositiveG2(
    new BN(bYElement.slice(0, 32)),
    new BN(bYElement.slice(32, 64)),
  );

  bXElement[0] = addBitmaskToByte(bXElement[0], proofBIsPositive);

  const cXElement = proofC.slice(0, 32);
  const cYElement = proofC.slice(32, 64);
  const proofCIsPositive = yElementIsPositiveG1(new BN(cYElement));
  cXElement[0] = addBitmaskToByte(cXElement[0], proofCIsPositive);

  const compressedProof: CompressedProof_IdlType = {
    a: Array.from(aXElement),
    b: Array.from(bXElement),
    c: Array.from(cXElement),
  };

  return compressedProof;
}

function deserializeHexStringToBeBytes(hexStr: string): Uint8Array {
  const trimmedStr = hexStr.startsWith('0x') ? hexStr.substring(2) : hexStr;
  const bigInt = BigInt(`0x${trimmedStr}`);
  const bigIntBytes = new Uint8Array(
    bigInt
      .toString(16)
      .padStart(64, '0')
      .match(/.{1,2}/g)!
      .map((byte) => parseInt(byte, 16)),
  );
  if (bigIntBytes.length < 32) {
    const result = new Uint8Array(32);
    result.set(bigIntBytes, 32 - bigIntBytes.length);
    return result;
  } else {
    return bigIntBytes.slice(0, 32);
  }
}

function yElementIsPositiveG1(yElement: BN): boolean {
  return yElement.lte(FIELD_SIZE.sub(yElement));
}

function yElementIsPositiveG2(yElement1: BN, yElement2: BN): boolean {
  const fieldMidpoint = FIELD_SIZE.div(new BN(2));

  // Compare the first component of the y coordinate
  if (yElement1.lt(fieldMidpoint)) {
    return true;
  } else if (yElement1.gt(fieldMidpoint)) {
    return false;
  }

  // If the first component is equal to the midpoint, compare the second component
  return yElement2.lt(fieldMidpoint);
}
// bitmask compatible with solana altbn128 compression syscall and arkworks' implementation
// https://github.com/arkworks-rs/algebra/blob/master/ff/src/fields/models/fp/mod.rs#L580
// https://github.com/arkworks-rs/algebra/blob/master/serialize/src/flags.rs#L18
// fn u8_bitmask(value: u8, inf: bool, neg: bool) -> u8 {
//     let mut mask = 0;
//     match self {
//         inf => mask |= 1 << 6,
//         neg => mask |= 1 << 7,
//         _ => (),
//     }
//     mask
// }
function addBitmaskToByte(byte: number, yIsPositive: boolean): number {
  if (!yIsPositive) {
    return (byte |= 1 << 7);
  } else {
    return byte;
  }
}
