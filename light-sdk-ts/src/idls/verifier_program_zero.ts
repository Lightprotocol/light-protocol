export type VerifierProgramZero = {
  "version": "0.1.0",
  "name": "verifier_program_zero",
  "instructions": [
    {
      "name": "shieldedTransferInputs",
      "docs": [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2"
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "preInsertedLeavesIndex",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "sender",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "escrow",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom."
          ]
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": "bytes"
        },
        {
          "name": "amount",
          "type": "bytes"
        },
        {
          "name": "nullifiers",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              2
            ]
          }
        },
        {
          "name": "leaves",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              2
            ]
          }
        },
        {
          "name": "feeAmount",
          "type": "bytes"
        },
        {
          "name": "rootIndex",
          "type": "u64"
        },
        {
          "name": "relayerFee",
          "type": "u64"
        },
        {
          "name": "encryptedUtxos",
          "type": "bytes"
        }
      ]
    }
  ]
};

export const IDL: VerifierProgramZero = {
  "version": "0.1.0",
  "name": "verifier_program_zero",
  "instructions": [
    {
      "name": "shieldedTransferInputs",
      "docs": [
        "This instruction is the first step of a shieled transaction.",
        "It creates and initializes a verifier state account to save state of a verification during",
        "computation verifying the zero-knowledge proof (ZKP). Additionally, it stores other data",
        "such as leaves, amounts, recipients, nullifiers, etc. to execute the protocol logic",
        "in the last transaction after successful ZKP verification. light_verifier_sdk::light_instruction::LightInstruction2"
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "programMerkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "merkleTree",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "preInsertedLeavesIndex",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "sender",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientFee",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipient",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "escrow",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs ot exist Is not checked the relayer has complete freedom."
          ]
        }
      ],
      "args": [
        {
          "name": "proof",
          "type": "bytes"
        },
        {
          "name": "amount",
          "type": "bytes"
        },
        {
          "name": "nullifiers",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              2
            ]
          }
        },
        {
          "name": "leaves",
          "type": {
            "array": [
              {
                "array": [
                  "u8",
                  32
                ]
              },
              2
            ]
          }
        },
        {
          "name": "feeAmount",
          "type": "bytes"
        },
        {
          "name": "rootIndex",
          "type": "u64"
        },
        {
          "name": "relayerFee",
          "type": "u64"
        },
        {
          "name": "encryptedUtxos",
          "type": "bytes"
        }
      ]
    }
  ]
};
