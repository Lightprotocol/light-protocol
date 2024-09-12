import { Program, AnchorProvider, setProvider, BN } from '@coral-xyz/anchor';
import {
    PublicKey,
    Keypair,
    Connection,
    TransactionInstruction,
    SystemProgram,
} from '@solana/web3.js';
import {
    IDL,
    LightRegistry as LightRegistryIDL,
} from '../idls/light_registry';
import { useWallet } from '../wallet';
import { Rpc } from '../rpc';
import { bn } from '../state';
import { LightSystemProgram } from './system';
import { defaultStaticAccountsStruct, getRegisteredProgramPda } from '../constants';

const {accountCompressionProgram} = defaultStaticAccountsStruct()


export class LightRegistryProgram {
    /**
     * @internal
     */
    constructor() {}

    /**
     * Public key that identifies the LightRegistry program
     */
    static programId: PublicKey = new PublicKey(
        'Lighton6oQpVkeewmo2mcPTQQp7kYHr4fWpAgJyEmDX'
    );

    private static _program: Program<LightRegistryIDL> | null = null;

    static get program(): Program<LightRegistryIDL> {
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
     * Creates instructions to initialize a new state tree and nullifier queue.
     */
    static async createStateTreeAndNullifierQueueInstructions(
        rpc: Rpc,
        payer: Keypair,
        merkleTreeKeypair: Keypair,
        queueKeypair: Keypair,
        cpiContextKeypair: Keypair,
        programOwner: PublicKey | null,
        forester: PublicKey | null,
        index: number
    ): Promise<TransactionInstruction[]> {
        const stateMerkleTreeConfig = {
            height: 26,
            changelogSize: bn(1400),
            rootsSize: bn(2400),
            canopyDepth: bn(10),
            networkFee: bn(5000),
            rolloverThreshold: bn(0),
            closeThreshold: null,
        };
        const nullifierQueueConfig = {
            capacity: 28807,
            sequenceThreshold: new BN(2400),
            networkFee: new BN(5000),
        };

        const merkleTreeSize =1364288;
        const queueSize = 1382992;
        const merkleTreeAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: merkleTreeKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(merkleTreeSize),
            space: merkleTreeSize,
            programId: accountCompressionProgram,
        });

        const queueAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: queueKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(queueSize),
            space: queueSize,
            programId: accountCompressionProgram,
        });

        const cpiContextSize = 20 * 1024 + 8; 
        const cpiContextAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: cpiContextKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(cpiContextSize),
            space: cpiContextSize,
            programId: LightSystemProgram.programId,
        });

        const registeredProgramPda = getRegisteredProgramPda(this.programId);
      
        const [cpiAuthorityPda, bump] = PublicKey.findProgramAddressSync([Buffer.from("cpi_authority")], this.programId);

        const protocolConfigPda = PublicKey.findProgramAddressSync([Buffer.from("authority")], this.programId)[0];

        const initializeInstruction = await this.program.methods
            .initializeStateMerkleTree(
                bump,
                programOwner,
                forester,
                stateMerkleTreeConfig,
                nullifierQueueConfig
            )
            .accounts({
                authority: payer.publicKey,
                registeredProgramPda,
                merkleTree: merkleTreeKeypair.publicKey,
                queue: queueKeypair.publicKey,
                cpiAuthority: cpiAuthorityPda,
                accountCompressionProgram,
                protocolConfigPda,
                lightSystemProgram: LightSystemProgram.programId,
                cpiContextAccount: cpiContextKeypair.publicKey,
            })
            .instruction();

        return [
            cpiContextAccountCreateIx,
            merkleTreeAccountCreateIx,
            queueAccountCreateIx,
            initializeInstruction
        ];
    }
}