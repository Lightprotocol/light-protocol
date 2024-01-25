export type LightPsp2in2out = {
  "version": "0.3.1",
  "name": "light_psp2in2out",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i\""
    },
    {
      "name": "VERIFYINGKEY_PRIVATE_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [29 , 190 , 79 , 228 , 28 , 230 , 219 , 109 , 243 , 244 , 163 , 60 , 133 , 129 , 220 , 67 , 178 , 255 , 76 , 20 , 58 , 149 , 244 , 62 , 207 , 45 , 222 , 91 , 120 , 105 , 218 , 213 , 2 , 210 , 175 , 224 , 36 , 1 , 53 , 94 , 37 , 135 , 9 , 202 , 134 , 20 , 95 , 142 , 205 , 147 , 211 , 141 , 104 , 13 , 112 , 219 , 221 , 10 , 186 , 175 , 98 , 243 , 161 , 0 , 42 , 150 , 89 , 138 , 33 , 162 , 204 , 170 , 1 , 202 , 162 , 232 , 75 , 51 , 109 , 255 , 132 , 233 , 195 , 108 , 235 , 85 , 53 , 183 , 222 , 222 , 179 , 193 , 17 , 215 , 207 , 238 , 0 , 100 , 124 , 125 , 171 , 228 , 134 , 72 , 82 , 40 , 249 , 199 , 14 , 201 , 23 , 101 , 195 , 217 , 160 , 34 , 149 , 0 , 131 , 141 , 174 , 25 , 199 , 59 , 131 , 43 , 143 , 212] , vk_ic : & [[28 , 37 , 226 , 89 , 65 , 199 , 152 , 244 , 158 , 177 , 90 , 248 , 253 , 220 , 250 , 160 , 148 , 18 , 110 , 26 , 205 , 150 , 159 , 50 , 3 , 48 , 67 , 205 , 101 , 66 , 128 , 1 , 25 , 105 , 253 , 0 , 228 , 21 , 39 , 79 , 143 , 65 , 181 , 136 , 196 , 17 , 98 , 86 , 243 , 145 , 250 , 80 , 55 , 197 , 202 , 232 , 54 , 180 , 122 , 195 , 35 , 101 , 238 , 164] , [9 , 147 , 30 , 143 , 5 , 59 , 180 , 110 , 222 , 34 , 1 , 162 , 231 , 214 , 144 , 103 , 185 , 219 , 125 , 192 , 152 , 109 , 105 , 32 , 161 , 72 , 126 , 50 , 51 , 43 , 108 , 103 , 25 , 81 , 25 , 139 , 182 , 226 , 242 , 217 , 183 , 152 , 150 , 66 , 233 , 160 , 86 , 62 , 23 , 109 , 190 , 76 , 104 , 38 , 143 , 233 , 40 , 208 , 47 , 121 , 136 , 10 , 198 , 244] , [46 , 8 , 32 , 136 , 114 , 38 , 102 , 51 , 51 , 235 , 168 , 213 , 190 , 193 , 216 , 247 , 114 , 76 , 139 , 31 , 109 , 67 , 86 , 243 , 3 , 57 , 22 , 227 , 14 , 1 , 94 , 138 , 18 , 83 , 164 , 33 , 72 , 79 , 86 , 141 , 118 , 23 , 229 , 169 , 105 , 132 , 246 , 103 , 250 , 35 , 0 , 230 , 209 , 198 , 10 , 30 , 74 , 237 , 169 , 76 , 48 , 64 , 129 , 163] , [9 , 243 , 97 , 84 , 183 , 242 , 102 , 95 , 87 , 87 , 26 , 243 , 183 , 9 , 86 , 22 , 21 , 92 , 59 , 155 , 124 , 194 , 90 , 218 , 209 , 200 , 224 , 134 , 232 , 223 , 107 , 61 , 36 , 198 , 236 , 31 , 23 , 152 , 58 , 168 , 202 , 206 , 13 , 206 , 162 , 138 , 63 , 33 , 12 , 20 , 50 , 122 , 221 , 173 , 212 , 15 , 130 , 98 , 146 , 68 , 205 , 0 , 201 , 57] , [22 , 229 , 209 , 227 , 185 , 143 , 171 , 173 , 56 , 29 , 91 , 143 , 21 , 186 , 69 , 152 , 75 , 102 , 15 , 246 , 61 , 224 , 83 , 243 , 51 , 12 , 0 , 94 , 234 , 181 , 138 , 98 , 23 , 201 , 215 , 132 , 43 , 36 , 197 , 171 , 139 , 110 , 4 , 191 , 21 , 250 , 233 , 212 , 136 , 223 , 164 , 8 , 108 , 150 , 56 , 174 , 13 , 67 , 217 , 83 , 80 , 114 , 125 , 236] , [1 , 137 , 95 , 63 , 9 , 195 , 121 , 7 , 172 , 116 , 83 , 248 , 117 , 222 , 253 , 246 , 189 , 171 , 120 , 57 , 73 , 250 , 32 , 9 , 106 , 32 , 128 , 217 , 193 , 49 , 86 , 165 , 12 , 175 , 151 , 15 , 162 , 122 , 139 , 140 , 34 , 100 , 227 , 151 , 152 , 123 , 217 , 27 , 18 , 245 , 180 , 193 , 52 , 75 , 90 , 247 , 159 , 202 , 102 , 217 , 47 , 56 , 249 , 43] , [39 , 175 , 107 , 14 , 5 , 36 , 130 , 255 , 75 , 7 , 14 , 183 , 215 , 7 , 140 , 222 , 19 , 94 , 56 , 133 , 117 , 231 , 76 , 158 , 24 , 143 , 74 , 46 , 37 , 166 , 106 , 53 , 28 , 171 , 35 , 118 , 106 , 13 , 193 , 82 , 15 , 41 , 14 , 92 , 43 , 114 , 137 , 52 , 230 , 73 , 52 , 220 , 152 , 149 , 56 , 249 , 66 , 48 , 96 , 126 , 118 , 122 , 6 , 38] , [33 , 34 , 189 , 123 , 120 , 215 , 116 , 33 , 49 , 177 , 79 , 129 , 72 , 109 , 12 , 209 , 170 , 154 , 65 , 193 , 135 , 59 , 7 , 181 , 81 , 139 , 28 , 26 , 57 , 29 , 38 , 24 , 37 , 151 , 214 , 176 , 144 , 238 , 67 , 184 , 172 , 75 , 22 , 126 , 215 , 66 , 41 , 214 , 177 , 172 , 20 , 47 , 141 , 229 , 25 , 76 , 124 , 58 , 35 , 239 , 80 , 69 , 142 , 50] , [3 , 93 , 165 , 112 , 187 , 253 , 117 , 246 , 147 , 130 , 88 , 19 , 50 , 236 , 202 , 13 , 192 , 190 , 233 , 89 , 84 , 57 , 36 , 246 , 240 , 20 , 12 , 200 , 191 , 230 , 41 , 93 , 17 , 94 , 68 , 253 , 51 , 115 , 116 , 234 , 8 , 10 , 192 , 237 , 137 , 238 , 220 , 70 , 66 , 174 , 195 , 0 , 174 , 153 , 225 , 127 , 87 , 24 , 151 , 240 , 160 , 41 , 51 , 0] , [28 , 41 , 227 , 48 , 202 , 209 , 77 , 89 , 53 , 35 , 16 , 181 , 84 , 46 , 55 , 136 , 160 , 149 , 234 , 93 , 70 , 26 , 67 , 205 , 79 , 113 , 76 , 255 , 12 , 171 , 150 , 47 , 40 , 220 , 239 , 166 , 11 , 51 , 205 , 213 , 126 , 215 , 77 , 131 , 206 , 193 , 19 , 36 , 163 , 229 , 32 , 215 , 53 , 46 , 182 , 86 , 31 , 47 , 85 , 100 , 109 , 46 , 181 , 241] , [27 , 119 , 74 , 10 , 102 , 108 , 48 , 137 , 141 , 79 , 201 , 85 , 180 , 153 , 101 , 87 , 185 , 146 , 118 , 135 , 91 , 72 , 117 , 86 , 67 , 201 , 199 , 139 , 250 , 68 , 215 , 20 , 12 , 43 , 111 , 206 , 160 , 37 , 113 , 209 , 161 , 188 , 234 , 36 , 129 , 216 , 164 , 79 , 64 , 92 , 60 , 136 , 248 , 212 , 251 , 77 , 115 , 70 , 119 , 225 , 195 , 205 , 9 , 181] , [29 , 32 , 39 , 44 , 114 , 192 , 193 , 126 , 180 , 96 , 142 , 124 , 93 , 165 , 37 , 202 , 245 , 36 , 126 , 159 , 157 , 132 , 110 , 40 , 69 , 180 , 179 , 8 , 7 , 85 , 237 , 246 , 46 , 15 , 174 , 41 , 75 , 66 , 155 , 185 , 144 , 203 , 15 , 191 , 52 , 224 , 176 , 156 , 33 , 44 , 72 , 31 , 34 , 129 , 110 , 242 , 184 , 196 , 109 , 248 , 39 , 249 , 254 , 184] , [44 , 251 , 166 , 115 , 98 , 51 , 141 , 19 , 156 , 235 , 68 , 71 , 191 , 152 , 4 , 185 , 218 , 164 , 236 , 197 , 145 , 66 , 85 , 23 , 233 , 87 , 136 , 221 , 202 , 12 , 145 , 168 , 32 , 246 , 0 , 50 , 31 , 81 , 93 , 154 , 38 , 25 , 17 , 65 , 217 , 20 , 43 , 216 , 226 , 19 , 157 , 96 , 69 , 35 , 242 , 209 , 219 , 254 , 6 , 62 , 192 , 98 , 160 , 226]] , }"
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferFirst",
      "docs": [
        "This instruction is the first step of a compressed transaction.",
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
          "name": "merkleTreeSet",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "rpcRecipientSol",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
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
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs to exist."
          ]
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
    }
  ],
  "accounts": [
    {
      "name": "instructionDataCompressedTransferFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
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
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
            "name": "publicAmountSol",
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
            "name": "rpcFee",
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
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "dataHash",
            "type": "u256"
          },
          {
            "name": "accountCompressionPublicKey",
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
          }
        ]
      }
    },
    {
      "name": "outUtxo",
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
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "utxoDataHash",
            "type": "u256"
          },
          {
            "name": "accountCompressionPublicKey",
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
            "name": "isFillingUtxo",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
          },
          {
            "name": "address",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "metaHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
                2
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "merkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                2
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
                2
              ]
            }
          },
          {
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                2
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
                2
              ]
            }
          },
          {
            "name": "outOwner",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outDataHash",
            "type": {
              "array": [
                "u8",
                2
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
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionPrivateTransaction2In2OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicDataHash",
            "type": {
              "array": [
                "u8",
                32
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
          }
        ]
      }
    }
  ]
};

export const IDL: LightPsp2in2out = {
  "version": "0.3.1",
  "name": "light_psp2in2out",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"J1RRetZ4ujphU75LP8RadjXMf3sA12yC2R44CF7PmU7i\""
    },
    {
      "name": "VERIFYINGKEY_PRIVATE_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [29 , 190 , 79 , 228 , 28 , 230 , 219 , 109 , 243 , 244 , 163 , 60 , 133 , 129 , 220 , 67 , 178 , 255 , 76 , 20 , 58 , 149 , 244 , 62 , 207 , 45 , 222 , 91 , 120 , 105 , 218 , 213 , 2 , 210 , 175 , 224 , 36 , 1 , 53 , 94 , 37 , 135 , 9 , 202 , 134 , 20 , 95 , 142 , 205 , 147 , 211 , 141 , 104 , 13 , 112 , 219 , 221 , 10 , 186 , 175 , 98 , 243 , 161 , 0 , 42 , 150 , 89 , 138 , 33 , 162 , 204 , 170 , 1 , 202 , 162 , 232 , 75 , 51 , 109 , 255 , 132 , 233 , 195 , 108 , 235 , 85 , 53 , 183 , 222 , 222 , 179 , 193 , 17 , 215 , 207 , 238 , 0 , 100 , 124 , 125 , 171 , 228 , 134 , 72 , 82 , 40 , 249 , 199 , 14 , 201 , 23 , 101 , 195 , 217 , 160 , 34 , 149 , 0 , 131 , 141 , 174 , 25 , 199 , 59 , 131 , 43 , 143 , 212] , vk_ic : & [[28 , 37 , 226 , 89 , 65 , 199 , 152 , 244 , 158 , 177 , 90 , 248 , 253 , 220 , 250 , 160 , 148 , 18 , 110 , 26 , 205 , 150 , 159 , 50 , 3 , 48 , 67 , 205 , 101 , 66 , 128 , 1 , 25 , 105 , 253 , 0 , 228 , 21 , 39 , 79 , 143 , 65 , 181 , 136 , 196 , 17 , 98 , 86 , 243 , 145 , 250 , 80 , 55 , 197 , 202 , 232 , 54 , 180 , 122 , 195 , 35 , 101 , 238 , 164] , [9 , 147 , 30 , 143 , 5 , 59 , 180 , 110 , 222 , 34 , 1 , 162 , 231 , 214 , 144 , 103 , 185 , 219 , 125 , 192 , 152 , 109 , 105 , 32 , 161 , 72 , 126 , 50 , 51 , 43 , 108 , 103 , 25 , 81 , 25 , 139 , 182 , 226 , 242 , 217 , 183 , 152 , 150 , 66 , 233 , 160 , 86 , 62 , 23 , 109 , 190 , 76 , 104 , 38 , 143 , 233 , 40 , 208 , 47 , 121 , 136 , 10 , 198 , 244] , [46 , 8 , 32 , 136 , 114 , 38 , 102 , 51 , 51 , 235 , 168 , 213 , 190 , 193 , 216 , 247 , 114 , 76 , 139 , 31 , 109 , 67 , 86 , 243 , 3 , 57 , 22 , 227 , 14 , 1 , 94 , 138 , 18 , 83 , 164 , 33 , 72 , 79 , 86 , 141 , 118 , 23 , 229 , 169 , 105 , 132 , 246 , 103 , 250 , 35 , 0 , 230 , 209 , 198 , 10 , 30 , 74 , 237 , 169 , 76 , 48 , 64 , 129 , 163] , [9 , 243 , 97 , 84 , 183 , 242 , 102 , 95 , 87 , 87 , 26 , 243 , 183 , 9 , 86 , 22 , 21 , 92 , 59 , 155 , 124 , 194 , 90 , 218 , 209 , 200 , 224 , 134 , 232 , 223 , 107 , 61 , 36 , 198 , 236 , 31 , 23 , 152 , 58 , 168 , 202 , 206 , 13 , 206 , 162 , 138 , 63 , 33 , 12 , 20 , 50 , 122 , 221 , 173 , 212 , 15 , 130 , 98 , 146 , 68 , 205 , 0 , 201 , 57] , [22 , 229 , 209 , 227 , 185 , 143 , 171 , 173 , 56 , 29 , 91 , 143 , 21 , 186 , 69 , 152 , 75 , 102 , 15 , 246 , 61 , 224 , 83 , 243 , 51 , 12 , 0 , 94 , 234 , 181 , 138 , 98 , 23 , 201 , 215 , 132 , 43 , 36 , 197 , 171 , 139 , 110 , 4 , 191 , 21 , 250 , 233 , 212 , 136 , 223 , 164 , 8 , 108 , 150 , 56 , 174 , 13 , 67 , 217 , 83 , 80 , 114 , 125 , 236] , [1 , 137 , 95 , 63 , 9 , 195 , 121 , 7 , 172 , 116 , 83 , 248 , 117 , 222 , 253 , 246 , 189 , 171 , 120 , 57 , 73 , 250 , 32 , 9 , 106 , 32 , 128 , 217 , 193 , 49 , 86 , 165 , 12 , 175 , 151 , 15 , 162 , 122 , 139 , 140 , 34 , 100 , 227 , 151 , 152 , 123 , 217 , 27 , 18 , 245 , 180 , 193 , 52 , 75 , 90 , 247 , 159 , 202 , 102 , 217 , 47 , 56 , 249 , 43] , [39 , 175 , 107 , 14 , 5 , 36 , 130 , 255 , 75 , 7 , 14 , 183 , 215 , 7 , 140 , 222 , 19 , 94 , 56 , 133 , 117 , 231 , 76 , 158 , 24 , 143 , 74 , 46 , 37 , 166 , 106 , 53 , 28 , 171 , 35 , 118 , 106 , 13 , 193 , 82 , 15 , 41 , 14 , 92 , 43 , 114 , 137 , 52 , 230 , 73 , 52 , 220 , 152 , 149 , 56 , 249 , 66 , 48 , 96 , 126 , 118 , 122 , 6 , 38] , [33 , 34 , 189 , 123 , 120 , 215 , 116 , 33 , 49 , 177 , 79 , 129 , 72 , 109 , 12 , 209 , 170 , 154 , 65 , 193 , 135 , 59 , 7 , 181 , 81 , 139 , 28 , 26 , 57 , 29 , 38 , 24 , 37 , 151 , 214 , 176 , 144 , 238 , 67 , 184 , 172 , 75 , 22 , 126 , 215 , 66 , 41 , 214 , 177 , 172 , 20 , 47 , 141 , 229 , 25 , 76 , 124 , 58 , 35 , 239 , 80 , 69 , 142 , 50] , [3 , 93 , 165 , 112 , 187 , 253 , 117 , 246 , 147 , 130 , 88 , 19 , 50 , 236 , 202 , 13 , 192 , 190 , 233 , 89 , 84 , 57 , 36 , 246 , 240 , 20 , 12 , 200 , 191 , 230 , 41 , 93 , 17 , 94 , 68 , 253 , 51 , 115 , 116 , 234 , 8 , 10 , 192 , 237 , 137 , 238 , 220 , 70 , 66 , 174 , 195 , 0 , 174 , 153 , 225 , 127 , 87 , 24 , 151 , 240 , 160 , 41 , 51 , 0] , [28 , 41 , 227 , 48 , 202 , 209 , 77 , 89 , 53 , 35 , 16 , 181 , 84 , 46 , 55 , 136 , 160 , 149 , 234 , 93 , 70 , 26 , 67 , 205 , 79 , 113 , 76 , 255 , 12 , 171 , 150 , 47 , 40 , 220 , 239 , 166 , 11 , 51 , 205 , 213 , 126 , 215 , 77 , 131 , 206 , 193 , 19 , 36 , 163 , 229 , 32 , 215 , 53 , 46 , 182 , 86 , 31 , 47 , 85 , 100 , 109 , 46 , 181 , 241] , [27 , 119 , 74 , 10 , 102 , 108 , 48 , 137 , 141 , 79 , 201 , 85 , 180 , 153 , 101 , 87 , 185 , 146 , 118 , 135 , 91 , 72 , 117 , 86 , 67 , 201 , 199 , 139 , 250 , 68 , 215 , 20 , 12 , 43 , 111 , 206 , 160 , 37 , 113 , 209 , 161 , 188 , 234 , 36 , 129 , 216 , 164 , 79 , 64 , 92 , 60 , 136 , 248 , 212 , 251 , 77 , 115 , 70 , 119 , 225 , 195 , 205 , 9 , 181] , [29 , 32 , 39 , 44 , 114 , 192 , 193 , 126 , 180 , 96 , 142 , 124 , 93 , 165 , 37 , 202 , 245 , 36 , 126 , 159 , 157 , 132 , 110 , 40 , 69 , 180 , 179 , 8 , 7 , 85 , 237 , 246 , 46 , 15 , 174 , 41 , 75 , 66 , 155 , 185 , 144 , 203 , 15 , 191 , 52 , 224 , 176 , 156 , 33 , 44 , 72 , 31 , 34 , 129 , 110 , 242 , 184 , 196 , 109 , 248 , 39 , 249 , 254 , 184] , [44 , 251 , 166 , 115 , 98 , 51 , 141 , 19 , 156 , 235 , 68 , 71 , 191 , 152 , 4 , 185 , 218 , 164 , 236 , 197 , 145 , 66 , 85 , 23 , 233 , 87 , 136 , 221 , 202 , 12 , 145 , 168 , 32 , 246 , 0 , 50 , 31 , 81 , 93 , 154 , 38 , 25 , 17 , 65 , 217 , 20 , 43 , 216 , 226 , 19 , 157 , 96 , 69 , 35 , 242 , 209 , 219 , 254 , 6 , 62 , 192 , 98 , 160 , 226]] , }"
    }
  ],
  "instructions": [
    {
      "name": "compressedTransferFirst",
      "docs": [
        "This instruction is the first step of a compressed transaction.",
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
          "name": "merkleTreeSet",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "authority",
          "isMut": true,
          "isSigner": false
        },
        {
          "name": "rpcRecipientSol",
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
          "name": "tokenProgram",
          "isMut": false,
          "isSigner": false
        },
        {
          "name": "tokenAuthority",
          "isMut": true,
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
          "name": "registeredVerifierPda",
          "isMut": true,
          "isSigner": false,
          "docs": [
            "Verifier config pda which needs to exist."
          ]
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
    }
  ],
  "accounts": [
    {
      "name": "instructionDataCompressedTransferFirst",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "proofA",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "proofB",
            "type": {
              "array": [
                "u8",
                64
              ]
            }
          },
          {
            "name": "proofC",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
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
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
            "name": "publicAmountSol",
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
            "name": "rpcFee",
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
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "dataHash",
            "type": "u256"
          },
          {
            "name": "accountCompressionPublicKey",
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
          }
        ]
      }
    },
    {
      "name": "outUtxo",
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
            "name": "blinding",
            "type": "u256"
          },
          {
            "name": "utxoDataHash",
            "type": "u256"
          },
          {
            "name": "accountCompressionPublicKey",
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
            "name": "isFillingUtxo",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainProofInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "assetPublicKeys",
            "type": {
              "array": [
                "u8",
                3
              ]
            }
          },
          {
            "name": "privatePublicDataHash",
            "type": "u8"
          },
          {
            "name": "address",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "metaHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
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
                2
              ]
            }
          },
          {
            "name": "inPrivateKey",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "leafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "merkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                2
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
                2
              ]
            }
          },
          {
            "name": "nullifierLeafIndex",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "nullifierMerkleProof",
            "type": {
              "array": [
                {
                  "array": [
                    "u8",
                    22
                  ]
                },
                2
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
                2
              ]
            }
          },
          {
            "name": "outOwner",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outBlinding",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "outDataHash",
            "type": {
              "array": [
                "u8",
                2
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
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "zKprivateTransaction2In2OutMainPublicInputs",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicNullifierRoot",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicAmountSpl",
            "type": "u8"
          },
          {
            "name": "publicDataHash",
            "type": "u8"
          },
          {
            "name": "publicAmountSol",
            "type": "u8"
          },
          {
            "name": "publicMintPublicKey",
            "type": "u8"
          },
          {
            "name": "publicNullifier",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicOutUtxoHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          }
        ]
      }
    },
    {
      "name": "instructionDataLightInstructionPrivateTransaction2In2OutMainSecond",
      "type": {
        "kind": "struct",
        "fields": [
          {
            "name": "publicStateRoot",
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
            "name": "publicNullifierRoot",
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
            "name": "publicAmountSpl",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicDataHash",
            "type": {
              "array": [
                "u8",
                32
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
            "name": "publicMintPublicKey",
            "type": {
              "array": [
                "u8",
                32
              ]
            }
          },
          {
            "name": "publicNullifier",
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
            "name": "publicOutUtxoHash",
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
          }
        ]
      }
    }
  ]
};
