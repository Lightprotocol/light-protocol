import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    Keypair,
    Connection,
    TransactionInstruction,
} from '@solana/web3.js';

import {
    IDL,
    AccountCompression as AccountCompressionIDL,
} from '../idls/account_compression';
import { useWallet } from '../wallet';
import { bn } from '../state';

export class AccountCompressionProgram {
    /**
     * @internal
     */
    constructor() {}

    /**
     * Public key that identifies the AccountCompression program
     */
    static programId: PublicKey = new PublicKey(
        'compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq',
    );

    private static _program: Program<AccountCompressionIDL> | null = null;

    static get program(): Program<AccountCompressionIDL> {
        if (!this._program) {
            this.initializeProgram();
        }
        return this._program!;
    }

    /**
     * Initializes the program statically if not already initialized.
     */
    private static initializeProgram() {
        if (!this._program) {
            const mockKeypair = Keypair.generate();
            const mockConnection = new Connection(
                'http://127.0.0.1:8899',
                'confirmed',
            );
            const mockProvider = new AnchorProvider(
                mockConnection,
                useWallet(mockKeypair),
                {
                    commitment: 'confirmed',
                    preflightCommitment: 'confirmed',
                },
            );
            setProvider(mockProvider);
            this._program = new Program(IDL, this.programId, mockProvider);
        }
    }
    /**
     * Creates an instruction to initialize a new state tree and nullifier queue.
     */
    static async initStateTreeAndNullifierQueue(
        authority: PublicKey,
        index: number,
        programOwner: PublicKey | null,
        forester: PublicKey | null,
    ): Promise<TransactionInstruction> {
        const merkleTree = Keypair.generate();
        const nullifierQueue = Keypair.generate();


        const stateMerkleTreeConfig = {
            height: 26,
            changelogSize: bn(1400),
            rootsSize: bn(2400),
            canopyDepth: bn(10),
            networkFee: null,
            rolloverThreshold: bn(0),
            closeThreshold: null,
        };

        const nullifierQueueConfig = {
            capacity: 28807,
            sequenceThreshold: bn(2400),
            networkFee: null,
        };

        const additionalBytes = bn(0);

        const instruction = await this.program.methods
            .initializeStateMerkleTreeAndNullifierQueue(
                new BN(index),
                programOwner,
                forester,
                stateMerkleTreeConfig,
                nullifierQueueConfig,
                additionalBytes 
            )
            .accounts({
                authority,
                merkleTree: merkleTree.publicKey,
                nullifierQueue: nullifierQueue.publicKey,
                registeredProgramPda: null,
                // systemProgram: SystemProgram.programId,
            })
            .instruction();

        return instruction;
    }
}
