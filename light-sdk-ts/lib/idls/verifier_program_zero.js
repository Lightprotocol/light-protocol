"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.VerifierProgramZero = void 0;
exports.VerifierProgramZero = {
  version: "0.1.0",
  name: "verifier_program_zero",
  instructions: [
    {
      name: "shieldedTransferInputs",
      accounts: [
        {
          name: "signingAddress",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "programMerkleTree",
          isMut: false,
          isSigner: false,
        },
        {
          name: "merkleTree",
          isMut: false,
          isSigner: false,
        },
        {
          name: "preInsertedLeavesIndex",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "sender",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipient",
          isMut: true,
          isSigner: false,
        },
        {
          name: "senderFee",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipientFee",
          isMut: true,
          isSigner: false,
        },
        {
          name: "relayerRecipient",
          isMut: true,
          isSigner: false,
        },
        {
          name: "escrow",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "proof",
          type: "bytes",
        },
        {
          name: "amount",
          type: "bytes",
        },
        {
          name: "nullifiers",
          type: {
            array: [
              {
                array: ["u8", 32],
              },
              2,
            ],
          },
        },
        {
          name: "leaves",
          type: {
            array: [
              {
                array: ["u8", 32],
              },
              2,
            ],
          },
        },
        {
          name: "feeAmount",
          type: "bytes",
        },
        {
          name: "rootIndex",
          type: "u64",
        },
        {
          name: "relayerFee",
          type: "u64",
        },
        {
          name: "encryptedUtxos",
          type: "bytes",
        },
      ],
    },
  ],
};
exports.default = exports.VerifierProgramZero;
