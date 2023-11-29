/**
 * MIT License
 * Copyright (c) 2020 Barry WhiteHat, Kendrick Tan, Kobi Gurkan, Kirill Goncharov
 * Cory Dickson, Han Jian, Chih-Cheng Liang, and Koh Wei Jie
 * Permission is hereby granted, free of charge, to any person obtaining a copy
 * of this software and associated documentation files (the "Software"), to deal
 * in the Software without restriction, including without limitation the rights
 * to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
 * copies of the Software, and to permit persons to whom the Software is
 * furnished to do so, subject to the following conditions:
 * The above copyright notice and this permission notice shall be included in all
 * copies or substantial portions of the Software.
 * THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
 * IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
 * FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
 * AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
 * LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
 * OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
 * SOFTWARE.
 * This is a custom port of the maci-crypto library.
 * See: https://github.com/privacy-scaling-explorations/maci/blob/aa4ba27f7e415c9d068da7346dff6105cb602fd7/crypto/ts/index.ts
 *
 */

import * as crypto from "crypto";
import assert from "assert";
const eddsa = require("./eddsa");
const ff = require("ffjavascript");
// const createBlakeHash = require("blake-hash");
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

type PrivKey = BigInt;
type PubKey = BigInt[];

/*
 * Convert a BigInt to a Buffer
 */
const bigInt2Buffer = (i: BigInt): Buffer => {
  return Buffer.from(i.toString(16), "hex");
};

// The BN254 group order p
const SNARK_FIELD_SIZE = BigInt(
  "21888242871839275222246405745257275088548364400416034343698204186575808495617",
);

/*
 * Returns a BabyJub-compatible random value. We create it by first generating
 * a random value (initially 256 bits large) modulo the snark field size as
 * described in EIP197. This results in a key size of roughly 253 bits and no
 * more than 254 bits. To prevent modulo bias, we then use this efficient
 * algorithm:
 * http://cvsweb.openbsd.org/cgi-bin/cvsweb/~checkout~/src/lib/libc/crypt/arc4random_uniform.c
 * @return A BabyJub-compatible random value.
 */
const genRandomBabyJubValue = (): BigInt => {
  // Prevent modulo bias
  //const lim = BigInt('0x10000000000000000000000000000000000000000000000000000000000000000')
  //const min = (lim - SNARK_FIELD_SIZE) % SNARK_FIELD_SIZE
  const min = BigInt(
    "6350874878119819312338956282401532410528162663560392320966563075034087161851",
  );

  let rand;
  while (true) {
    rand = BigInt("0x" + crypto.randomBytes(32).toString("hex"));

    if (rand >= min) {
      break;
    }
  }

  const privKey: PrivKey = rand % SNARK_FIELD_SIZE;
  //@ts-ignore
  assert(privKey < SNARK_FIELD_SIZE);

  return privKey;
};

/*
 * @param privKey A private key generated using genPrivKey()
 * @return A public key associated with the private key
 */
const genPubKey = (privKey: PrivKey): PubKey => {
  // Check whether privKey is a field element
  privKey = BigInt(privKey.toString());
  //@ts-ignore
  assert(privKey < SNARK_FIELD_SIZE);
  return eddsa.prv2pub(bigInt2Buffer(privKey));
};

/*
 * @return A BabyJub-compatible private key.
 */
const genPrivKey = (): PrivKey => {
  return genRandomBabyJubValue();
};

/*
 * An internal function which formats a random private key to be compatible
 * with the BabyJub curve. This is the format which should be passed into the
 * PubKey and other circuits.
 */
const formatPrivKeyForBabyJub = (privKey: PrivKey) => {
  const sBuff = eddsa.pruneBuffer(
    blake2b.create(b2params).update(privKey).digest(),
  );
  const s = ff.utils.leBuff2int(sBuff);
  return ff.Scalar.shr(s, 3);
};

/*
 * @return A BabyJub-compatible salt.
 */
const genRandomSalt = (): PrivKey => {
  return genRandomBabyJubValue();
};

export { genPubKey, genPrivKey, formatPrivKeyForBabyJub, genRandomSalt };
