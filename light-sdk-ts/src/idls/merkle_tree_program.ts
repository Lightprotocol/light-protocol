export type MerkleTreeProgram = {
  version: "0.1.0";
  name: "merkle_tree_program";
  instructions: [
    {
      name: "initializeNewTransactionMerkleTree";
      docs: [
        "Initializes a new Merkle tree from config bytes.",
        "Can only be called from the merkle_tree_authority.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "lockDuration";
          type: "u64";
        },
      ];
    },
    {
      name: "initializeNewMessageMerkleTree";
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "messageMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "initializeMerkleTreeAuthority";
      docs: [
        "Initializes a new merkle tree authority which can register new verifiers and configure",
        "permissions to create new pools.",
      ];
      accounts: [
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "updateMerkleTreeAuthority";
      docs: ["Updates the merkle tree authority to a new authority."];
      accounts: [
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
        {
          name: "newAuthority";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "updateLockDuration";
      docs: ["Updates the lock duration for a specific merkle tree."];
      accounts: [
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "lockDuration";
          type: "u64";
        },
      ];
    },
    {
      name: "enableNfts";
      docs: [
        "Enables permissionless deposits of any spl token with supply of one and zero decimals.",
      ];
      accounts: [
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
      ];
      args: [
        {
          name: "enablePermissionless";
          type: "bool";
        },
      ];
    },
    {
      name: "enablePermissionlessSplTokens";
      docs: ["Enables anyone to create token pools."];
      accounts: [
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: false;
          isSigner: true;
        },
      ];
      args: [
        {
          name: "enablePermissionless";
          type: "bool";
        },
      ];
    },
    {
      name: "registerVerifier";
      docs: [
        "Registers a new verifier which can withdraw tokens, insert new nullifiers, add new leaves.",
        "These functions can only be invoked from registered verifiers.",
      ];
      accounts: [
        {
          name: "registeredVerifierPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeAuthorityPda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "verifierPubkey";
          type: "publicKey";
        },
      ];
    },
    {
      name: "registerPoolType";
      docs: ["Registers a new pooltype."];
      accounts: [
        {
          name: "registeredPoolTypePda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "merkleTreeAuthorityPda";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "poolType";
          type: {
            array: ["u8", 32];
          };
        },
      ];
    },
    {
      name: "registerSplPool";
      docs: [
        "Creates a new spl token pool which can be used by any registered verifier.",
      ];
      accounts: [
        {
          name: "registeredAssetPoolPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "merkleTreePdaToken";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "mint";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredPoolTypePda";
          isMut: false;
          isSigner: false;
          docs: ["Just needs to exist and be derived correctly."];
        },
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "registerSolPool";
      docs: [
        "Creates a new sol pool which can be used by any registered verifier.",
      ];
      accounts: [
        {
          name: "registeredAssetPoolPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredPoolTypePda";
          isMut: false;
          isSigner: false;
        },
        {
          name: "merkleTreeAuthorityPda";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "initializeMerkleTreeUpdateState";
      docs: [
        "Initializes a merkle tree update state pda. This pda stores the leaves to be inserted",
        "and state of the computation of poseidon hashes to update the Merkle tree.",
        "A maximum of 16 pairs of leaves can be passed in as leaves accounts as remaining accounts.",
        "Every leaf is copied into this account such that no further accounts or data have to be",
        "passed in during the following instructions which compute the poseidon hashes to update the tree.",
        "The hashes are computed with the update merkle tree instruction and the new root is inserted",
        "with the insert root merkle tree instruction.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeUpdateState";
          isMut: true;
          isSigner: false;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "rent";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "updateTransactionMerkleTree";
      docs: ["Computes poseidon hashes to update the Merkle tree."];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeUpdateState";
          isMut: true;
          isSigner: false;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "bump";
          type: "u64";
        },
      ];
    },
    {
      name: "insertRootMerkleTree";
      docs: [
        "This is the last step of a Merkle tree update which inserts the prior computed Merkle tree",
        "root.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeUpdateState";
          isMut: true;
          isSigner: false;
          docs: [
            "Merkle tree is locked by merkle_tree_update_state",
            "Is in correct instruction for root insert thus Merkle Tree update has been completed.",
            "The account is closed to the authority at the end of the instruction.",
          ];
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "logWrapper";
          isMut: false;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "bump";
          type: "u64";
        },
      ];
    },
    {
      name: "closeMerkleTreeUpdateState";
      docs: [
        "Closes the Merkle tree update state.",
        "A relayer can only close its own update state account.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeUpdateState";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [];
    },
    {
      name: "insertTwoLeaves";
      docs: [
        "Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.",
        "The inserted leaves are not part of the Merkle tree yet and marked accordingly.",
        "The Merkle tree has to be updated after.",
        "Can only be called from a registered verifier program.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "twoLeavesPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "transactionMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "leafLeft";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "leafRight";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "encryptedUtxo";
          type: {
            array: ["u8", 256];
          };
        },
      ];
    },
    {
      name: "insertTwoLeavesMessage";
      accounts: [
        {
          name: "messageMerkleTree";
          isMut: true;
          isSigner: false;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "leafLeft";
          type: {
            array: ["u8", 32];
          };
        },
        {
          name: "leafRight";
          type: {
            array: ["u8", 32];
          };
        },
      ];
    },
    {
      name: "withdrawSol";
      docs: [
        "Withdraws sol from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeToken";
          isMut: true;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: true;
          isSigner: false;
        },
        {
          name: "recipient";
          isMut: true;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        },
      ];
    },
    {
      name: "withdrawSpl";
      docs: [
        "Withdraws spl tokens from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program.",
      ];
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "merkleTreeToken";
          isMut: true;
          isSigner: false;
        },
        {
          name: "recipient";
          isMut: true;
          isSigner: false;
        },
        {
          name: "tokenProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "tokenAuthority";
          isMut: true;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "amount";
          type: "u64";
        },
      ];
    },
    {
      name: "initializeNullifiers";
      accounts: [
        {
          name: "authority";
          isMut: true;
          isSigner: true;
        },
        {
          name: "systemProgram";
          isMut: false;
          isSigner: false;
        },
        {
          name: "registeredVerifierPda";
          isMut: false;
          isSigner: false;
        },
      ];
      args: [
        {
          name: "nullifiers";
          type: {
            vec: {
              array: ["u8", 32];
            };
          };
        },
      ];
    },
  ];
  accounts: [
    {
      name: "registeredAssetPool";
      docs: [
        "Nullfier pdas are derived from the nullifier",
        "existence of a nullifier is the check to prevent double spends.",
      ];
      type: {
        kind: "struct";
        fields: [
          {
            name: "assetPoolPubkey";
            type: "publicKey";
          },
          {
            name: "poolType";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "index";
            type: "u64";
          },
        ];
      };
    },
    {
      name: "registeredPoolType";
      docs: ["Pool type"];
      type: {
        kind: "struct";
        fields: [
          {
            name: "poolType";
            type: {
              array: ["u8", 32];
            };
          },
        ];
      };
    },
    {
      name: "messageMerkleTree";
      type: {
        kind: "struct";
        fields: [
          {
            name: "merkleTree";
            type: {
              defined: "MerkleTree";
            };
          },
        ];
      };
    },
    {
      name: "merkleTreePdaToken";
      type: {
        kind: "struct";
        fields: [];
      };
    },
    {
      name: "preInsertedLeavesIndex";
      type: {
        kind: "struct";
        fields: [
          {
            name: "nextIndex";
            type: "u64";
          },
        ];
      };
    },
    {
      name: "merkleTreeAuthority";
      docs: [
        "Configures the authority of the merkle tree which can:",
        "- register new verifiers",
        "- register new asset pools",
        "- register new asset pool types",
        "- set permissions for new asset pool creation",
        "- keeps current highest index for assets and merkle trees to enable lookups of these",
      ];
      type: {
        kind: "struct";
        fields: [
          {
            name: "pubkey";
            type: "publicKey";
          },
          {
            name: "merkleTreeIndex";
            type: "u64";
          },
          {
            name: "registeredAssetIndex";
            type: "u64";
          },
          {
            name: "enableNfts";
            type: "bool";
          },
          {
            name: "enablePermissionlessSplTokens";
            type: "bool";
          },
          {
            name: "enablePermissionlessMerkleTreeRegistration";
            type: "bool";
          },
        ];
      };
    },
    {
      name: "merkleTreeUpdateState";
      type: {
        kind: "struct";
        fields: [
          {
            name: "nodeLeft";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "nodeRight";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "leafLeft";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "leafRight";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "relayer";
            type: "publicKey";
          },
          {
            name: "merkleTreePdaPubkey";
            type: "publicKey";
          },
          {
            name: "state";
            type: {
              array: ["u8", 96];
            };
          },
          {
            name: "currentRound";
            type: "u64";
          },
          {
            name: "currentRoundIndex";
            type: "u64";
          },
          {
            name: "currentInstructionIndex";
            type: "u64";
          },
          {
            name: "currentIndex";
            type: "u64";
          },
          {
            name: "currentLevel";
            type: "u64";
          },
          {
            name: "currentLevelHash";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "tmpLeavesIndex";
            type: "u64";
          },
          {
            name: "filledSubtrees";
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                18,
              ];
            };
          },
          {
            name: "leaves";
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 32];
                    },
                    2,
                  ];
                },
                16,
              ];
            };
          },
          {
            name: "numberOfLeaves";
            type: "u8";
          },
          {
            name: "insertLeavesIndex";
            type: "u8";
          },
        ];
      };
    },
    {
      name: "registeredVerifier";
      docs: [""];
      type: {
        kind: "struct";
        fields: [
          {
            name: "pubkey";
            type: "publicKey";
          },
        ];
      };
    },
    {
      name: "transactionMerkleTree";
      type: {
        kind: "struct";
        fields: [
          {
            name: "filledSubtrees";
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                18,
              ];
            };
          },
          {
            name: "currentRootIndex";
            type: "u64";
          },
          {
            name: "nextIndex";
            type: "u64";
          },
          {
            name: "roots";
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                256,
              ];
            };
          },
          {
            name: "pubkeyLocked";
            type: "publicKey";
          },
          {
            name: "timeLocked";
            type: "u64";
          },
          {
            name: "height";
            type: "u64";
          },
          {
            name: "merkleTreeNr";
            type: "u64";
          },
          {
            name: "lockDuration";
            type: "u64";
          },
          {
            name: "nextQueuedIndex";
            type: "u64";
          },
        ];
      };
    },
    {
      name: "twoLeavesBytesPda";
      type: {
        kind: "struct";
        fields: [
          {
            name: "nodeLeft";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "nodeRight";
            type: {
              array: ["u8", 32];
            };
          },
          {
            name: "merkleTreePubkey";
            type: "publicKey";
          },
          {
            name: "encryptedUtxos";
            type: {
              array: ["u8", 256];
            };
          },
          {
            name: "leftLeafIndex";
            type: "u64";
          },
        ];
      };
    },
  ];
  types: [
    {
      name: "MerkleTree";
      type: {
        kind: "struct";
        fields: [
          {
            name: "height";
            docs: ["Height of the Merkle tree."];
            type: "u64";
          },
          {
            name: "filledSubtrees";
            docs: ["Subtree hashes."];
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                18,
              ];
            };
          },
          {
            name: "roots";
            docs: [
              "Full history of roots of the Merkle tree (the last one is the current",
              "one).",
            ];
            type: {
              array: [
                {
                  array: ["u8", 32];
                },
                20,
              ];
            };
          },
          {
            name: "nextIndex";
            docs: ["Next index to insert a leaf."];
            type: "u64";
          },
          {
            name: "currentRootIndex";
            docs: ["Current index of the root."];
            type: "u64";
          },
          {
            name: "hashFunction";
            docs: ["Hash implementation used on the Merkle tree."];
            type: {
              defined: "HashFunction";
            };
          },
        ];
      };
    },
    {
      name: "HashFunction";
      type: {
        kind: "enum";
        variants: [
          {
            name: "Sha256";
          },
          {
            name: "Poseidon";
          },
        ];
      };
    },
  ];
  errors: [
    {
      code: 6000;
      name: "MtTmpPdaInitFailed";
      msg: "Merkle tree tmp account init failed wrong pda.";
    },
    {
      code: 6001;
      name: "MerkleTreeInitFailed";
      msg: "Merkle tree tmp account init failed.";
    },
    {
      code: 6002;
      name: "ContractStillLocked";
      msg: "Contract is still locked.";
    },
    {
      code: 6003;
      name: "InvalidMerkleTree";
      msg: "InvalidMerkleTree.";
    },
    {
      code: 6004;
      name: "InvalidMerkleTreeOwner";
      msg: "InvalidMerkleTreeOwner.";
    },
    {
      code: 6005;
      name: "PubkeyCheckFailed";
      msg: "PubkeyCheckFailed";
    },
    {
      code: 6006;
      name: "CloseAccountFailed";
      msg: "CloseAccountFailed";
    },
    {
      code: 6007;
      name: "WithdrawalFailed";
      msg: "WithdrawalFailed";
    },
    {
      code: 6008;
      name: "MerkleTreeUpdateNotInRootInsert";
      msg: "MerkleTreeUpdateNotInRootInsert";
    },
    {
      code: 6009;
      name: "MerkleTreeUpdateNotInRootInsertState";
      msg: "MerkleTreeUpdateNotInRootInsert";
    },
    {
      code: 6010;
      name: "InvalidNumberOfLeaves";
      msg: "InvalidNumberOfLeaves";
    },
    {
      code: 6011;
      name: "LeafAlreadyInserted";
      msg: "LeafAlreadyInserted";
    },
    {
      code: 6012;
      name: "WrongLeavesLastTx";
      msg: "WrongLeavesLastTx";
    },
    {
      code: 6013;
      name: "FirstLeavesPdaIncorrectIndex";
      msg: "FirstLeavesPdaIncorrectIndex";
    },
    {
      code: 6014;
      name: "NullifierAlreadyExists";
      msg: "NullifierAlreadyExists";
    },
    {
      code: 6015;
      name: "LeavesOfWrongTree";
      msg: "LeavesOfWrongTree";
    },
    {
      code: 6016;
      name: "InvalidAuthority";
      msg: "InvalidAuthority";
    },
    {
      code: 6017;
      name: "InvalidVerifier";
      msg: "InvalidVerifier";
    },
    {
      code: 6018;
      name: "PubkeyTryFromFailed";
      msg: "PubkeyTryFromFailed";
    },
  ];
};

export const IDL: MerkleTreeProgram = {
  version: "0.1.0",
  name: "merkle_tree_program",
  instructions: [
    {
      name: "initializeNewTransactionMerkleTree",
      docs: [
        "Initializes a new Merkle tree from config bytes.",
        "Can only be called from the merkle_tree_authority.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "lockDuration",
          type: "u64",
        },
      ],
    },
    {
      name: "initializeNewMessageMerkleTree",
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "messageMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "initializeMerkleTreeAuthority",
      docs: [
        "Initializes a new merkle tree authority which can register new verifiers and configure",
        "permissions to create new pools.",
      ],
      accounts: [
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "updateMerkleTreeAuthority",
      docs: ["Updates the merkle tree authority to a new authority."],
      accounts: [
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
        {
          name: "newAuthority",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "updateLockDuration",
      docs: ["Updates the lock duration for a specific merkle tree."],
      accounts: [
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "lockDuration",
          type: "u64",
        },
      ],
    },
    {
      name: "enableNfts",
      docs: [
        "Enables permissionless deposits of any spl token with supply of one and zero decimals.",
      ],
      accounts: [
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
      ],
      args: [
        {
          name: "enablePermissionless",
          type: "bool",
        },
      ],
    },
    {
      name: "enablePermissionlessSplTokens",
      docs: ["Enables anyone to create token pools."],
      accounts: [
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: false,
          isSigner: true,
        },
      ],
      args: [
        {
          name: "enablePermissionless",
          type: "bool",
        },
      ],
    },
    {
      name: "registerVerifier",
      docs: [
        "Registers a new verifier which can withdraw tokens, insert new nullifiers, add new leaves.",
        "These functions can only be invoked from registered verifiers.",
      ],
      accounts: [
        {
          name: "registeredVerifierPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeAuthorityPda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "verifierPubkey",
          type: "publicKey",
        },
      ],
    },
    {
      name: "registerPoolType",
      docs: ["Registers a new pooltype."],
      accounts: [
        {
          name: "registeredPoolTypePda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "merkleTreeAuthorityPda",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "poolType",
          type: {
            array: ["u8", 32],
          },
        },
      ],
    },
    {
      name: "registerSplPool",
      docs: [
        "Creates a new spl token pool which can be used by any registered verifier.",
      ],
      accounts: [
        {
          name: "registeredAssetPoolPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "merkleTreePdaToken",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "mint",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredPoolTypePda",
          isMut: false,
          isSigner: false,
          docs: ["Just needs to exist and be derived correctly."],
        },
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "registerSolPool",
      docs: [
        "Creates a new sol pool which can be used by any registered verifier.",
      ],
      accounts: [
        {
          name: "registeredAssetPoolPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredPoolTypePda",
          isMut: false,
          isSigner: false,
        },
        {
          name: "merkleTreeAuthorityPda",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "initializeMerkleTreeUpdateState",
      docs: [
        "Initializes a merkle tree update state pda. This pda stores the leaves to be inserted",
        "and state of the computation of poseidon hashes to update the Merkle tree.",
        "A maximum of 16 pairs of leaves can be passed in as leaves accounts as remaining accounts.",
        "Every leaf is copied into this account such that no further accounts or data have to be",
        "passed in during the following instructions which compute the poseidon hashes to update the tree.",
        "The hashes are computed with the update merkle tree instruction and the new root is inserted",
        "with the insert root merkle tree instruction.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeUpdateState",
          isMut: true,
          isSigner: false,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "rent",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "updateTransactionMerkleTree",
      docs: ["Computes poseidon hashes to update the Merkle tree."],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeUpdateState",
          isMut: true,
          isSigner: false,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "bump",
          type: "u64",
        },
      ],
    },
    {
      name: "insertRootMerkleTree",
      docs: [
        "This is the last step of a Merkle tree update which inserts the prior computed Merkle tree",
        "root.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeUpdateState",
          isMut: true,
          isSigner: false,
          docs: [
            "Merkle tree is locked by merkle_tree_update_state",
            "Is in correct instruction for root insert thus Merkle Tree update has been completed.",
            "The account is closed to the authority at the end of the instruction.",
          ],
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "logWrapper",
          isMut: false,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "bump",
          type: "u64",
        },
      ],
    },
    {
      name: "closeMerkleTreeUpdateState",
      docs: [
        "Closes the Merkle tree update state.",
        "A relayer can only close its own update state account.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeUpdateState",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [],
    },
    {
      name: "insertTwoLeaves",
      docs: [
        "Creates and initializes a pda which stores two merkle tree leaves and encrypted Utxos.",
        "The inserted leaves are not part of the Merkle tree yet and marked accordingly.",
        "The Merkle tree has to be updated after.",
        "Can only be called from a registered verifier program.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "twoLeavesPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "transactionMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "leafLeft",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "leafRight",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "encryptedUtxo",
          type: {
            array: ["u8", 256],
          },
        },
      ],
    },
    {
      name: "insertTwoLeavesMessage",
      accounts: [
        {
          name: "messageMerkleTree",
          isMut: true,
          isSigner: false,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "leafLeft",
          type: {
            array: ["u8", 32],
          },
        },
        {
          name: "leafRight",
          type: {
            array: ["u8", 32],
          },
        },
      ],
    },
    {
      name: "withdrawSol",
      docs: [
        "Withdraws sol from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeToken",
          isMut: true,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipient",
          isMut: true,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "withdrawSpl",
      docs: [
        "Withdraws spl tokens from a liquidity pool.",
        "An arbitrary number of recipients can be passed in with remaining accounts.",
        "Can only be called from a registered verifier program.",
      ],
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "merkleTreeToken",
          isMut: true,
          isSigner: false,
        },
        {
          name: "recipient",
          isMut: true,
          isSigner: false,
        },
        {
          name: "tokenProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "tokenAuthority",
          isMut: true,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "amount",
          type: "u64",
        },
      ],
    },
    {
      name: "initializeNullifiers",
      accounts: [
        {
          name: "authority",
          isMut: true,
          isSigner: true,
        },
        {
          name: "systemProgram",
          isMut: false,
          isSigner: false,
        },
        {
          name: "registeredVerifierPda",
          isMut: false,
          isSigner: false,
        },
      ],
      args: [
        {
          name: "nullifiers",
          type: {
            vec: {
              array: ["u8", 32],
            },
          },
        },
      ],
    },
  ],
  accounts: [
    {
      name: "registeredAssetPool",
      docs: [
        "Nullfier pdas are derived from the nullifier",
        "existence of a nullifier is the check to prevent double spends.",
      ],
      type: {
        kind: "struct",
        fields: [
          {
            name: "assetPoolPubkey",
            type: "publicKey",
          },
          {
            name: "poolType",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "index",
            type: "u64",
          },
        ],
      },
    },
    {
      name: "registeredPoolType",
      docs: ["Pool type"],
      type: {
        kind: "struct",
        fields: [
          {
            name: "poolType",
            type: {
              array: ["u8", 32],
            },
          },
        ],
      },
    },
    {
      name: "messageMerkleTree",
      type: {
        kind: "struct",
        fields: [
          {
            name: "merkleTree",
            type: {
              defined: "MerkleTree",
            },
          },
        ],
      },
    },
    {
      name: "merkleTreePdaToken",
      type: {
        kind: "struct",
        fields: [],
      },
    },
    {
      name: "preInsertedLeavesIndex",
      type: {
        kind: "struct",
        fields: [
          {
            name: "nextIndex",
            type: "u64",
          },
        ],
      },
    },
    {
      name: "merkleTreeAuthority",
      docs: [
        "Configures the authority of the merkle tree which can:",
        "- register new verifiers",
        "- register new asset pools",
        "- register new asset pool types",
        "- set permissions for new asset pool creation",
        "- keeps current highest index for assets and merkle trees to enable lookups of these",
      ],
      type: {
        kind: "struct",
        fields: [
          {
            name: "pubkey",
            type: "publicKey",
          },
          {
            name: "merkleTreeIndex",
            type: "u64",
          },
          {
            name: "registeredAssetIndex",
            type: "u64",
          },
          {
            name: "enableNfts",
            type: "bool",
          },
          {
            name: "enablePermissionlessSplTokens",
            type: "bool",
          },
          {
            name: "enablePermissionlessMerkleTreeRegistration",
            type: "bool",
          },
        ],
      },
    },
    {
      name: "merkleTreeUpdateState",
      type: {
        kind: "struct",
        fields: [
          {
            name: "nodeLeft",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "nodeRight",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "leafLeft",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "leafRight",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "relayer",
            type: "publicKey",
          },
          {
            name: "merkleTreePdaPubkey",
            type: "publicKey",
          },
          {
            name: "state",
            type: {
              array: ["u8", 96],
            },
          },
          {
            name: "currentRound",
            type: "u64",
          },
          {
            name: "currentRoundIndex",
            type: "u64",
          },
          {
            name: "currentInstructionIndex",
            type: "u64",
          },
          {
            name: "currentIndex",
            type: "u64",
          },
          {
            name: "currentLevel",
            type: "u64",
          },
          {
            name: "currentLevelHash",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "tmpLeavesIndex",
            type: "u64",
          },
          {
            name: "filledSubtrees",
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                18,
              ],
            },
          },
          {
            name: "leaves",
            type: {
              array: [
                {
                  array: [
                    {
                      array: ["u8", 32],
                    },
                    2,
                  ],
                },
                16,
              ],
            },
          },
          {
            name: "numberOfLeaves",
            type: "u8",
          },
          {
            name: "insertLeavesIndex",
            type: "u8",
          },
        ],
      },
    },
    {
      name: "registeredVerifier",
      docs: [""],
      type: {
        kind: "struct",
        fields: [
          {
            name: "pubkey",
            type: "publicKey",
          },
        ],
      },
    },
    {
      name: "transactionMerkleTree",
      type: {
        kind: "struct",
        fields: [
          {
            name: "filledSubtrees",
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                18,
              ],
            },
          },
          {
            name: "currentRootIndex",
            type: "u64",
          },
          {
            name: "nextIndex",
            type: "u64",
          },
          {
            name: "roots",
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                256,
              ],
            },
          },
          {
            name: "pubkeyLocked",
            type: "publicKey",
          },
          {
            name: "timeLocked",
            type: "u64",
          },
          {
            name: "height",
            type: "u64",
          },
          {
            name: "merkleTreeNr",
            type: "u64",
          },
          {
            name: "lockDuration",
            type: "u64",
          },
          {
            name: "nextQueuedIndex",
            type: "u64",
          },
        ],
      },
    },
    {
      name: "twoLeavesBytesPda",
      type: {
        kind: "struct",
        fields: [
          {
            name: "nodeLeft",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "nodeRight",
            type: {
              array: ["u8", 32],
            },
          },
          {
            name: "merkleTreePubkey",
            type: "publicKey",
          },
          {
            name: "encryptedUtxos",
            type: {
              array: ["u8", 256],
            },
          },
          {
            name: "leftLeafIndex",
            type: "u64",
          },
        ],
      },
    },
  ],
  types: [
    {
      name: "MerkleTree",
      type: {
        kind: "struct",
        fields: [
          {
            name: "height",
            docs: ["Height of the Merkle tree."],
            type: "u64",
          },
          {
            name: "filledSubtrees",
            docs: ["Subtree hashes."],
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                18,
              ],
            },
          },
          {
            name: "roots",
            docs: [
              "Full history of roots of the Merkle tree (the last one is the current",
              "one).",
            ],
            type: {
              array: [
                {
                  array: ["u8", 32],
                },
                20,
              ],
            },
          },
          {
            name: "nextIndex",
            docs: ["Next index to insert a leaf."],
            type: "u64",
          },
          {
            name: "currentRootIndex",
            docs: ["Current index of the root."],
            type: "u64",
          },
          {
            name: "hashFunction",
            docs: ["Hash implementation used on the Merkle tree."],
            type: {
              defined: "HashFunction",
            },
          },
        ],
      },
    },
    {
      name: "HashFunction",
      type: {
        kind: "enum",
        variants: [
          {
            name: "Sha256",
          },
          {
            name: "Poseidon",
          },
        ],
      },
    },
  ],
  errors: [
    {
      code: 6000,
      name: "MtTmpPdaInitFailed",
      msg: "Merkle tree tmp account init failed wrong pda.",
    },
    {
      code: 6001,
      name: "MerkleTreeInitFailed",
      msg: "Merkle tree tmp account init failed.",
    },
    {
      code: 6002,
      name: "ContractStillLocked",
      msg: "Contract is still locked.",
    },
    {
      code: 6003,
      name: "InvalidMerkleTree",
      msg: "InvalidMerkleTree.",
    },
    {
      code: 6004,
      name: "InvalidMerkleTreeOwner",
      msg: "InvalidMerkleTreeOwner.",
    },
    {
      code: 6005,
      name: "PubkeyCheckFailed",
      msg: "PubkeyCheckFailed",
    },
    {
      code: 6006,
      name: "CloseAccountFailed",
      msg: "CloseAccountFailed",
    },
    {
      code: 6007,
      name: "WithdrawalFailed",
      msg: "WithdrawalFailed",
    },
    {
      code: 6008,
      name: "MerkleTreeUpdateNotInRootInsert",
      msg: "MerkleTreeUpdateNotInRootInsert",
    },
    {
      code: 6009,
      name: "MerkleTreeUpdateNotInRootInsertState",
      msg: "MerkleTreeUpdateNotInRootInsert",
    },
    {
      code: 6010,
      name: "InvalidNumberOfLeaves",
      msg: "InvalidNumberOfLeaves",
    },
    {
      code: 6011,
      name: "LeafAlreadyInserted",
      msg: "LeafAlreadyInserted",
    },
    {
      code: 6012,
      name: "WrongLeavesLastTx",
      msg: "WrongLeavesLastTx",
    },
    {
      code: 6013,
      name: "FirstLeavesPdaIncorrectIndex",
      msg: "FirstLeavesPdaIncorrectIndex",
    },
    {
      code: 6014,
      name: "NullifierAlreadyExists",
      msg: "NullifierAlreadyExists",
    },
    {
      code: 6015,
      name: "LeavesOfWrongTree",
      msg: "LeavesOfWrongTree",
    },
    {
      code: 6016,
      name: "InvalidAuthority",
      msg: "InvalidAuthority",
    },
    {
      code: 6017,
      name: "InvalidVerifier",
      msg: "InvalidVerifier",
    },
    {
      code: 6018,
      name: "PubkeyTryFromFailed",
      msg: "PubkeyTryFromFailed",
    },
  ],
};
