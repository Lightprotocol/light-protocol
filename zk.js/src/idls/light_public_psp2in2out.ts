export type LightPublicPsp2in2out = {
  "version": "0.3.1",
  "name": "light_public_psp2in2out",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE\""
    },
    {
      "name": "VERIFYINGKEY_PUBLIC_PROGRAM_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 14 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [46 , 55 , 248 , 175 , 67 , 72 , 52 , 48 , 182 , 166 , 22 , 161 , 203 , 168 , 241 , 78 , 116 , 44 , 81 , 182 , 189 , 59 , 194 , 240 , 227 , 184 , 200 , 123 , 186 , 241 , 71 , 78 , 33 , 133 , 84 , 203 , 227 , 215 , 175 , 189 , 55 , 148 , 14 , 113 , 150 , 34 , 242 , 221 , 204 , 122 , 97 , 250 , 86 , 212 , 57 , 168 , 169 , 164 , 251 , 96 , 158 , 191 , 31 , 117 , 3 , 178 , 134 , 205 , 161 , 186 , 254 , 25 , 105 , 48 , 165 , 101 , 247 , 248 , 5 , 54 , 126 , 26 , 206 , 111 , 84 , 155 , 50 , 248 , 173 , 102 , 87 , 88 , 85 , 102 , 197 , 136 , 3 , 35 , 20 , 178 , 185 , 254 , 194 , 14 , 189 , 108 , 60 , 167 , 118 , 63 , 94 , 63 , 64 , 120 , 238 , 81 , 210 , 233 , 4 , 54 , 95 , 90 , 248 , 66 , 165 , 43 , 49 , 106] , vk_ic : & [[27 , 239 , 108 , 182 , 28 , 236 , 192 , 28 , 96 , 226 , 64 , 119 , 244 , 169 , 46 , 53 , 61 , 187 , 254 , 53 , 255 , 62 , 164 , 223 , 129 , 153 , 109 , 205 , 238 , 85 , 103 , 246 , 28 , 192 , 111 , 115 , 231 , 154 , 201 , 213 , 255 , 99 , 24 , 144 , 70 , 193 , 133 , 196 , 188 , 139 , 222 , 160 , 156 , 35 , 35 , 151 , 11 , 105 , 82 , 157 , 202 , 206 , 30 , 177] , [41 , 71 , 174 , 11 , 205 , 250 , 54 , 217 , 5 , 121 , 29 , 201 , 21 , 228 , 63 , 1 , 212 , 231 , 114 , 154 , 145 , 0 , 157 , 153 , 78 , 34 , 89 , 94 , 54 , 127 , 107 , 254 , 19 , 171 , 3 , 79 , 128 , 254 , 65 , 107 , 205 , 110 , 69 , 34 , 136 , 11 , 90 , 185 , 209 , 56 , 117 , 152 , 235 , 124 , 238 , 28 , 153 , 253 , 11 , 32 , 153 , 237 , 14 , 139] , [0 , 100 , 4 , 152 , 183 , 7 , 176 , 93 , 70 , 23 , 113 , 183 , 227 , 43 , 25 , 64 , 91 , 186 , 7 , 92 , 119 , 36 , 228 , 160 , 196 , 192 , 225 , 48 , 59 , 18 , 124 , 11 , 33 , 109 , 121 , 189 , 63 , 180 , 215 , 78 , 146 , 209 , 210 , 212 , 132 , 123 , 219 , 204 , 251 , 30 , 248 , 82 , 16 , 59 , 94 , 180 , 2 , 25 , 193 , 122 , 16 , 185 , 192 , 234] , [9 , 72 , 165 , 64 , 15 , 135 , 180 , 240 , 238 , 69 , 221 , 96 , 114 , 240 , 56 , 98 , 123 , 117 , 216 , 94 , 33 , 175 , 56 , 31 , 109 , 122 , 188 , 167 , 79 , 31 , 32 , 121 , 19 , 180 , 240 , 169 , 30 , 90 , 122 , 201 , 143 , 241 , 58 , 171 , 60 , 196 , 67 , 113 , 98 , 102 , 64 , 194 , 223 , 94 , 143 , 158 , 50 , 143 , 48 , 80 , 135 , 174 , 241 , 1] , [15 , 121 , 117 , 48 , 52 , 221 , 77 , 225 , 98 , 67 , 200 , 90 , 214 , 160 , 75 , 107 , 69 , 223 , 242 , 125 , 26 , 28 , 240 , 2 , 161 , 48 , 1 , 161 , 181 , 19 , 241 , 212 , 20 , 246 , 176 , 46 , 196 , 152 , 76 , 60 , 114 , 245 , 247 , 99 , 243 , 106 , 235 , 198 , 158 , 108 , 47 , 215 , 156 , 173 , 236 , 112 , 15 , 72 , 67 , 40 , 229 , 23 , 54 , 98] , [15 , 121 , 127 , 168 , 198 , 172 , 150 , 191 , 29 , 249 , 226 , 17 , 182 , 56 , 212 , 154 , 80 , 196 , 25 , 115 , 244 , 192 , 241 , 46 , 202 , 147 , 75 , 115 , 205 , 168 , 7 , 232 , 18 , 202 , 87 , 54 , 71 , 113 , 78 , 193 , 176 , 193 , 57 , 58 , 217 , 88 , 239 , 172 , 198 , 0 , 119 , 4 , 10 , 128 , 224 , 239 , 203 , 94 , 53 , 153 , 171 , 74 , 103 , 132] , [38 , 39 , 20 , 227 , 196 , 23 , 168 , 223 , 98 , 39 , 63 , 53 , 53 , 182 , 99 , 238 , 48 , 8 , 147 , 74 , 234 , 205 , 113 , 50 , 201 , 211 , 124 , 224 , 154 , 30 , 67 , 84 , 15 , 8 , 232 , 93 , 104 , 57 , 238 , 30 , 242 , 121 , 97 , 194 , 39 , 116 , 209 , 100 , 241 , 78 , 179 , 170 , 107 , 9 , 249 , 81 , 131 , 168 , 76 , 186 , 5 , 211 , 98 , 236] , [37 , 149 , 180 , 32 , 227 , 98 , 111 , 126 , 194 , 177 , 177 , 211 , 154 , 1 , 210 , 205 , 11 , 0 , 31 , 2 , 107 , 200 , 41 , 108 , 132 , 169 , 144 , 151 , 201 , 44 , 225 , 224 , 8 , 112 , 156 , 56 , 21 , 198 , 127 , 15 , 9 , 184 , 172 , 88 , 139 , 236 , 240 , 34 , 51 , 208 , 38 , 250 , 135 , 91 , 28 , 225 , 100 , 182 , 100 , 74 , 241 , 172 , 37 , 50] , [8 , 240 , 198 , 234 , 209 , 76 , 140 , 191 , 175 , 185 , 14 , 50 , 171 , 54 , 171 , 185 , 184 , 239 , 147 , 75 , 233 , 16 , 137 , 66 , 222 , 30 , 12 , 84 , 166 , 165 , 1 , 187 , 11 , 165 , 231 , 175 , 164 , 49 , 111 , 57 , 180 , 13 , 116 , 189 , 53 , 18 , 153 , 226 , 132 , 181 , 91 , 47 , 254 , 200 , 153 , 221 , 146 , 82 , 235 , 232 , 227 , 236 , 213 , 179] , [4 , 206 , 122 , 187 , 250 , 30 , 38 , 132 , 26 , 139 , 164 , 123 , 203 , 69 , 253 , 27 , 85 , 237 , 252 , 100 , 148 , 67 , 250 , 214 , 11 , 87 , 23 , 12 , 198 , 221 , 95 , 44 , 26 , 184 , 61 , 18 , 253 , 40 , 69 , 122 , 180 , 88 , 10 , 83 , 127 , 119 , 60 , 45 , 224 , 232 , 7 , 40 , 100 , 115 , 109 , 59 , 137 , 68 , 171 , 248 , 139 , 81 , 80 , 81] , [4 , 188 , 123 , 189 , 121 , 84 , 134 , 112 , 88 , 93 , 177 , 169 , 124 , 16 , 26 , 252 , 195 , 130 , 87 , 32 , 96 , 245 , 112 , 204 , 39 , 84 , 82 , 154 , 194 , 64 , 131 , 17 , 37 , 229 , 30 , 249 , 181 , 200 , 133 , 221 , 75 , 106 , 158 , 142 , 14 , 35 , 52 , 130 , 147 , 190 , 59 , 33 , 211 , 83 , 70 , 121 , 245 , 193 , 69 , 175 , 6 , 239 , 148 , 123] , [16 , 103 , 74 , 52 , 139 , 67 , 251 , 12 , 40 , 255 , 29 , 98 , 20 , 48 , 255 , 94 , 165 , 104 , 112 , 190 , 153 , 175 , 217 , 49 , 243 , 224 , 3 , 57 , 183 , 141 , 65 , 183 , 28 , 96 , 187 , 227 , 190 , 149 , 159 , 14 , 41 , 118 , 28 , 51 , 177 , 185 , 26 , 170 , 51 , 241 , 180 , 117 , 33 , 36 , 140 , 213 , 188 , 94 , 125 , 236 , 155 , 170 , 86 , 96] , [28 , 244 , 109 , 89 , 81 , 189 , 1 , 4 , 221 , 193 , 35 , 65 , 91 , 198 , 215 , 172 , 152 , 194 , 107 , 239 , 235 , 188 , 105 , 158 , 203 , 83 , 173 , 135 , 126 , 4 , 239 , 159 , 45 , 35 , 107 , 54 , 143 , 70 , 32 , 179 , 62 , 45 , 147 , 193 , 58 , 28 , 43 , 105 , 35 , 231 , 22 , 79 , 17 , 235 , 80 , 228 , 35 , 119 , 238 , 122 , 208 , 17 , 74 , 0] , [23 , 12 , 239 , 116 , 124 , 105 , 206 , 144 , 227 , 188 , 179 , 217 , 16 , 63 , 192 , 54 , 224 , 177 , 85 , 8 , 119 , 40 , 219 , 120 , 96 , 137 , 219 , 205 , 3 , 194 , 219 , 108 , 10 , 149 , 35 , 77 , 32 , 107 , 96 , 117 , 225 , 165 , 194 , 85 , 213 , 11 , 177 , 177 , 208 , 120 , 121 , 198 , 0 , 83 , 240 , 140 , 39 , 196 , 244 , 232 , 155 , 82 , 38 , 96] , [22 , 86 , 205 , 164 , 60 , 154 , 53 , 249 , 6 , 238 , 24 , 119 , 92 , 160 , 255 , 251 , 218 , 61 , 97 , 71 , 88 , 233 , 143 , 58 , 150 , 141 , 135 , 207 , 240 , 193 , 12 , 101 , 36 , 59 , 19 , 144 , 86 , 102 , 196 , 175 , 107 , 228 , 247 , 240 , 11 , 219 , 49 , 164 , 75 , 144 , 211 , 214 , 55 , 91 , 240 , 39 , 25 , 242 , 219 , 222 , 233 , 32 , 41 , 9]] , }"
    }
  ],
  "instructions": [
    {
      "name": "shieldedTransferFirst",
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
          "name": "eventMerkleTree",
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
      "name": "instructionDataShieldedTransferFirst",
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
            "name": "verifierAddressIndex",
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
            "name": "isFillingUtxo",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "zKpublicProgramTransaction2In2OutMainProofInputs",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicInUtxoDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
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
            "name": "isInProgramUtxo",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inOwner",
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
                    18
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
                    18
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
            "name": "isMetaHashUtxo",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isInAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isOutProgramUtxo",
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
      "name": "zKpublicProgramTransaction2In2OutMainPublicInputs",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicInUtxoDataHash",
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
      "name": "instructionDataLightInstructionPublicProgramTransaction2In2OutMainSecond",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
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
            "name": "publicInUtxoDataHash",
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

export const IDL: LightPublicPsp2in2out = {
  "version": "0.3.1",
  "name": "light_public_psp2in2out",
  "constants": [
    {
      "name": "PROGRAM_ID",
      "type": "string",
      "value": "\"9sixVEthz2kMSKfeApZXHwuboT6DZuT6crAYJTciUCqE\""
    },
    {
      "name": "VERIFYINGKEY_PUBLIC_PROGRAM_TRANSACTION2_IN2_OUT_MAIN",
      "type": {
        "defined": "Groth16Verifyingkey"
      },
      "value": "Groth16Verifyingkey { nr_pubinputs : 14 , vk_alpha_g1 : [45 , 77 , 154 , 167 , 227 , 2 , 217 , 223 , 65 , 116 , 157 , 85 , 7 , 148 , 157 , 5 , 219 , 234 , 51 , 251 , 177 , 108 , 100 , 59 , 34 , 245 , 153 , 162 , 190 , 109 , 242 , 226 , 20 , 190 , 221 , 80 , 60 , 55 , 206 , 176 , 97 , 216 , 236 , 96 , 32 , 159 , 227 , 69 , 206 , 137 , 131 , 10 , 25 , 35 , 3 , 1 , 240 , 118 , 202 , 255 , 0 , 77 , 25 , 38] , vk_beta_g2 : [9 , 103 , 3 , 47 , 203 , 247 , 118 , 209 , 175 , 201 , 133 , 248 , 136 , 119 , 241 , 130 , 211 , 132 , 128 , 166 , 83 , 242 , 222 , 202 , 169 , 121 , 76 , 188 , 59 , 243 , 6 , 12 , 14 , 24 , 120 , 71 , 173 , 76 , 121 , 131 , 116 , 208 , 214 , 115 , 43 , 245 , 1 , 132 , 125 , 214 , 139 , 192 , 224 , 113 , 36 , 30 , 2 , 19 , 188 , 127 , 193 , 61 , 183 , 171 , 48 , 76 , 251 , 209 , 224 , 138 , 112 , 74 , 153 , 245 , 232 , 71 , 217 , 63 , 140 , 60 , 170 , 253 , 222 , 196 , 107 , 122 , 13 , 55 , 157 , 166 , 154 , 77 , 17 , 35 , 70 , 167 , 23 , 57 , 193 , 177 , 164 , 87 , 168 , 199 , 49 , 49 , 35 , 210 , 77 , 47 , 145 , 146 , 248 , 150 , 183 , 198 , 62 , 234 , 5 , 169 , 213 , 127 , 6 , 84 , 122 , 208 , 206 , 200] , vk_gamme_g2 : [25 , 142 , 147 , 147 , 146 , 13 , 72 , 58 , 114 , 96 , 191 , 183 , 49 , 251 , 93 , 37 , 241 , 170 , 73 , 51 , 53 , 169 , 231 , 18 , 151 , 228 , 133 , 183 , 174 , 243 , 18 , 194 , 24 , 0 , 222 , 239 , 18 , 31 , 30 , 118 , 66 , 106 , 0 , 102 , 94 , 92 , 68 , 121 , 103 , 67 , 34 , 212 , 247 , 94 , 218 , 221 , 70 , 222 , 189 , 92 , 217 , 146 , 246 , 237 , 9 , 6 , 137 , 208 , 88 , 95 , 240 , 117 , 236 , 158 , 153 , 173 , 105 , 12 , 51 , 149 , 188 , 75 , 49 , 51 , 112 , 179 , 142 , 243 , 85 , 172 , 218 , 220 , 209 , 34 , 151 , 91 , 18 , 200 , 94 , 165 , 219 , 140 , 109 , 235 , 74 , 171 , 113 , 128 , 141 , 203 , 64 , 143 , 227 , 209 , 231 , 105 , 12 , 67 , 211 , 123 , 76 , 230 , 204 , 1 , 102 , 250 , 125 , 170] , vk_delta_g2 : [46 , 55 , 248 , 175 , 67 , 72 , 52 , 48 , 182 , 166 , 22 , 161 , 203 , 168 , 241 , 78 , 116 , 44 , 81 , 182 , 189 , 59 , 194 , 240 , 227 , 184 , 200 , 123 , 186 , 241 , 71 , 78 , 33 , 133 , 84 , 203 , 227 , 215 , 175 , 189 , 55 , 148 , 14 , 113 , 150 , 34 , 242 , 221 , 204 , 122 , 97 , 250 , 86 , 212 , 57 , 168 , 169 , 164 , 251 , 96 , 158 , 191 , 31 , 117 , 3 , 178 , 134 , 205 , 161 , 186 , 254 , 25 , 105 , 48 , 165 , 101 , 247 , 248 , 5 , 54 , 126 , 26 , 206 , 111 , 84 , 155 , 50 , 248 , 173 , 102 , 87 , 88 , 85 , 102 , 197 , 136 , 3 , 35 , 20 , 178 , 185 , 254 , 194 , 14 , 189 , 108 , 60 , 167 , 118 , 63 , 94 , 63 , 64 , 120 , 238 , 81 , 210 , 233 , 4 , 54 , 95 , 90 , 248 , 66 , 165 , 43 , 49 , 106] , vk_ic : & [[27 , 239 , 108 , 182 , 28 , 236 , 192 , 28 , 96 , 226 , 64 , 119 , 244 , 169 , 46 , 53 , 61 , 187 , 254 , 53 , 255 , 62 , 164 , 223 , 129 , 153 , 109 , 205 , 238 , 85 , 103 , 246 , 28 , 192 , 111 , 115 , 231 , 154 , 201 , 213 , 255 , 99 , 24 , 144 , 70 , 193 , 133 , 196 , 188 , 139 , 222 , 160 , 156 , 35 , 35 , 151 , 11 , 105 , 82 , 157 , 202 , 206 , 30 , 177] , [41 , 71 , 174 , 11 , 205 , 250 , 54 , 217 , 5 , 121 , 29 , 201 , 21 , 228 , 63 , 1 , 212 , 231 , 114 , 154 , 145 , 0 , 157 , 153 , 78 , 34 , 89 , 94 , 54 , 127 , 107 , 254 , 19 , 171 , 3 , 79 , 128 , 254 , 65 , 107 , 205 , 110 , 69 , 34 , 136 , 11 , 90 , 185 , 209 , 56 , 117 , 152 , 235 , 124 , 238 , 28 , 153 , 253 , 11 , 32 , 153 , 237 , 14 , 139] , [0 , 100 , 4 , 152 , 183 , 7 , 176 , 93 , 70 , 23 , 113 , 183 , 227 , 43 , 25 , 64 , 91 , 186 , 7 , 92 , 119 , 36 , 228 , 160 , 196 , 192 , 225 , 48 , 59 , 18 , 124 , 11 , 33 , 109 , 121 , 189 , 63 , 180 , 215 , 78 , 146 , 209 , 210 , 212 , 132 , 123 , 219 , 204 , 251 , 30 , 248 , 82 , 16 , 59 , 94 , 180 , 2 , 25 , 193 , 122 , 16 , 185 , 192 , 234] , [9 , 72 , 165 , 64 , 15 , 135 , 180 , 240 , 238 , 69 , 221 , 96 , 114 , 240 , 56 , 98 , 123 , 117 , 216 , 94 , 33 , 175 , 56 , 31 , 109 , 122 , 188 , 167 , 79 , 31 , 32 , 121 , 19 , 180 , 240 , 169 , 30 , 90 , 122 , 201 , 143 , 241 , 58 , 171 , 60 , 196 , 67 , 113 , 98 , 102 , 64 , 194 , 223 , 94 , 143 , 158 , 50 , 143 , 48 , 80 , 135 , 174 , 241 , 1] , [15 , 121 , 117 , 48 , 52 , 221 , 77 , 225 , 98 , 67 , 200 , 90 , 214 , 160 , 75 , 107 , 69 , 223 , 242 , 125 , 26 , 28 , 240 , 2 , 161 , 48 , 1 , 161 , 181 , 19 , 241 , 212 , 20 , 246 , 176 , 46 , 196 , 152 , 76 , 60 , 114 , 245 , 247 , 99 , 243 , 106 , 235 , 198 , 158 , 108 , 47 , 215 , 156 , 173 , 236 , 112 , 15 , 72 , 67 , 40 , 229 , 23 , 54 , 98] , [15 , 121 , 127 , 168 , 198 , 172 , 150 , 191 , 29 , 249 , 226 , 17 , 182 , 56 , 212 , 154 , 80 , 196 , 25 , 115 , 244 , 192 , 241 , 46 , 202 , 147 , 75 , 115 , 205 , 168 , 7 , 232 , 18 , 202 , 87 , 54 , 71 , 113 , 78 , 193 , 176 , 193 , 57 , 58 , 217 , 88 , 239 , 172 , 198 , 0 , 119 , 4 , 10 , 128 , 224 , 239 , 203 , 94 , 53 , 153 , 171 , 74 , 103 , 132] , [38 , 39 , 20 , 227 , 196 , 23 , 168 , 223 , 98 , 39 , 63 , 53 , 53 , 182 , 99 , 238 , 48 , 8 , 147 , 74 , 234 , 205 , 113 , 50 , 201 , 211 , 124 , 224 , 154 , 30 , 67 , 84 , 15 , 8 , 232 , 93 , 104 , 57 , 238 , 30 , 242 , 121 , 97 , 194 , 39 , 116 , 209 , 100 , 241 , 78 , 179 , 170 , 107 , 9 , 249 , 81 , 131 , 168 , 76 , 186 , 5 , 211 , 98 , 236] , [37 , 149 , 180 , 32 , 227 , 98 , 111 , 126 , 194 , 177 , 177 , 211 , 154 , 1 , 210 , 205 , 11 , 0 , 31 , 2 , 107 , 200 , 41 , 108 , 132 , 169 , 144 , 151 , 201 , 44 , 225 , 224 , 8 , 112 , 156 , 56 , 21 , 198 , 127 , 15 , 9 , 184 , 172 , 88 , 139 , 236 , 240 , 34 , 51 , 208 , 38 , 250 , 135 , 91 , 28 , 225 , 100 , 182 , 100 , 74 , 241 , 172 , 37 , 50] , [8 , 240 , 198 , 234 , 209 , 76 , 140 , 191 , 175 , 185 , 14 , 50 , 171 , 54 , 171 , 185 , 184 , 239 , 147 , 75 , 233 , 16 , 137 , 66 , 222 , 30 , 12 , 84 , 166 , 165 , 1 , 187 , 11 , 165 , 231 , 175 , 164 , 49 , 111 , 57 , 180 , 13 , 116 , 189 , 53 , 18 , 153 , 226 , 132 , 181 , 91 , 47 , 254 , 200 , 153 , 221 , 146 , 82 , 235 , 232 , 227 , 236 , 213 , 179] , [4 , 206 , 122 , 187 , 250 , 30 , 38 , 132 , 26 , 139 , 164 , 123 , 203 , 69 , 253 , 27 , 85 , 237 , 252 , 100 , 148 , 67 , 250 , 214 , 11 , 87 , 23 , 12 , 198 , 221 , 95 , 44 , 26 , 184 , 61 , 18 , 253 , 40 , 69 , 122 , 180 , 88 , 10 , 83 , 127 , 119 , 60 , 45 , 224 , 232 , 7 , 40 , 100 , 115 , 109 , 59 , 137 , 68 , 171 , 248 , 139 , 81 , 80 , 81] , [4 , 188 , 123 , 189 , 121 , 84 , 134 , 112 , 88 , 93 , 177 , 169 , 124 , 16 , 26 , 252 , 195 , 130 , 87 , 32 , 96 , 245 , 112 , 204 , 39 , 84 , 82 , 154 , 194 , 64 , 131 , 17 , 37 , 229 , 30 , 249 , 181 , 200 , 133 , 221 , 75 , 106 , 158 , 142 , 14 , 35 , 52 , 130 , 147 , 190 , 59 , 33 , 211 , 83 , 70 , 121 , 245 , 193 , 69 , 175 , 6 , 239 , 148 , 123] , [16 , 103 , 74 , 52 , 139 , 67 , 251 , 12 , 40 , 255 , 29 , 98 , 20 , 48 , 255 , 94 , 165 , 104 , 112 , 190 , 153 , 175 , 217 , 49 , 243 , 224 , 3 , 57 , 183 , 141 , 65 , 183 , 28 , 96 , 187 , 227 , 190 , 149 , 159 , 14 , 41 , 118 , 28 , 51 , 177 , 185 , 26 , 170 , 51 , 241 , 180 , 117 , 33 , 36 , 140 , 213 , 188 , 94 , 125 , 236 , 155 , 170 , 86 , 96] , [28 , 244 , 109 , 89 , 81 , 189 , 1 , 4 , 221 , 193 , 35 , 65 , 91 , 198 , 215 , 172 , 152 , 194 , 107 , 239 , 235 , 188 , 105 , 158 , 203 , 83 , 173 , 135 , 126 , 4 , 239 , 159 , 45 , 35 , 107 , 54 , 143 , 70 , 32 , 179 , 62 , 45 , 147 , 193 , 58 , 28 , 43 , 105 , 35 , 231 , 22 , 79 , 17 , 235 , 80 , 228 , 35 , 119 , 238 , 122 , 208 , 17 , 74 , 0] , [23 , 12 , 239 , 116 , 124 , 105 , 206 , 144 , 227 , 188 , 179 , 217 , 16 , 63 , 192 , 54 , 224 , 177 , 85 , 8 , 119 , 40 , 219 , 120 , 96 , 137 , 219 , 205 , 3 , 194 , 219 , 108 , 10 , 149 , 35 , 77 , 32 , 107 , 96 , 117 , 225 , 165 , 194 , 85 , 213 , 11 , 177 , 177 , 208 , 120 , 121 , 198 , 0 , 83 , 240 , 140 , 39 , 196 , 244 , 232 , 155 , 82 , 38 , 96] , [22 , 86 , 205 , 164 , 60 , 154 , 53 , 249 , 6 , 238 , 24 , 119 , 92 , 160 , 255 , 251 , 218 , 61 , 97 , 71 , 88 , 233 , 143 , 58 , 150 , 141 , 135 , 207 , 240 , 193 , 12 , 101 , 36 , 59 , 19 , 144 , 86 , 102 , 196 , 175 , 107 , 228 , 247 , 240 , 11 , 219 , 49 , 164 , 75 , 144 , 211 , 214 , 55 , 91 , 240 , 39 , 25 , 242 , 219 , 222 , 233 , 32 , 41 , 9]] , }"
    }
  ],
  "instructions": [
    {
      "name": "shieldedTransferFirst",
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
          "name": "eventMerkleTree",
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
      "name": "instructionDataShieldedTransferFirst",
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
            "name": "verifierAddressIndex",
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
            "name": "isFillingUtxo",
            "type": "bool"
          }
        ]
      }
    },
    {
      "name": "zKpublicProgramTransaction2In2OutMainProofInputs",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicInUtxoDataHash",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicProgramId",
            "type": "u8"
          },
          {
            "name": "publicTransactionHash",
            "type": "u8"
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
            "name": "isInProgramUtxo",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inOwner",
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
                    18
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
                    18
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
            "name": "isMetaHashUtxo",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "inAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isInAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "isOutProgramUtxo",
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
      "name": "zKpublicProgramTransaction2In2OutMainPublicInputs",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
            "type": {
              "array": [
                "u8",
                2
              ]
            }
          },
          {
            "name": "publicInUtxoDataHash",
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
      "name": "instructionDataLightInstructionPublicProgramTransaction2In2OutMainSecond",
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
            "name": "publicInUtxoHash",
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
            "name": "publicNewAddress",
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
            "name": "publicInUtxoDataHash",
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
