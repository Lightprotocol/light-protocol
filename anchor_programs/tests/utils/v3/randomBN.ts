const anchor = require("@project-serum/anchor")

const crypto = require('crypto');
export const randomBN = (nbytes = 31) => new anchor.BN(crypto.randomBytes(nbytes));