import { AffinePoint } from "@noble/curves/abstract/curve";
import { ExtPointType } from "@noble/curves/abstract/edwards";
import { babyjubjub } from "./babyjubjubConfig";

import {
  genPubKey as generatePublicKey,
  genPrivKey as generateSecretKey,
  formatPrivKeyForBabyJub as formatSecretKey,
  genRandomSalt,
} from "./maci-crypto/maci-crypto-utils";

const ff = require("ffjavascript");

type SecretKey = bigint;
type PublicKey = ExtPointType;

interface Keypair {
  secretKey: SecretKey;
  publicKey: PublicKey;
}

const generateRandomSalt = (): bigint => {
  return genRandomSalt() as bigint;
};

const babyjubjubExt = babyjubjub.ExtendedPoint;

const generateKeypair = (): Keypair => {
  const secretKey = generateSecretKey() as bigint;
  const publicKey = generatePublicKey(secretKey);
  const publicKeyExt = ElGamalUtils.coordinatesToExtPoint<BigInt>(
    publicKey[0],
    publicKey[1],
  );

  const Keypair: Keypair = { secretKey, publicKey: publicKeyExt };

  return Keypair;
};

/**
 * @returns A random extended point on the Baby Jubjub curve.
 */
function generateRandomPoint(): ExtPointType {
  const salt = generateRandomSalt();
  const randomPoint = generatePublicKey(salt);
  const randomPointExt = ElGamalUtils.coordinatesToExtPoint<BigInt>(
    randomPoint[0],
    randomPoint[1],
  );
  return randomPointExt;
}

/**
 * Encrypts a plaintext such that only the owner of the specified public key
 * may decrypt it.
 * @param publicKey The recepient's public key
 * @param plaintext A 32-bit bigint -> to be encoded and then encrypted
 * @param nonce A random value used along with the secret key to generate the ciphertext (optional)
 */
function encrypt(
  publicKey: PublicKey,
  plaintext: bigint,
  nonce: bigint = formatSecretKey(generateRandomSalt()),
) {
  if (publicKey.equals(babyjubjubExt.ZERO)) {
    throw new Error("Invalid Public Key (is zero point)!");
  }
  try {
    publicKey.assertValidity();
  } catch (e) {
    throw new Error("Invalid Public Key!");
  }
  const encodedMessage: ExtPointType = encode(plaintext);

  // The sender calculates an ephemeral key => [nonce].Base
  const ephemeralKey = babyjubjubExt.BASE.multiply(nonce);
  const maskingKey = publicKey.multiply(nonce);
  let ciphertext = encodedMessage.add(maskingKey);

  return { ephemeralKey, ciphertext };
}

/**
 * Decrypts a ciphertext using a secret key.
 * @param secretKey The secret key
 * @param ciphertext The ciphertext to decrypt
 */
function decrypt(
  secretKey: bigint,
  ephemeralKey: ExtPointType,
  ciphertext: ExtPointType,
): ExtPointType {
  // The receiver decrypts the message => ciphertext - [secretKey].ephemeralKey
  const maskingKey = ephemeralKey.multiply(formatSecretKey(secretKey));
  const decryptedMessage = ciphertext.add(maskingKey.negate());

  return decryptedMessage;
}

/**
 * Randomize a ciphertext such that it is different from the original
 * ciphertext but can be decrypted by the same secret key.
 * @param publicKey The same public key used to encrypt the original encodedMessage
 * @param ciphertext The ciphertext to re-randomize.
 * @param randomVal A random value z such that the re-randomized ciphertext could have been generated a random value y+z in the first
 *                  place (optional)
 */
function rerandomize(
  publicKey: PublicKey,
  ephemeralKey: ExtPointType,
  ciphertext: ExtPointType,
  randomVal?: bigint,
) {
  const nonce = randomVal ?? generateRandomSalt();
  const randomizedEphemeralKey = ephemeralKey.add(
    babyjubjubExt.BASE.multiply(nonce as bigint),
  );

  const randomizedEncryptedMessage = ciphertext.add(
    publicKey.multiply(nonce as bigint),
  );

  return { randomizedEphemeralKey, randomizedEncryptedMessage };
}

namespace ElGamalUtils {
  export const stringifyBigInts: (obj: object) => any =
    ff.utils.stringifyBigInts;
  export const unstringifyBigInts: (obj: object) => any =
    ff.utils.unstringifyBigInts;

  /**
   * Convert a Baby Jubjub extended point into an array of two bigints.
   */
  export function toBigIntArray(point: ExtPointType): Array<bigint> {
    const affinePoint = point.toAffine();
    const x = affinePoint.x;
    const y = affinePoint.y;
    return [x, y];
  }

  /**
   * Convert an Baby Jubjub extended point into an array of two strings.
   */
  export function pointToStringArray(point: ExtPointType): Array<string> {
    const affinePoint = point.toAffine();
    const x = affinePoint.x.toString();
    const y = affinePoint.y.toString();
    return [x, y];
  }

  type ConvertibleToBigInt = string | number | bigint | boolean | BigInt;
  /**
   * Convert two strings x and y into a Baby Jubjub extended point.
   */
  export function coordinatesToExtPoint<T extends ConvertibleToBigInt>(
    x: T,
    y: T,
  ): ExtPointType {
    const xBigint = BigInt(x as bigint);
    const yBigint = BigInt(y as bigint);
    const affinePoint: AffinePoint<bigint> = { x: xBigint, y: yBigint };
    const extendedPoint = babyjubjubExt.fromAffine(affinePoint);
    extendedPoint.assertValidity;
    return extendedPoint;
  }
}

/**
 * @param plaintext A 32-bit bigint
 * @returns A point on the Baby Jubjub curve
 */
export function encode(plaintext: bigint): ExtPointType {
  if (plaintext >= BigInt(2 ** 32)) {
    throw new Error("The plaintext should nit be bigger than a 32-bit bigint");
  } else return babyjubjubExt.BASE.multiplyUnsafe(plaintext);
}

export {
  generateRandomPoint,
  generateRandomSalt,
  generateKeypair,
  formatSecretKey,
  encrypt,
  decrypt,
  rerandomize,
  babyjubjubExt,
  PublicKey,
  Keypair,
  ElGamalUtils,
};
