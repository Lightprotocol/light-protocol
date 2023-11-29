//@ts-nocheck
/**
 * This is a custom port with select functions of circomlib under GPL-3.0 license.
 * See: https://github.com/weijiekoh/circomlib/blob/24ed08eee0bb613b8c0135d66c1013bd9f78d50a/src/eddsa.js
 */

const babyJub = require("./babyjub");
const ff = require("ffjavascript");
const { blake2b } = require("@noble/hashes/blake2b");
const b2params = { dkLen: 32 };

function pruneBuffer(_buff) {
  const buff = Buffer.from(_buff);
  buff[0] = buff[0] & 0xf8;
  buff[31] = buff[31] & 0x7f;
  buff[31] = buff[31] | 0x40;
  return buff;
}

function prv2pub(prv) {
  const blakeHash = blake2b.create(b2params).update(prv).digest();
  const sBuff = pruneBuffer(blakeHash);
  let s = ff.utils.leBuff2int(sBuff);
  const A = babyJub.mulPointEscalar(babyJub.Base8, ff.Scalar.shr(s, 3));
  return A;
}

export { pruneBuffer, prv2pub };
