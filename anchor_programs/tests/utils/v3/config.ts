import * as anchor from "@project-serum/anchor";
import { Program } from '@project-serum/anchor';
import { MerkleTreeProgram } from '../../../target/types/merkle_tree_program';
import { VerifierProgram } from '../../../target/types/verifier_program';

export async function getConfig(network: Network) {
	if(Network.MAINNET === network) {
    console.log("Not implemented yet");
	}
	else if(Network.DEVNET === network) {
    console.log("Not implemented yet");
  }
	else if (Network.LOCAL === network) {
		anchor.setProvider(anchor.AnchorProvider.env());
    const provider = anchor.AnchorProvider.local('http://127.0.0.1:8899', {preflightCommitment: "finalized", commitment: "finalized"});//anchor.getProvider(); // Obv. replace local and 127.0.0.1 with mainnet info
    const verifierProgram = await anchor.workspace.VerifierProgram as Program<VerifierProgram>;
		const merkleTreeProgram = await anchor.workspace.MerkleTreeProgram as Program<MerkleTreeProgram>;
		return {provider, verifierProgram, merkleTreeProgram}
	}
}


export enum Network {
  MAINNET = 'mainnet-beta',
  DEVNET = 'devnet',
  LOCAL = 'local',
}
