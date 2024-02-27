import ProvenZk

import FormalVerification
import FormalVerification.Common
import FormalVerification.Poseidon

open SemaphoreMTB (F Order)
open SemaphoreMTB renaming VerifyProof_31_30 → gVerifyProof

def hashLevel (d : Bool) (h s : F): F := match d with
  | true => poseidon₂ (vec![h, s])
  | false => poseidon₂ (vec![s, h])

lemma ProofRound_uncps {direction: Bool} {hash: F} {sibling: F} {k: F -> Prop} :
    SemaphoreMTB.ProofRound direction.toZMod hash sibling k ↔ k (hashLevel direction hash sibling) := by
    cases direction <;>
      simp [SemaphoreMTB.ProofRound, Gates.is_bool, Gates.select, Gates.is_bool, Poseidon2_uncps, hashLevel]

lemma MerkleTree.recover_snoc':
  MerkleTree.recover poseidon₂ (ps.snoc p) (ss.snoc s) item = recover poseidon₂ ps ss (hashLevel p s item) := by
  cases p <;> simp [MerkleTree.recover_snoc, hashLevel]

lemma VerifyProof_uncps {PathIndices: Vector Bool D} {Siblings: Vector F D} {Item : F} {k : F -> Prop}:
    gVerifyProof (Item ::ᵥ Siblings) (Vector.map Bool.toZMod PathIndices) k ↔
    k (MerkleTree.recover poseidon₂ PathIndices.reverse Siblings.reverse Item) := by
    repeat (
      cases PathIndices using Vector.casesOn; rename_i _ PathIndices
      cases Siblings using Vector.casesOn; rename_i _ Siblings
    )
    unfold gVerifyProof
    simp [ProofRound_uncps, MerkleTree.recover_snoc']
    simp [MerkleTree.recover]

lemma VerifyProof_uncps' {Index: Fin (2^D)} {Siblings: Vector F D} {Item : F} {k : F -> Prop}:
    gVerifyProof (Item ::ᵥ Siblings) (Index.toBitsBE.reverse.map Bool.toZMod) k ↔
    k (MerkleTree.recoverAtFin poseidon₂ Index Siblings.reverse Item) := by
    simp [VerifyProof_uncps, MerkleTree.recoverAtFin]
