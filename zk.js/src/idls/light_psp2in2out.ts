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
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [12 , 67 , 57 , 94 , 98 , 23 , 149 , 34 , 150 , 207 , 93 , 182 , 90 , 106 , 199 , 97 , 0 , 110 , 134 , 29 , 39 , 143 , 8 , 210 , 30 , 10 , 44 , 141 , 26 , 16 , 75 , 93 , 14 , 50 , 87 , 54 , 238 , 94 , 146 , 62 , 155 , 46 , 119 , 5 , 143 , 252 , 220 , 169 , 51 , 44 , 152 , 183 , 24 , 60 , 186 , 140 , 200 , 85 , 141 , 56 , 207 , 153 , 61 , 222 , 44 , 218 , 211 , 147 , 78 , 121 , 235 , 251 , 159 , 193 , 206 , 142 , 22 , 83 , 185 , 142 , 159 , 58 , 245 , 4 , 235 , 69 , 21 , 142 , 169 , 213 , 203 , 96 , 163 , 46 , 86 , 20 , 45 , 27 , 77 , 4 , 3 , 182 , 254 , 126 , 59 , 101 , 76 , 124 , 174 , 146 , 80 , 136 , 72 , 22 , 159 , 31 , 221 , 145 , 225 , 236 , 94 , 99 , 19 , 34 , 194 , 61 , 176 , 116] , vk_ic : & [[17 , 96 , 226 , 134 , 22 , 229 , 105 , 207 , 66 , 98 , 210 , 97 , 155 , 151 , 174 , 143 , 43 , 226 , 2 , 176 , 73 , 181 , 8 , 18 , 3 , 128 , 250 , 86 , 173 , 73 , 210 , 127 , 13 , 228 , 98 , 69 , 60 , 171 , 30 , 119 , 206 , 169 , 171 , 121 , 102 , 206 , 163 , 72 , 151 , 132 , 52 , 120 , 52 , 143 , 32 , 220 , 59 , 101 , 83 , 108 , 234 , 229 , 164 , 128] , [26 , 185 , 61 , 224 , 157 , 15 , 133 , 47 , 134 , 218 , 255 , 115 , 218 , 180 , 79 , 89 , 142 , 169 , 218 , 45 , 238 , 162 , 141 , 12 , 144 , 94 , 13 , 189 , 177 , 211 , 78 , 251 , 5 , 87 , 146 , 211 , 15 , 12 , 220 , 114 , 27 , 245 , 93 , 205 , 186 , 66 , 15 , 118 , 185 , 94 , 96 , 202 , 32 , 42 , 10 , 46 , 6 , 162 , 106 , 61 , 221 , 157 , 185 , 17] , [34 , 73 , 2 , 178 , 215 , 205 , 28 , 60 , 82 , 73 , 48 , 194 , 121 , 96 , 48 , 185 , 187 , 214 , 163 , 213 , 177 , 39 , 95 , 167 , 119 , 153 , 93 , 28 , 241 , 238 , 93 , 239 , 31 , 125 , 203 , 4 , 243 , 239 , 137 , 80 , 97 , 168 , 235 , 29 , 196 , 51 , 197 , 77 , 59 , 175 , 71 , 117 , 241 , 195 , 40 , 200 , 199 , 76 , 148 , 55 , 214 , 207 , 207 , 237] , [26 , 251 , 25 , 7 , 63 , 71 , 244 , 218 , 1 , 92 , 124 , 125 , 74 , 115 , 39 , 197 , 155 , 39 , 189 , 159 , 135 , 145 , 39 , 36 , 186 , 191 , 48 , 90 , 169 , 73 , 132 , 147 , 41 , 13 , 115 , 57 , 46 , 227 , 33 , 205 , 76 , 106 , 18 , 136 , 210 , 72 , 159 , 58 , 67 , 162 , 219 , 69 , 157 , 147 , 160 , 28 , 103 , 185 , 137 , 133 , 226 , 242 , 192 , 199] , [20 , 193 , 151 , 73 , 255 , 193 , 198 , 86 , 237 , 152 , 190 , 140 , 0 , 205 , 143 , 204 , 223 , 121 , 188 , 196 , 49 , 52 , 237 , 240 , 158 , 37 , 29 , 206 , 57 , 109 , 138 , 67 , 48 , 12 , 242 , 174 , 179 , 109 , 237 , 204 , 36 , 20 , 134 , 115 , 79 , 66 , 125 , 109 , 6 , 114 , 111 , 176 , 119 , 73 , 193 , 89 , 67 , 200 , 120 , 57 , 143 , 230 , 20 , 74] , [6 , 231 , 39 , 82 , 135 , 39 , 238 , 97 , 49 , 32 , 169 , 4 , 185 , 38 , 167 , 188 , 82 , 115 , 212 , 236 , 243 , 60 , 233 , 9 , 207 , 63 , 190 , 190 , 199 , 177 , 47 , 74 , 28 , 68 , 135 , 8 , 57 , 47 , 54 , 0 , 15 , 224 , 64 , 25 , 236 , 8 , 242 , 159 , 208 , 231 , 39 , 155 , 155 , 74 , 203 , 18 , 20 , 45 , 103 , 144 , 52 , 130 , 42 , 167] , [6 , 72 , 137 , 154 , 166 , 237 , 134 , 122 , 241 , 52 , 182 , 170 , 158 , 29 , 121 , 190 , 137 , 52 , 151 , 222 , 16 , 133 , 218 , 45 , 243 , 48 , 117 , 181 , 236 , 123 , 138 , 138 , 18 , 30 , 23 , 75 , 147 , 25 , 137 , 6 , 27 , 148 , 9 , 155 , 46 , 66 , 242 , 147 , 48 , 106 , 160 , 120 , 236 , 49 , 244 , 107 , 99 , 113 , 126 , 90 , 19 , 158 , 2 , 154] , [35 , 89 , 198 , 251 , 60 , 97 , 178 , 36 , 182 , 205 , 53 , 39 , 59 , 32 , 194 , 130 , 93 , 199 , 213 , 27 , 88 , 22 , 90 , 157 , 92 , 196 , 147 , 74 , 3 , 195 , 98 , 220 , 18 , 67 , 118 , 68 , 88 , 142 , 7 , 109 , 236 , 140 , 249 , 3 , 26 , 173 , 234 , 120 , 222 , 57 , 158 , 236 , 98 , 116 , 39 , 65 , 60 , 252 , 67 , 19 , 110 , 236 , 79 , 25] , [46 , 224 , 158 , 202 , 226 , 139 , 186 , 61 , 237 , 3 , 176 , 32 , 189 , 235 , 127 , 70 , 88 , 221 , 125 , 196 , 151 , 87 , 163 , 156 , 175 , 118 , 147 , 149 , 136 , 4 , 27 , 216 , 37 , 71 , 40 , 33 , 175 , 68 , 23 , 182 , 196 , 35 , 1 , 158 , 131 , 46 , 74 , 233 , 192 , 156 , 28 , 209 , 139 , 18 , 184 , 115 , 59 , 34 , 164 , 225 , 69 , 200 , 228 , 239] , [24 , 231 , 91 , 122 , 58 , 69 , 3 , 108 , 80 , 109 , 223 , 239 , 30 , 83 , 161 , 135 , 172 , 217 , 170 , 85 , 44 , 63 , 157 , 208 , 235 , 197 , 168 , 21 , 104 , 232 , 166 , 13 , 22 , 150 , 49 , 116 , 152 , 86 , 220 , 150 , 230 , 247 , 246 , 28 , 167 , 193 , 177 , 110 , 186 , 84 , 40 , 231 , 24 , 244 , 96 , 103 , 232 , 18 , 111 , 118 , 76 , 72 , 29 , 127] , [45 , 73 , 102 , 206 , 165 , 249 , 223 , 42 , 14 , 135 , 92 , 217 , 114 , 81 , 95 , 46 , 193 , 182 , 212 , 82 , 17 , 193 , 107 , 19 , 69 , 190 , 167 , 43 , 180 , 117 , 217 , 242 , 42 , 53 , 167 , 15 , 176 , 163 , 213 , 3 , 145 , 191 , 153 , 230 , 217 , 134 , 204 , 65 , 230 , 239 , 224 , 24 , 142 , 171 , 7 , 178 , 180 , 245 , 89 , 170 , 42 , 225 , 100 , 172] , [14 , 148 , 80 , 164 , 24 , 82 , 179 , 27 , 0 , 14 , 22 , 182 , 165 , 8 , 133 , 115 , 106 , 33 , 14 , 213 , 144 , 139 , 192 , 148 , 56 , 203 , 150 , 13 , 209 , 7 , 90 , 28 , 3 , 162 , 165 , 252 , 38 , 201 , 208 , 108 , 171 , 139 , 98 , 199 , 79 , 184 , 172 , 30 , 178 , 202 , 90 , 220 , 195 , 194 , 57 , 46 , 118 , 32 , 135 , 126 , 154 , 195 , 197 , 107] , [14 , 183 , 2 , 28 , 127 , 67 , 43 , 16 , 152 , 17 , 93 , 104 , 85 , 47 , 40 , 56 , 129 , 26 , 90 , 90 , 157 , 17 , 43 , 55 , 97 , 149 , 150 , 98 , 218 , 43 , 82 , 179 , 48 , 70 , 205 , 41 , 175 , 25 , 157 , 152 , 64 , 166 , 59 , 92 , 69 , 139 , 239 , 218 , 31 , 153 , 230 , 78 , 200 , 15 , 53 , 241 , 200 , 39 , 1 , 98 , 154 , 127 , 76 , 238]] , }"
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
        },
        {
          "name": "merkleTreeSet",
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
      "value": "Groth16Verifyingkey { nr_pubinputs : 12 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [12 , 67 , 57 , 94 , 98 , 23 , 149 , 34 , 150 , 207 , 93 , 182 , 90 , 106 , 199 , 97 , 0 , 110 , 134 , 29 , 39 , 143 , 8 , 210 , 30 , 10 , 44 , 141 , 26 , 16 , 75 , 93 , 14 , 50 , 87 , 54 , 238 , 94 , 146 , 62 , 155 , 46 , 119 , 5 , 143 , 252 , 220 , 169 , 51 , 44 , 152 , 183 , 24 , 60 , 186 , 140 , 200 , 85 , 141 , 56 , 207 , 153 , 61 , 222 , 44 , 218 , 211 , 147 , 78 , 121 , 235 , 251 , 159 , 193 , 206 , 142 , 22 , 83 , 185 , 142 , 159 , 58 , 245 , 4 , 235 , 69 , 21 , 142 , 169 , 213 , 203 , 96 , 163 , 46 , 86 , 20 , 45 , 27 , 77 , 4 , 3 , 182 , 254 , 126 , 59 , 101 , 76 , 124 , 174 , 146 , 80 , 136 , 72 , 22 , 159 , 31 , 221 , 145 , 225 , 236 , 94 , 99 , 19 , 34 , 194 , 61 , 176 , 116] , vk_ic : & [[17 , 96 , 226 , 134 , 22 , 229 , 105 , 207 , 66 , 98 , 210 , 97 , 155 , 151 , 174 , 143 , 43 , 226 , 2 , 176 , 73 , 181 , 8 , 18 , 3 , 128 , 250 , 86 , 173 , 73 , 210 , 127 , 13 , 228 , 98 , 69 , 60 , 171 , 30 , 119 , 206 , 169 , 171 , 121 , 102 , 206 , 163 , 72 , 151 , 132 , 52 , 120 , 52 , 143 , 32 , 220 , 59 , 101 , 83 , 108 , 234 , 229 , 164 , 128] , [26 , 185 , 61 , 224 , 157 , 15 , 133 , 47 , 134 , 218 , 255 , 115 , 218 , 180 , 79 , 89 , 142 , 169 , 218 , 45 , 238 , 162 , 141 , 12 , 144 , 94 , 13 , 189 , 177 , 211 , 78 , 251 , 5 , 87 , 146 , 211 , 15 , 12 , 220 , 114 , 27 , 245 , 93 , 205 , 186 , 66 , 15 , 118 , 185 , 94 , 96 , 202 , 32 , 42 , 10 , 46 , 6 , 162 , 106 , 61 , 221 , 157 , 185 , 17] , [34 , 73 , 2 , 178 , 215 , 205 , 28 , 60 , 82 , 73 , 48 , 194 , 121 , 96 , 48 , 185 , 187 , 214 , 163 , 213 , 177 , 39 , 95 , 167 , 119 , 153 , 93 , 28 , 241 , 238 , 93 , 239 , 31 , 125 , 203 , 4 , 243 , 239 , 137 , 80 , 97 , 168 , 235 , 29 , 196 , 51 , 197 , 77 , 59 , 175 , 71 , 117 , 241 , 195 , 40 , 200 , 199 , 76 , 148 , 55 , 214 , 207 , 207 , 237] , [26 , 251 , 25 , 7 , 63 , 71 , 244 , 218 , 1 , 92 , 124 , 125 , 74 , 115 , 39 , 197 , 155 , 39 , 189 , 159 , 135 , 145 , 39 , 36 , 186 , 191 , 48 , 90 , 169 , 73 , 132 , 147 , 41 , 13 , 115 , 57 , 46 , 227 , 33 , 205 , 76 , 106 , 18 , 136 , 210 , 72 , 159 , 58 , 67 , 162 , 219 , 69 , 157 , 147 , 160 , 28 , 103 , 185 , 137 , 133 , 226 , 242 , 192 , 199] , [20 , 193 , 151 , 73 , 255 , 193 , 198 , 86 , 237 , 152 , 190 , 140 , 0 , 205 , 143 , 204 , 223 , 121 , 188 , 196 , 49 , 52 , 237 , 240 , 158 , 37 , 29 , 206 , 57 , 109 , 138 , 67 , 48 , 12 , 242 , 174 , 179 , 109 , 237 , 204 , 36 , 20 , 134 , 115 , 79 , 66 , 125 , 109 , 6 , 114 , 111 , 176 , 119 , 73 , 193 , 89 , 67 , 200 , 120 , 57 , 143 , 230 , 20 , 74] , [6 , 231 , 39 , 82 , 135 , 39 , 238 , 97 , 49 , 32 , 169 , 4 , 185 , 38 , 167 , 188 , 82 , 115 , 212 , 236 , 243 , 60 , 233 , 9 , 207 , 63 , 190 , 190 , 199 , 177 , 47 , 74 , 28 , 68 , 135 , 8 , 57 , 47 , 54 , 0 , 15 , 224 , 64 , 25 , 236 , 8 , 242 , 159 , 208 , 231 , 39 , 155 , 155 , 74 , 203 , 18 , 20 , 45 , 103 , 144 , 52 , 130 , 42 , 167] , [6 , 72 , 137 , 154 , 166 , 237 , 134 , 122 , 241 , 52 , 182 , 170 , 158 , 29 , 121 , 190 , 137 , 52 , 151 , 222 , 16 , 133 , 218 , 45 , 243 , 48 , 117 , 181 , 236 , 123 , 138 , 138 , 18 , 30 , 23 , 75 , 147 , 25 , 137 , 6 , 27 , 148 , 9 , 155 , 46 , 66 , 242 , 147 , 48 , 106 , 160 , 120 , 236 , 49 , 244 , 107 , 99 , 113 , 126 , 90 , 19 , 158 , 2 , 154] , [35 , 89 , 198 , 251 , 60 , 97 , 178 , 36 , 182 , 205 , 53 , 39 , 59 , 32 , 194 , 130 , 93 , 199 , 213 , 27 , 88 , 22 , 90 , 157 , 92 , 196 , 147 , 74 , 3 , 195 , 98 , 220 , 18 , 67 , 118 , 68 , 88 , 142 , 7 , 109 , 236 , 140 , 249 , 3 , 26 , 173 , 234 , 120 , 222 , 57 , 158 , 236 , 98 , 116 , 39 , 65 , 60 , 252 , 67 , 19 , 110 , 236 , 79 , 25] , [46 , 224 , 158 , 202 , 226 , 139 , 186 , 61 , 237 , 3 , 176 , 32 , 189 , 235 , 127 , 70 , 88 , 221 , 125 , 196 , 151 , 87 , 163 , 156 , 175 , 118 , 147 , 149 , 136 , 4 , 27 , 216 , 37 , 71 , 40 , 33 , 175 , 68 , 23 , 182 , 196 , 35 , 1 , 158 , 131 , 46 , 74 , 233 , 192 , 156 , 28 , 209 , 139 , 18 , 184 , 115 , 59 , 34 , 164 , 225 , 69 , 200 , 228 , 239] , [24 , 231 , 91 , 122 , 58 , 69 , 3 , 108 , 80 , 109 , 223 , 239 , 30 , 83 , 161 , 135 , 172 , 217 , 170 , 85 , 44 , 63 , 157 , 208 , 235 , 197 , 168 , 21 , 104 , 232 , 166 , 13 , 22 , 150 , 49 , 116 , 152 , 86 , 220 , 150 , 230 , 247 , 246 , 28 , 167 , 193 , 177 , 110 , 186 , 84 , 40 , 231 , 24 , 244 , 96 , 103 , 232 , 18 , 111 , 118 , 76 , 72 , 29 , 127] , [45 , 73 , 102 , 206 , 165 , 249 , 223 , 42 , 14 , 135 , 92 , 217 , 114 , 81 , 95 , 46 , 193 , 182 , 212 , 82 , 17 , 193 , 107 , 19 , 69 , 190 , 167 , 43 , 180 , 117 , 217 , 242 , 42 , 53 , 167 , 15 , 176 , 163 , 213 , 3 , 145 , 191 , 153 , 230 , 217 , 134 , 204 , 65 , 230 , 239 , 224 , 24 , 142 , 171 , 7 , 178 , 180 , 245 , 89 , 170 , 42 , 225 , 100 , 172] , [14 , 148 , 80 , 164 , 24 , 82 , 179 , 27 , 0 , 14 , 22 , 182 , 165 , 8 , 133 , 115 , 106 , 33 , 14 , 213 , 144 , 139 , 192 , 148 , 56 , 203 , 150 , 13 , 209 , 7 , 90 , 28 , 3 , 162 , 165 , 252 , 38 , 201 , 208 , 108 , 171 , 139 , 98 , 199 , 79 , 184 , 172 , 30 , 178 , 202 , 90 , 220 , 195 , 194 , 57 , 46 , 118 , 32 , 135 , 126 , 154 , 195 , 197 , 107] , [14 , 183 , 2 , 28 , 127 , 67 , 43 , 16 , 152 , 17 , 93 , 104 , 85 , 47 , 40 , 56 , 129 , 26 , 90 , 90 , 157 , 17 , 43 , 55 , 97 , 149 , 150 , 98 , 218 , 43 , 82 , 179 , 48 , 70 , 205 , 41 , 175 , 25 , 157 , 152 , 64 , 166 , 59 , 92 , 69 , 139 , 239 , 218 , 31 , 153 , 230 , 78 , 200 , 15 , 53 , 241 , 200 , 39 , 1 , 98 , 154 , 127 , 76 , 238]] , }"
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
        },
        {
          "name": "merkleTreeSet",
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
