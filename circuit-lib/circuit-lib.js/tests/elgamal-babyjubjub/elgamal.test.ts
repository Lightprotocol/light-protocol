import { assert, expect } from "chai";
import {
  babyjubjubExt,
  generateKeypair,
  encrypt,
  decrypt,
  rerandomize,
} from "../../src/elgamal-babyjubjub/elgamal";

import { getRandomBytes } from "./test-utils";

import {
  encode,
  decode,
  split64BigInt,
  precompute,
} from "../../src/elgamal-babyjubjub/pointEncoding";

const fs = require("fs");

const TWO_32 = 4294967296n;

describe("Testing ElGamal Scheme on EC points directly", () => {
  it("Check compliance of orignal and decrypted message as points", () => {
    const keypair = generateKeypair();
    const randomPlaintext = getRandomBytes(4);
    const message = encode(randomPlaintext);

    const encryption = encrypt(keypair.publicKey, randomPlaintext);
    const decryptedMessage = decrypt(
      keypair.secretKey,
      encryption.ephemeralKey,
      encryption.ciphertext,
    );
    expect(message.toAffine(), "Decrypted message is different!").deep.equal(
      decryptedMessage.toAffine(),
    );
  });

  it("Check unhappy compliance of orignal and decrypted message as points", () => {
    const keypair = generateKeypair();
    const randomPlaintext = getRandomBytes(4);
    const message = encode(randomPlaintext);

    let encryption = encrypt(keypair.publicKey, randomPlaintext);
    // we just need to modify any of the inputs
    const { randomizedEphemeralKey } = rerandomize(
      keypair.publicKey,
      encryption.ephemeralKey,
      encryption.ciphertext,
    );
    const decryptedMessage = decrypt(
      keypair.secretKey,
      randomizedEphemeralKey,
      encryption.ciphertext,
    );

    expect(message.toAffine(), "Somehting went wrong!").to.not.deep.equal(
      decryptedMessage.toAffine(),
    );
  });

  it("Check LOOPED compliance of orignal and decrypted message as points", () => {
    for (let i = 0; i < 100; i++) {
      let keypair = generateKeypair();
      let randomPlaintext = getRandomBytes(4);
      let message = encode(randomPlaintext);

      let encryption = encrypt(keypair.publicKey, randomPlaintext);
      let decryptedMessage = decrypt(
        keypair.secretKey,
        encryption.ephemeralKey,
        encryption.ciphertext,
      );

      expect(
        message.toAffine(),
        "Decrypted message is different!",
      ).to.deep.equal(decryptedMessage.toAffine());
    }
  });

  it("Check homomorphic properties of the Elgamal Scheme", () => {
    const keypair = generateKeypair();

    const randomPlaintext1 = getRandomBytes(4);
    const message1 = encode(randomPlaintext1);

    const randomPlaintext2 = getRandomBytes(4);
    const message2 = encode(randomPlaintext2);

    const encryption1 = encrypt(keypair.publicKey, randomPlaintext1);
    const encryption2 = encrypt(keypair.publicKey, randomPlaintext2);

    // We want to prove that message3 is equal to decrypted(encryptedMessage3)
    const message3 = message1.add(message2);
    const encryptedMessage3 = encryption1.ciphertext.add(
      encryption2.ciphertext,
    );
    const ephemeralKey3 = encryption1.ephemeralKey.add(
      encryption2.ephemeralKey,
    );

    const decryptedMessage3 = decrypt(
      keypair.secretKey,
      ephemeralKey3,
      encryptedMessage3,
    );

    expect(
      decryptedMessage3.toAffine(),
      "Invalid linear homomorphism!",
    ).to.deep.equal(message3.toAffine());
  });

  it("Check unhappy homomorphic properties for wrong inputs", () => {
    const keypair = generateKeypair();

    const randomPlaintext1 = getRandomBytes(4);
    const message1 = encode(randomPlaintext1);

    const randomPlaintext2 = getRandomBytes(4);
    const message2 = encode(randomPlaintext2);

    const encryption1 = encrypt(keypair.publicKey, randomPlaintext1);
    const encryption2 = encrypt(keypair.publicKey, randomPlaintext2);

    const message3 = message1.add(message2);
    const encryptedMessage3 = encryption1.ciphertext.add(
      encryption2.ciphertext,
    );
    // we only modifiy ephemeralKey3 in this example
    const ephemeralKey3 = encryption1.ephemeralKey.add(babyjubjubExt.BASE);

    const decryptedMessage3 = decrypt(
      keypair.secretKey,
      ephemeralKey3,
      encryptedMessage3,
    );

    expect(
      decryptedMessage3.toAffine(),
      "Invalid linear homomorphism!",
    ).to.not.deep.equal(message3.toAffine());
  });
});

describe("Testing Encoding/Decoding for ElGamal Scheme", async () => {
  let lookupTable: any;
  let directoryPath = "./build";
  const lookupTable19Path = directoryPath + `/lookupTableBBJub19.json`;
  before(() => {
    if (!fs.existsSync(directoryPath)) {
      fs.mkdirSync(directoryPath, { recursive: true });
      console.log(`Directory created: ${directoryPath}`);
    }
    if (!fs.existsSync(lookupTable19Path)) {
      console.log("Building the lookup table file...");
      precompute(19, directoryPath);
      lookupTable = JSON.parse(fs.readFileSync(lookupTable19Path));
    } else lookupTable = JSON.parse(fs.readFileSync(lookupTable19Path));
  });

  it("Check encoding a plain text bigger than 32 bits returns error", () => {
    const plaintext = 4294967297n;
    let expected = Error;
    const exercise = () => encode(plaintext);
    assert.throws(exercise, expected);
  });

  it("Check encoded value is a valid Baby Jubjub point", () => {
    const plaintext = getRandomBytes(4);
    const encoded = encode(plaintext);
    encoded.assertValidity();
  });

  it("Check compliance of orignal and decoded message as 32-bit numbers", async () => {
    const plaintext = getRandomBytes(4);
    const encoded = encode(plaintext);
    const decoded = decode(encoded, 19, lookupTable);

    assert(plaintext === decoded, "Decoded number is different!");
  });

  it("Check LOOPED compliance of orignal and decoded message as 32-bit numbers", async () => {
    for (let i = 0; i < 15; i++) {
      let plaintext = getRandomBytes(4);
      let encoded = encode(plaintext);
      let decoded = decode(encoded, 19, lookupTable);

      assert(plaintext === decoded, "Decoded number is different!");
    }
  });

  it("Check decoding preserves Elgamal linear homomorphism", async () => {
    // The input should be a 64-bit number
    const plaintext = getRandomBytes(8);

    // the initial input is split into two 32-bit numbers for faster decoding
    const [xlo, xhi] = split64BigInt(plaintext);

    const keypair = generateKeypair();

    const encryption1 = encrypt(keypair.publicKey, xlo);
    const encryption2 = encrypt(keypair.publicKey, xhi);

    const decryptedMessage1 = decrypt(
      keypair.secretKey,
      encryption1.ephemeralKey,
      encryption1.ciphertext,
    );
    const decryptedMessage2 = decrypt(
      keypair.secretKey,
      encryption2.ephemeralKey,
      encryption2.ciphertext,
    );

    const xlo_decoded = decode(decryptedMessage1, 19, lookupTable);
    const xhi_decoded = decode(decryptedMessage2, 19, lookupTable);

    const decoded_input = xlo_decoded + TWO_32 * xhi_decoded;

    assert(decoded_input === plaintext, "decoding led to different result!");
  });

  it("Check unhappy decoding breaks Elgamal linear homomorphism", async () => {
    // The input should be a 64-bit number
    const input = getRandomBytes(8);

    // the initial input is split into two 32-bit numbers for faster decoding
    const [xlo, xhi] = split64BigInt(input);

    const keypair = generateKeypair();

    // we swap xlo and xhi to mess with the decoding
    const encryption1 = encrypt(keypair.publicKey, xhi);
    const encryption2 = encrypt(keypair.publicKey, xlo);

    const decryptedMessage1 = decrypt(
      keypair.secretKey,
      encryption1.ephemeralKey,
      encryption1.ciphertext,
    );
    const decryptedMessage2 = decrypt(
      keypair.secretKey,
      encryption2.ephemeralKey,
      encryption2.ciphertext,
    );

    const xl0_decoded = decode(decryptedMessage1, 19, lookupTable);
    const xhi_decoded = decode(decryptedMessage2, 19, lookupTable);

    const decoded_input = xl0_decoded + TWO_32 * xhi_decoded;

    assert(decoded_input !== input, "decoding led to different result!");
  });

  it("Check LOOPED decoding preserves Elgamal linear homomorphism", async () => {
    for (let i = 0; i < 10; i++) {
      // The input should be a 64-bit number
      const input = getRandomBytes(8);

      // the initial input is split into two 32-bit numbers for faster decoding
      let [xlo, xhi] = split64BigInt(input);

      let keypair = generateKeypair();

      const encryption1 = encrypt(keypair.publicKey, xlo);
      const encryption2 = encrypt(keypair.publicKey, xhi);

      const decryptedMessage1 = decrypt(
        keypair.secretKey,
        encryption1.ephemeralKey,
        encryption1.ciphertext,
      );
      const decryptedMessage2 = decrypt(
        keypair.secretKey,
        encryption2.ephemeralKey,
        encryption2.ciphertext,
      );

      const xl0_decoded = decode(decryptedMessage1, 19, lookupTable);
      const xhi_decoded = decode(decryptedMessage2, 19, lookupTable);

      const decoded_input = xl0_decoded + TWO_32 * xhi_decoded;

      assert(decoded_input === input, "decoding led to different result!");
    }
  });
});
