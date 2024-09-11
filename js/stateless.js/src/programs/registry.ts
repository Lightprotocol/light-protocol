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
import { confirmConfig, getRegisteredProgramPda } from '../constants';
import { bn } from '../state';

const ACCOUNT_COMPRESSION_PROGRAM_ID = new PublicKey("compr6CUsB5m2jS4Y3831ztGSTnDpnKJTKS95d64XVq");


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
            programId: ACCOUNT_COMPRESSION_PROGRAM_ID,
        });

        const queueAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: queueKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(queueSize),
            space: queueSize,
            programId: ACCOUNT_COMPRESSION_PROGRAM_ID,
        });

        const cpiContextSize = 20 * 1024 + 8; 
        const cpiContextAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: cpiContextKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(cpiContextSize),
            space: cpiContextSize,
            programId: SystemProgram.programId,
        });

        const cpiAuthorityPda = PublicKey.findProgramAddressSync([Buffer.from("cpi_authority")], this.programId);
        console.log("cpiAuthorityPda address:", cpiAuthorityPda[0].toBase58());
  
        const protocolConfigPda = PublicKey.findProgramAddressSync([Buffer.from([97, 117, 116, 104, 111, 114, 105, 116, 121])], this.programId)[0];
        console.log("protocolconfigPda address:", protocolConfigPda.toBase58());
        
        console.log("rpc conn", rpc.rpcEndpoint)
        const protocolConfigAccountInfo = await this.program.provider.connection.getAccountInfo(protocolConfigPda);
        const protocolConfigData = this.program.coder.accounts.decode('ProtocolConfigPda', protocolConfigAccountInfo!.data);
        console.log("Protocol configpda, ", protocolConfigData.config.networkFee.toNumber(), "conf", protocolConfigData.config);
       
        
        const registeredProgramPda = getRegisteredProgramPda();
       
    
        console.log("data programOwner", programOwner?.toBase58());
        console.log("data forester", forester?.toBase58());
        console.log("data stateMerkleTreeConfig", stateMerkleTreeConfig);
        console.log("data nullifierQueueConfig", nullifierQueueConfig);


        
        const initializeInstruction = await this.program.methods
            .initializeStateMerkleTree(
                cpiAuthorityPda[1],
                programOwner,
                forester,
                stateMerkleTreeConfig,
                nullifierQueueConfig
            )
            .accounts({
                authority: payer.publicKey,
                merkleTree: merkleTreeKeypair.publicKey,
                queue: queueKeypair.publicKey,
                registeredProgramPda,
                cpiAuthority: cpiAuthorityPda[0],
                accountCompressionProgram: ACCOUNT_COMPRESSION_PROGRAM_ID,
                protocolConfigPda: protocolConfigPda,
                cpiContextAccount: cpiContextKeypair.publicKey,
                lightSystemProgram: this.programId,
            })
            .instruction();

        const encConf = this.program.coder.types.encode('StateMerkleTreeConfig', stateMerkleTreeConfig);
        console.log("encoded config", encConf);
        const decConf = this.program.coder.types.decode('StateMerkleTreeConfig', encConf);
        console.log("decoded config", decConf);

        return [
            cpiContextAccountCreateIx,
            merkleTreeAccountCreateIx,
            queueAccountCreateIx,
            initializeInstruction
        ];
    }
}
