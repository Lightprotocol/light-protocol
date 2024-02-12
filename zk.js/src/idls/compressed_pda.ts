export type CompressedPda = {
  "version": "0.3.0",
  "name": "compressed_pda",
  "instructions": [
    {
      "name": "initializeUserEntry",
      "accounts": [
        {
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "userEntry",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "lightPubkey",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lightEncryptionPubkey",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "userEntry",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "solanaPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lightPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lightEncryptionPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    }
  ]
};

export const IDL: CompressedPda = {
  "version": "0.3.0",
  "name": "compressed_pda",
  "instructions": [
    {
      "name": "initializeUserEntry",
      "accounts": [
        {
          "name": "signer",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "userEntry",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "lightPubkey",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        },
        {
          "name": "lightEncryptionPubkey",
          "type": {
            "array": [
              "u8",
              32
            ]
          }
        }
      ]
    }
  ],
  "accounts": [
    {
      "name": "userEntry",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "solanaPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lightPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "lightEncryptionPubkey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    }
  ]
};
