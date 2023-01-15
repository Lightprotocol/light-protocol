export declare function executeUpdateMerkleTreeTransactions({ signer, merkleTreeProgram, leavesPdas, merkleTree, merkleTreeIndex, merkle_tree_pubkey, connection, provider, }: {
    signer: any;
    merkleTreeProgram: any;
    leavesPdas: any;
    merkleTree: any;
    merkleTreeIndex: any;
    merkle_tree_pubkey: any;
    connection: any;
    provider: any;
}): Promise<void>;
export declare function executeMerkleTreeUpdateTransactions({ merkleTreeProgram, merkleTreeUpdateState, merkle_tree_pubkey, provider, signer, numberOfTransactions, }: {
    merkleTreeProgram: any;
    merkleTreeUpdateState: any;
    merkle_tree_pubkey: any;
    provider: any;
    signer: any;
    numberOfTransactions: any;
}): Promise<undefined>;
