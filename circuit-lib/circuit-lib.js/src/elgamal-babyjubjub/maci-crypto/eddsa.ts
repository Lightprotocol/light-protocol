//@ts-nocheck
/**
 * Ported from circomlib/eddsa so that we don't have to use maci-crypto
 */
const createBlakeHash = require("blake-hash");
const babyJub = require("./babyjub");

function pruneBuffer(_buff) {
  const buff = Buffer.from(_buff);
  buff[0] = buff[0] & 0xf8;
  buff[31] = buff[31] & 0x7f;
  buff[31] = buff[31] | 0x40;
  return buff;
}

function prv2pub(prv) {
  const sBuff = pruneBuffer(
    createBlakeHash("blake512").update(prv).digest().slice(0, 32),
  );
  let s = utils.leBuff2int(sBuff);
  const A = babyJub.mulPointEscalar(babyJub.Base8, Scalar.shr(s, 3));
  return A;
}
