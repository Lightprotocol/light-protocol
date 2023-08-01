export type TmpTestPsp = {
  "version": "0.1.0",
  "name": "tmp_test_psp",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS\""
    },
    {
      "name": "NR_CHECKED_INPUTS",
      "type": {
        "defined": "usize"
      },
      "value": "3"
    },
    {
      "name": "VERIFYINGKEY",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 4 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [32 , 25 , 203 , 216 , 1 , 125 , 110 , 116 , 227 , 36 , 175 , 249 , 12 , 122 , 89 , 133 , 118 , 36 , 122 , 139 , 0 , 248 , 139 , 92 , 201 , 99 , 221 , 115 , 90 , 79 , 31 , 164 , 38 , 215 , 179 , 26 , 193 , 238 , 51 , 238 , 167 , 104 , 12 , 165 , 86 , 198 , 108 , 68 , 39 , 160 , 155 , 162 , 126 , 175 , 148 , 176 , 17 , 166 , 68 , 171 , 149 , 58 , 62 , 110 , 15 , 165 , 109 , 129 , 142 , 167 , 235 , 135 , 215 , 173 , 187 , 59 , 233 , 98 , 252 , 120 , 114 , 221 , 80 , 74 , 177 , 130 , 197 , 135 , 25 , 118 , 127 , 190 , 206 , 118 , 101 , 25 , 25 , 96 , 213 , 71 , 190 , 55 , 155 , 236 , 187 , 231 , 152 , 222 , 161 , 11 , 252 , 85 , 14 , 133 , 66 , 242 , 194 , 17 , 121 , 174 , 207 , 53 , 24 , 197 , 181 , 177 , 88 , 104] , vk_ic : & [[13 , 97 , 0 , 106 , 113 , 235 , 68 , 41 , 184 , 32 , 97 , 89 , 83 , 69 , 123 , 192 , 152 , 166 , 33 , 46 , 168 , 76 , 179 , 63 , 75 , 104 , 141 , 189 , 191 , 170 , 236 , 121 , 10 , 119 , 48 , 158 , 23 , 232 , 106 , 187 , 202 , 49 , 118 , 84 , 180 , 125 , 60 , 237 , 133 , 116 , 233 , 96 , 43 , 83 , 8 , 28 , 59 , 35 , 9 , 110 , 151 , 227 , 228 , 117] , [16 , 175 , 99 , 55 , 239 , 168 , 162 , 180 , 79 , 137 , 4 , 246 , 26 , 170 , 226 , 222 , 78 , 220 , 26 , 131 , 13 , 183 , 46 , 88 , 32 , 188 , 17 , 245 , 190 , 177 , 8 , 171 , 46 , 120 , 88 , 114 , 184 , 211 , 142 , 76 , 225 , 245 , 76 , 101 , 0 , 191 , 162 , 102 , 111 , 76 , 103 , 101 , 77 , 81 , 136 , 78 , 92 , 186 , 196 , 248 , 44 , 1 , 173 , 18] , [27 , 177 , 220 , 111 , 226 , 210 , 182 , 112 , 54 , 199 , 197 , 213 , 69 , 56 , 144 , 241 , 152 , 85 , 88 , 41 , 166 , 96 , 7 , 215 , 254 , 229 , 88 , 50 , 253 , 239 , 227 , 112 , 5 , 184 , 123 , 28 , 248 , 192 , 195 , 41 , 200 , 128 , 94 , 100 , 28 , 24 , 224 , 24 , 59 , 121 , 22 , 68 , 204 , 32 , 79 , 216 , 122 , 212 , 222 , 35 , 100 , 179 , 207 , 242] , [39 , 21 , 237 , 236 , 72 , 5 , 55 , 109 , 223 , 114 , 202 , 58 , 132 , 120 , 179 , 144 , 114 , 176 , 211 , 180 , 164 , 225 , 182 , 114 , 18 , 148 , 159 , 68 , 173 , 189 , 157 , 16 , 25 , 101 , 193 , 15 , 79 , 131 , 87 , 91 , 47 , 190 , 23 , 228 , 229 , 222 , 200 , 52 , 101 , 20 , 101 , 130 , 233 , 132 , 95 , 123 , 229 , 21 , 153 , 104 , 72 , 119 , 65 , 184]] , }"
    }
  ],
  "instructions": [
    {
      "name": "lightInstructionFirst",
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
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "lightInstructionSecond",
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "lightInstructionThird",
      "docs": [
        "This instruction is the third step of a shielded transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
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
          "name": "transactionMerkleTree",
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
          "name": "senderSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipientSol",
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
          "isSigner": false
        },
        {
          "name": "verifierProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "closeVerifierState",
      "docs": [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "instructionDataLightInstructionFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "transactionHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
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
    },
    {
      "name": "instructionDataLightInstructionSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "currentSlot",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionThird",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofAApp",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofBApp",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofCApp",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "u256",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "x",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "utxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amounts",
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "splAssetIndex",
            "type": "u64"
          },
          {
            "name": "verifierAddressIndex",
            "type": "u64"
          },
          {
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "appDataHash",
            "type": "u256"
          },
          {
            "name": "accountShieldedPublicKey",
            "type": "u256"
          },
          {
            "name": "accountEncryptionPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "releaseSlot",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "utxoAppData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "releaseSlot",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "zKtmpTestPspMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAppVerifier",
            "type": "u8"
          },
          {
            "name": "transactionHash",
            "type": "u8"
          },
          {
            "name": "currentSlot",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inPublicKey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAppDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inPoolType",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inVerifierPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outAppDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outPoolType",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outVerifierPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "assetPubkeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "transactionVersion",
            "type": "u8"
          },
          {
            "name": "releaseSlot",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "zKtmpTestPspMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAppVerifier",
            "type": "u8"
          },
          {
            "name": "transactionHash",
            "type": "u8"
          },
          {
            "name": "currentSlot",
            "type": "u8"
          }
        ]
      }
    }
  ]
};

export const IDL: TmpTestPsp = {
  "version": "0.1.0",
  "name": "tmp_test_psp",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"Fg6PaFpoGXkYsidMpWTK6W2BeZ7FEfcYkg476zPFsLnS\""
    },
    {
      "name": "NR_CHECKED_INPUTS",
      "type": {
        "defined": "usize"
      },
      "value": "3"
    },
    {
      "name": "VERIFYINGKEY",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 4 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [32 , 25 , 203 , 216 , 1 , 125 , 110 , 116 , 227 , 36 , 175 , 249 , 12 , 122 , 89 , 133 , 118 , 36 , 122 , 139 , 0 , 248 , 139 , 92 , 201 , 99 , 221 , 115 , 90 , 79 , 31 , 164 , 38 , 215 , 179 , 26 , 193 , 238 , 51 , 238 , 167 , 104 , 12 , 165 , 86 , 198 , 108 , 68 , 39 , 160 , 155 , 162 , 126 , 175 , 148 , 176 , 17 , 166 , 68 , 171 , 149 , 58 , 62 , 110 , 15 , 165 , 109 , 129 , 142 , 167 , 235 , 135 , 215 , 173 , 187 , 59 , 233 , 98 , 252 , 120 , 114 , 221 , 80 , 74 , 177 , 130 , 197 , 135 , 25 , 118 , 127 , 190 , 206 , 118 , 101 , 25 , 25 , 96 , 213 , 71 , 190 , 55 , 155 , 236 , 187 , 231 , 152 , 222 , 161 , 11 , 252 , 85 , 14 , 133 , 66 , 242 , 194 , 17 , 121 , 174 , 207 , 53 , 24 , 197 , 181 , 177 , 88 , 104] , vk_ic : & [[13 , 97 , 0 , 106 , 113 , 235 , 68 , 41 , 184 , 32 , 97 , 89 , 83 , 69 , 123 , 192 , 152 , 166 , 33 , 46 , 168 , 76 , 179 , 63 , 75 , 104 , 141 , 189 , 191 , 170 , 236 , 121 , 10 , 119 , 48 , 158 , 23 , 232 , 106 , 187 , 202 , 49 , 118 , 84 , 180 , 125 , 60 , 237 , 133 , 116 , 233 , 96 , 43 , 83 , 8 , 28 , 59 , 35 , 9 , 110 , 151 , 227 , 228 , 117] , [16 , 175 , 99 , 55 , 239 , 168 , 162 , 180 , 79 , 137 , 4 , 246 , 26 , 170 , 226 , 222 , 78 , 220 , 26 , 131 , 13 , 183 , 46 , 88 , 32 , 188 , 17 , 245 , 190 , 177 , 8 , 171 , 46 , 120 , 88 , 114 , 184 , 211 , 142 , 76 , 225 , 245 , 76 , 101 , 0 , 191 , 162 , 102 , 111 , 76 , 103 , 101 , 77 , 81 , 136 , 78 , 92 , 186 , 196 , 248 , 44 , 1 , 173 , 18] , [27 , 177 , 220 , 111 , 226 , 210 , 182 , 112 , 54 , 199 , 197 , 213 , 69 , 56 , 144 , 241 , 152 , 85 , 88 , 41 , 166 , 96 , 7 , 215 , 254 , 229 , 88 , 50 , 253 , 239 , 227 , 112 , 5 , 184 , 123 , 28 , 248 , 192 , 195 , 41 , 200 , 128 , 94 , 100 , 28 , 24 , 224 , 24 , 59 , 121 , 22 , 68 , 204 , 32 , 79 , 216 , 122 , 212 , 222 , 35 , 100 , 179 , 207 , 242] , [39 , 21 , 237 , 236 , 72 , 5 , 55 , 109 , 223 , 114 , 202 , 58 , 132 , 120 , 179 , 144 , 114 , 176 , 211 , 180 , 164 , 225 , 182 , 114 , 18 , 148 , 159 , 68 , 173 , 189 , 157 , 16 , 25 , 101 , 193 , 15 , 79 , 131 , 87 , 91 , 47 , 190 , 23 , 228 , 229 , 222 , 200 , 52 , 101 , 20 , 101 , 130 , 233 , 132 , 95 , 123 , 229 , 21 , 153 , 104 , 72 , 119 , 65 , 184]] , }"
    }
  ],
  "instructions": [
    {
      "name": "lightInstructionFirst",
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
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "systemProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "lightInstructionSecond",
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true,
          "docs": [
            "First transaction, therefore the signing address is not checked but saved to be checked in future instructions."
          ]
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "lightInstructionThird",
      "docs": [
        "This instruction is the third step of a shielded transaction.",
        "The proof is verified with the parameters saved in the first transaction.",
        "At successful verification protocol logic is executed."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
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
          "name": "transactionMerkleTree",
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
          "name": "senderSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSpl",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "senderSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "recipientSol",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "relayerRecipientSol",
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
          "isSigner": false
        },
        {
          "name": "verifierProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "logWrapper",
          "isMut": false,
          "isSigner": false
        }
      ],
      "args": [
        {
          "name": "inputs",
          "type": "bytes"
        }
      ]
    },
    {
      "name": "closeVerifierState",
      "docs": [
        "Close the verifier state to reclaim rent in case the proofdata is wrong and does not verify."
      ],
      "accounts": [
        {
          "name": "signingAddress",
          "isMut": true,
          "isSigner": true
        },
        {
          "name": "verifierState",
          "isMut": true,
          "isSigner": false
        }
      ],
      "args": []
    }
  ],
  "accounts": [
    {
      "name": "instructionDataLightInstructionFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "inputNullifier",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    32
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "publicAmountSol",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "transactionHash",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
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
    },
    {
      "name": "instructionDataLightInstructionSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "currentSlot",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionThird",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofAApp",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofBApp",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofCApp",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                128
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          }
        ]
      }
    },
    {
      "name": "u256",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "x",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          }
        ]
      }
    },
    {
      "name": "utxo",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "amounts",
            "type": {
              "array": [
                "u64",
                2
              ]
            }
          },
          {
            "name": "splAssetIndex",
            "type": "u64"
          },
          {
            "name": "verifierAddressIndex",
            "type": "u64"
          },
          {
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "appDataHash",
            "type": "u256"
          },
          {
            "name": "accountShieldedPublicKey",
            "type": "u256"
          },
          {
            "name": "accountEncryptionPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "releaseSlot",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "utxoAppData",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "releaseSlot",
            "type": "u256"
          }
        ]
      }
    },
    {
      "name": "zKtmpTestPspMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAppVerifier",
            "type": "u8"
          },
          {
            "name": "transactionHash",
            "type": "u8"
          },
          {
            "name": "currentSlot",
            "type": "u8"
          },
          {
            "name": "txIntegrityHash",
            "type": "u8"
          },
          {
            "name": "inAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "inPublicKey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inAppDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inPoolType",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inVerifierPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "inIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outputCommitment",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outAmount",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outAppDataHash",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outIndices",
            "type": {
              "array": [
                {
                  "array": [
                    {
                      "array": [
                        "u8",
                        3
                      ]
                    },
                    2
                  ]
                },
                4
              ]
            }
          },
          {
            "name": "outPoolType",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "outVerifierPubkey",
            "type": {
              "array": [
                "u8",
                4
              ]
            }
          },
          {
            "name": "assetPubkeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "transactionVersion",
            "type": "u8"
          },
          {
            "name": "releaseSlot",
            "type": "u8"
          }
        ]
      }
    },
    {
      "name": "zKtmpTestPspMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicAppVerifier",
            "type": "u8"
          },
          {
            "name": "transactionHash",
            "type": "u8"
          },
          {
            "name": "currentSlot",
            "type": "u8"
          }
        ]
      }
    }
  ]
};
