import { Keypair, PublicKey } from "@solana/web3.js";
import { AnchorError, BN } from "@coral-xyz/anchor";
export const ENCRYPTION_KEYPAIR = {
  PublicKey: new Uint8Array([
    45, 218, 154, 197, 141, 144, 160, 47, 100, 67, 150, 144, 22, 128, 18, 23,
    104, 2, 176, 172, 176, 238, 235, 14, 118, 139, 22, 151, 86, 26, 136, 84,
  ]),
  secretKey: new Uint8Array([
    246, 19, 199, 8, 120, 165, 210, 59, 113, 102, 63, 98, 185, 252, 50, 12, 35,
    89, 71, 60, 189, 251, 109, 89, 92, 74, 233, 128, 148, 50, 243, 162,
  ]),
};

export const USER_TOKEN_ACCOUNT = Keypair.fromSecretKey(
  new Uint8Array([
    213, 170, 148, 167, 77, 163, 59, 129, 233, 8, 59, 40, 203, 223, 53, 122,
    242, 95, 5, 9, 102, 7, 50, 204, 117, 74, 106, 114, 106, 225, 37, 203, 222,
    28, 100, 182, 147, 102, 98, 110, 249, 219, 249, 24, 50, 149, 18, 75, 184,
    183, 246, 83, 13, 66, 226, 103, 241, 88, 135, 253, 226, 32, 41, 186,
  ])
);

export const RECIPIENT_TOKEN_ACCOUNT = Keypair.fromSecretKey(
  new Uint8Array([
    242, 215, 38, 124, 190, 226, 219, 18, 34, 111, 222, 22, 105, 139, 168, 50,
    113, 227, 43, 76, 83, 234, 5, 93, 242, 182, 158, 40, 141, 213, 16, 229, 254,
    86, 86, 250, 191, 38, 191, 237, 255, 198, 0, 140, 74, 85, 247, 85, 30, 34,
    76, 42, 114, 252, 102, 230, 216, 107, 44, 225, 133, 40, 17, 6,
  ])
);

export var KEYPAIR_PRIVKEY = new BN(
  "d67b402d88fe6eb59004f4ab53b06a4b9dc72c74a05e60c31a07148eafa95896",
  "hex"
);

export const MINT_PRIVATE_KEY = new Uint8Array([
  194, 220, 38, 233, 140, 177, 44, 255, 131, 7, 129, 209, 20, 230, 130, 41, 128,
  186, 233, 161, 10, 77, 134, 70, 34, 141, 30, 246, 145, 69, 69, 35, 14, 129,
  15, 86, 229, 176, 155, 3, 8, 217, 125, 97, 221, 115, 252, 160, 127, 236, 37,
  229, 116, 84, 111, 6, 5, 182, 141, 86, 7, 23, 246, 215,
]);

export const MINT = new PublicKey([
  14, 129, 15, 86, 229, 176, 155, 3, 8, 217, 125, 97, 221, 115, 252, 160, 127,
  236, 37, 229, 116, 84, 111, 6, 5, 182, 141, 86, 7, 23, 246, 215,
]);

export const PRIVATE_KEY = [
  17, 34, 231, 31, 83, 147, 93, 173, 61, 164, 25, 0, 204, 82, 234, 91, 202, 187,
  228, 110, 146, 97, 112, 131, 180, 164, 96, 220, 57, 207, 65, 107, 2, 99, 226,
  251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105, 144, 72, 121,
  176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];

export const MERKLE_TREE_INIT_AUTHORITY = [
  2, 99, 226, 251, 88, 66, 92, 33, 25, 216, 211, 185, 112, 203, 212, 238, 105,
  144, 72, 121, 176, 253, 106, 168, 115, 158, 154, 188, 62, 255, 166, 81,
];

export const ADMIN_AUTH_KEY: PublicKey = new PublicKey(
  new Uint8Array(MERKLE_TREE_INIT_AUTHORITY)
);

export const ADMIN_AUTH_KEYPAIR: Keypair = Keypair.fromSecretKey(
  new Uint8Array(PRIVATE_KEY)
);

export const userTokenAccount = new PublicKey(
  "CfyD2mSomGrjnyMKWrgNEk1ApaaUvKRDsnQngGkCVTFk"
);
export const recipientTokenAccount = new PublicKey(
  "6RtYrpXTyH98dvTf9ufivkyDG8mF48oMDbhiRW9r5KjD"
);
