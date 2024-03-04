"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
const expect = require("chai").expect;
const chai = require("chai");
const chaiAsPromised = require("chai-as-promised");
const wasm_tester = require("circom_tester").wasm;
// Load chai-as-promised support
chai.use(chaiAsPromised);
const chai_1 = require("chai");
const elgamal_babyjubjub_1 = require("../../../circuit-lib.js/src/elgamal-babyjubjub");
const test_utils_1 = require("../../../circuit-lib.js/tests/elgamal-babyjubjub/test-utils");
const { pointToStringArray, stringifyBigInts, toBigIntArray, coordinatesToExtPoint, } = elgamal_babyjubjub_1.ElGamalUtils;
const genCircuitInputs = (keypair, plaintext) => {
    plaintext || (plaintext = (0, test_utils_1.getRandomBytes)(4));
    const encodedMessage = (0, elgamal_babyjubjub_1.encode)(plaintext);
    const nonce = (0, elgamal_babyjubjub_1.formatSecretKey)((0, elgamal_babyjubjub_1.generateRandomSalt)());
    const encryption = (0, elgamal_babyjubjub_1.encrypt)(keypair.publicKey, plaintext, nonce);
    let input_encrypt = stringifyBigInts({
        message: toBigIntArray(encodedMessage),
        nonceKey: nonce,
        publicKey: toBigIntArray(keypair.publicKey),
    });
    const ephemeralKey = pointToStringArray(encryption.ephemeralKey);
    const ciphertext = pointToStringArray(encryption.ciphertext);
    return { input_encrypt, ephemeralKey, ciphertext };
};
const loadCircuit = async (circuit, inputs_object, witness_return = false) => {
    const witness = await circuit.calculateWitness(inputs_object, true);
    await circuit.checkConstraints(witness);
    await circuit.loadSymbols();
    if (witness_return)
        return witness;
};
const securityCheck = async (circuit, invalid_input, errorMessage) => {
    try {
        await loadCircuit(circuit, invalid_input);
        throw new Error("Expected to throw an error");
    }
    catch (error) {
        expect(error.message).to.contain(errorMessage);
    }
};
describe("ElGamal Circuits Tests", () => {
    let encryptCircuit;
    let decryptCircuit;
    before(async () => {
        const options = {
            include: ["node_modules/circomlib/circuits"],
        };
        encryptCircuit = await wasm_tester("./test-circuits/encrypt_test.circom", options);
        decryptCircuit = await wasm_tester("./test-circuits/decrypt_test.circom", options);
    });
    context("Testing Encrypt Circuit", () => {
        let input_encrypt;
        let keypair;
        let ephemeralKey;
        let ciphertext;
        let encrypt_witness;
        before(async () => {
            keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
            const object = genCircuitInputs(keypair);
            input_encrypt = object.input_encrypt;
            ephemeralKey = object.ephemeralKey;
            ciphertext = object.ciphertext;
            encrypt_witness = await encryptCircuit.calculateWitness(input_encrypt, true);
        });
        it("Encrypt circuit functional", async () => {
            // Verify compliant encryption output for the ephemeral key
            await encryptCircuit.assertOut(encrypt_witness, {
                ephemeralKey: ephemeralKey,
            });
            // Verify compliant encryption output for the encrypted message
            await encryptCircuit.assertOut(encrypt_witness, {
                ciphertext: ciphertext,
            });
        });
        it("Encrypt circuit invalid curve attacks should fail: public key not on curve", async () => {
            const invalid_input = {
                message: input_encrypt.message,
                nonceKey: input_encrypt.nonceKey,
                publicKey: ["1", "0"],
            };
            await securityCheck(encryptCircuit, invalid_input, "Error in template Encrypt_19 line: 58");
        });
        it("Encrypt circuit invalid curve attacks should fail: identity", async () => {
            const invalid_input = {
                message: input_encrypt.message,
                nonceKey: input_encrypt.nonceKey,
                publicKey: ["0", "1"],
            };
            await securityCheck(encryptCircuit, invalid_input, "Error in template Encrypt_19 line: 52");
        });
        it("Encrypt circuit should fail: message not on curve", async () => {
            const invalid_input = {
                message: ["1", "0"],
                nonceKey: input_encrypt.nonceKey,
                publicKey: input_encrypt.publicKey,
            };
            await securityCheck(encryptCircuit, invalid_input, "Error in template Encrypt_19 line: 63");
        });
        it("Encrypt circuit decrypt invalid nonce should fail", async () => {
            input_encrypt.nonceKey = (0, elgamal_babyjubjub_1.formatSecretKey)((0, elgamal_babyjubjub_1.generateRandomSalt)());
            const encrypt_witness = await loadCircuit(encryptCircuit, input_encrypt, true);
            // @ts-ignore: chai as promised is not recognized
            await chai_1.assert.isRejected(encryptCircuit.assertOut(encrypt_witness, {
                ephemeralKey: ephemeralKey,
            }));
            // @ts-ignore: chai as promised is not recognized
            await chai_1.assert.isRejected(encryptCircuit.assertOut(encrypt_witness, {
                ciphertext: ciphertext,
            }));
        });
        it("Encrypt circuit functional: 100 random inputs", async () => {
            for (let i = 0; i < 100; i++) {
                keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
                let { input_encrypt, ephemeralKey, ciphertext } = genCircuitInputs(keypair);
                let encrypt_witness = await encryptCircuit.calculateWitness(input_encrypt, true);
                await encryptCircuit.assertOut(encrypt_witness, {
                    ephemeralKey: ephemeralKey,
                });
                await encryptCircuit.assertOut(encrypt_witness, {
                    ciphertext: ciphertext,
                });
            }
        });
    });
    context("Testing Decrypt Circuit", () => {
        let input_decrypt;
        let keypair;
        let ephemeralKey;
        let ciphertext;
        let message;
        let randomPlaintext;
        let decrypt_witness;
        before(async () => {
            keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
            randomPlaintext = (0, test_utils_1.getRandomBytes)(4);
            message = pointToStringArray((0, elgamal_babyjubjub_1.encode)(randomPlaintext));
            const encryption = genCircuitInputs(keypair, randomPlaintext);
            ephemeralKey = encryption.ephemeralKey;
            ciphertext = encryption.ciphertext;
            input_decrypt = {
                ciphertext: ciphertext,
                ephemeralKey: ephemeralKey,
                secretKey: (0, elgamal_babyjubjub_1.formatSecretKey)(keypair.secretKey),
            };
            decrypt_witness = await decryptCircuit.calculateWitness(input_decrypt, true);
        });
        it("Decrypt circuit functional", async () => {
            // Verify compliant decryption output of the decrypted message
            await decryptCircuit.assertOut(decrypt_witness, {
                decryptedMessage: message,
            });
            // Verify compliant decryption input for the encrypted message
            await decryptCircuit.assertOut(decrypt_witness, {
                ciphertext: ciphertext,
            });
        });
        it("Decrypt circuit invalid curve attacks should fail: ciphertext not on curve", async () => {
            const invalid_input = {
                ciphertext: ["1", "0"],
                ephemeralKey: input_decrypt.ephemeralKey,
                secretKey: input_decrypt.secretKey,
            };
            await securityCheck(decryptCircuit, invalid_input, "Error in template Decrypt_13 line: 22");
        });
        it("Decrypt circuit invalid curve attacks should fail: ephemeralKey not on curve", async () => {
            const invalid_input = {
                ciphertext: input_decrypt.ciphertext,
                ephemeralKey: ["1", "0"],
                secretKey: input_decrypt.secretKey,
            };
            await securityCheck(decryptCircuit, invalid_input, "Error in template Decrypt_13 line: 27");
        });
        it("Decrypt circuit decrypt with different secret key should fail", async () => {
            // only modify the private key
            input_decrypt.secretKey = (0, elgamal_babyjubjub_1.formatSecretKey)((0, elgamal_babyjubjub_1.generateRandomSalt)());
            const decrypt_witness = await decryptCircuit.calculateWitness(input_decrypt, true);
            // @ts-ignore: chai as promised is not recognized
            await chai_1.assert.isRejected(decryptCircuit.assertOut(decrypt_witness, {
                decryptedMessage: message,
            }));
        });
        it("Decrypt circuit funtional 100 random inputs", async () => {
            for (let i = 0; i < 100; i++) {
                keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
                randomPlaintext = (0, test_utils_1.getRandomBytes)(4);
                message = pointToStringArray((0, elgamal_babyjubjub_1.encode)(randomPlaintext));
                const object = genCircuitInputs(keypair, randomPlaintext);
                ephemeralKey = object.ephemeralKey;
                ciphertext = object.ciphertext;
                input_decrypt = {
                    ciphertext: ciphertext,
                    ephemeralKey: ephemeralKey,
                    secretKey: (0, elgamal_babyjubjub_1.formatSecretKey)(keypair.secretKey),
                };
                const decrypt_witness = await decryptCircuit.calculateWitness(input_decrypt, true);
                await decryptCircuit.assertOut(decrypt_witness, {
                    decryptedMessage: message,
                });
                await decryptCircuit.assertOut(decrypt_witness, {
                    ciphertext: ciphertext,
                });
            }
        });
    });
    context("Testing compliance of Encrypt/Decrypt circuits: circuit to circuit", () => {
        let input_encrypt;
        let keypair;
        let message;
        let randomPlaintext;
        let encrypt_witness;
        before(async () => {
            keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
            randomPlaintext = (0, test_utils_1.getRandomBytes)(4);
            message = pointToStringArray((0, elgamal_babyjubjub_1.encode)(randomPlaintext));
            let encryption = genCircuitInputs(keypair, randomPlaintext);
            input_encrypt = encryption.input_encrypt;
            encrypt_witness = await loadCircuit(encryptCircuit, input_encrypt, true);
        });
        it("Encrypt + Decrypt Circuits functional", async () => {
            const input_decrypt = {
                ciphertext: [
                    (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ciphertext[0]"),
                    (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ciphertext[1]"),
                ],
                ephemeralKey: [
                    (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ephemeralKey[0]"),
                    (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ephemeralKey[1]"),
                ],
                secretKey: (0, elgamal_babyjubjub_1.formatSecretKey)(keypair.secretKey),
            };
            const decrypt_witness = await decryptCircuit.calculateWitness(input_decrypt, true);
            await decryptCircuit.assertOut(decrypt_witness, {
                decryptedMessage: message,
            });
        });
        it("Encrypt + Decrypt Circuits functional 100 random inputs", async () => {
            for (let i = 0; i < 100; i++) {
                randomPlaintext = (0, test_utils_1.getRandomBytes)(4);
                message = pointToStringArray((0, elgamal_babyjubjub_1.encode)(randomPlaintext));
                keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
                let object = genCircuitInputs(keypair, randomPlaintext);
                let input_encrypt = object.input_encrypt;
                const encrypt_witness = await loadCircuit(encryptCircuit, input_encrypt, true);
                // The input of the decrypt circuit is given by the output of the encrypt circuit
                let input_decrypt = {
                    ciphertext: [
                        (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ciphertext[0]"),
                        (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ciphertext[1]"),
                    ],
                    ephemeralKey: [
                        (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ephemeralKey[0]"),
                        (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt_witness, "ephemeralKey[1]"),
                    ],
                    secretKey: (0, elgamal_babyjubjub_1.formatSecretKey)(keypair.secretKey),
                };
                const decrypt_witness = await loadCircuit(decryptCircuit, input_decrypt, true);
                await decryptCircuit.assertOut(decrypt_witness, {
                    decryptedMessage: message,
                });
            }
        });
        it("Verify the ElGamal homomorphic property of two random messages", async () => {
            const keypair = (0, elgamal_babyjubjub_1.generateKeypair)();
            const randomPlaintext1 = (0, test_utils_1.getRandomBytes)(4);
            const encodedMessage1 = (0, elgamal_babyjubjub_1.encode)(randomPlaintext1);
            const encryption1 = genCircuitInputs(keypair, randomPlaintext1);
            const input_encrypt1 = encryption1.input_encrypt;
            const encrypt1_witness = await loadCircuit(encryptCircuit, input_encrypt1, true);
            const randomPlaintext2 = (0, test_utils_1.getRandomBytes)(4);
            const encodedMessage2 = (0, elgamal_babyjubjub_1.encode)(randomPlaintext2);
            const encryption2 = genCircuitInputs(keypair, randomPlaintext2);
            const input_encrypt2 = encryption2.input_encrypt;
            const encrypt2_witness = await loadCircuit(encryptCircuit, input_encrypt2, true);
            // Take the first encrypted message from the circuit output
            const encrypted_message1 = coordinatesToExtPoint((0, test_utils_1.getSignalByName)(encryptCircuit, encrypt1_witness, "ciphertext[0]"), (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt1_witness, "ciphertext[1]"));
            // Take the second encrypted message from the circuit output
            const encrypted_message2 = coordinatesToExtPoint((0, test_utils_1.getSignalByName)(encryptCircuit, encrypt2_witness, "ciphertext[0]"), (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt2_witness, "ciphertext[1]"));
            // Add both encrypted messages to verify the homomorphic property
            const encrypted_message3 = encrypted_message1.add(encrypted_message2);
            // Proving message is equal to the decrypted(encrypted_message3) => will prove the homomorphic property
            let message3 = encodedMessage1.add(encodedMessage2);
            // Take the first ephemeral key from the circuit output
            const ephemeral_key1 = coordinatesToExtPoint((0, test_utils_1.getSignalByName)(encryptCircuit, encrypt1_witness, "ephemeralKey[0]"), (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt1_witness, "ephemeralKey[1]"));
            // Take the second ephemeral key from the circuit output
            const ephemeral_key2 = coordinatesToExtPoint((0, test_utils_1.getSignalByName)(encryptCircuit, encrypt2_witness, "ephemeralKey[0]"), (0, test_utils_1.getSignalByName)(encryptCircuit, encrypt2_witness, "ephemeralKey[1]"));
            // The ephemeral key for homomorphic decryption should be ephemeral_key1 + ephemeral_key2
            const ephemeral_key3 = ephemeral_key1.add(ephemeral_key2);
            // The input of the decrypt circuit is given by the added outputs of the encrypt circuit for message1 and message2
            const input_decrypt3 = {
                ciphertext: pointToStringArray(encrypted_message3),
                ephemeralKey: pointToStringArray(ephemeral_key3),
                secretKey: (0, elgamal_babyjubjub_1.formatSecretKey)(keypair.secretKey),
            };
            const decrypt_witness = await loadCircuit(decryptCircuit, input_decrypt3, true);
            await decryptCircuit.assertOut(decrypt_witness, {
                decryptedMessage: pointToStringArray(message3),
            });
        });
    });
});
