"use strict";
Object.defineProperty(exports, "__esModule", { value: true });
exports.VerifierProgramOne = void 0;
exports.VerifierProgramOne = {
    version: "0.1.0",
    name: "verifier_program_one",
    instructions: [
        {
            name: "shieldedTransferFirst",
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
                    name: "verifierState",
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [
                {
                    name: "publicAmount",
                    type: "bytes",
                },
                {
                    name: "nullifiers",
                    type: {
                        array: [
                            {
                                array: ["u8", 32],
                            },
                            10,
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
        {
            name: "shieldedTransferSecond",
            accounts: [
                {
                    name: "signingAddress",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "verifierState",
                    isMut: true,
                    isSigner: false,
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
            ],
        },
        {
            name: "closeVerifierState",
            accounts: [
                {
                    name: "signingAddress",
                    isMut: true,
                    isSigner: true,
                },
                {
                    name: "verifierState",
                    isMut: true,
                    isSigner: false,
                },
            ],
            args: [],
        },
    ],
};
exports.default = exports.VerifierProgramOne;
