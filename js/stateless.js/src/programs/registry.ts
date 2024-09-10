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
import { AccountCompressionProgram } from './account-compression';
import { confirmConfig, getRegisteredProgramPda } from '../constants';

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
        cpiAuthorityKeypair: Keypair,
        programOwner: PublicKey | null,
        forester: PublicKey | null,
        index: number
    ): Promise<TransactionInstruction[]> {
        const stateMerkleTreeConfig = {
            height: 26,
            changelogSize: new BN(1400),
            rootsSize: new BN(2400),
            canopyDepth: 10,
            networkFee: null,
            rolloverThreshold: new BN(0),
            closeThreshold: null,
        };

        const nullifierQueueConfig = {
            capacity: 28807,
            sequenceThreshold: new BN(2400),
            networkFee: null,
        };

        const merkleTreeSize = AccountCompressionProgram.program.account.stateMerkleTreeAccount.size;
        const queueSize = AccountCompressionProgram.program.account.queueAccount.size;

        const merkleTreeAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: merkleTreeKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(merkleTreeSize),
            space: merkleTreeSize,
            programId: AccountCompressionProgram.programId,
        });

        const queueAccountCreateIx = SystemProgram.createAccount({
            fromPubkey: payer.publicKey,
            newAccountPubkey: queueKeypair.publicKey,
            lamports: await rpc.getMinimumBalanceForRentExemption(queueSize),
            space: queueSize,
            programId: AccountCompressionProgram.programId,
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
        const cpiAuthoritySize = (await rpc.getAccountInfo(cpiAuthorityPda[0]))?.data.length;
        console.log("cpiAuthoritySize", cpiAuthoritySize);
        
        
        // const cpiAuthorityAccountCreateIx = SystemProgram.createAccount({
        //     fromPubkey: payer.publicKey,
        //     newAccountPubkey: cpiAuthorityKeypair.publicKey,
        //     lamports: await rpc.getMinimumBalanceForRentExemption(cpiAuthoritySize!),
        //     space: cpiAuthoritySize!,
        //     programId: SystemProgram.programId,
        // });

       
        const registeredProgramPda = getRegisteredProgramPda();
       
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
                cpiAuthority: cpiAuthorityKeypair.publicKey,
                accountCompressionProgram: AccountCompressionProgram.programId,
                protocolConfigPda: PublicKey.findProgramAddressSync([Buffer.from([97, 117, 116, 104, 111, 114, 105, 116, 121])], this.programId)[0],
                cpiContextAccount: cpiContextKeypair.publicKey,
                lightSystemProgram: this.programId,
                // systemProgram: SystemProgram.programId,
            })
            .instruction();

        return [
            cpiContextAccountCreateIx,
            // cpiAuthorityAccountCreateIx,
            merkleTreeAccountCreateIx,
            queueAccountCreateIx,
            initializeInstruction
        ];
    }
}
