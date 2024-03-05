import ProvenZk

import FormalVerification
import FormalVerification.Common
import FormalVerification.Poseidon

import FormalVerification.MerkleProofs

open SemaphoreMTB (F Order)

open SemaphoreMTB renaming DeletionRound_30_30 → gDeletionRound
open SemaphoreMTB renaming DeletionProof_4_4_30_4_4_30 → gDeletionProof
open SemaphoreMTB renaming VerifyProof_31_30 → gVerifyProof

namespace Deletion

def deletionRoundSemantics (Index Item : F) (Tree : MerkleTree F poseidon₂ D) (Proof : Vector F D) (k : MerkleTree F poseidon₂ D → Prop): Prop :=
  if Index.val < 2 ^ (D + 1)
    then if h : Index.val < 2 ^ D
      then Tree.itemAtFin ⟨Index.val, h⟩ = Item ∧
           Tree.proofAtFin ⟨Index.val, h⟩ = Proof.reverse ∧
           k (Tree.setAtFin ⟨Index.val, h⟩ 0)
      else k Tree
    else False

theorem deletionRoundCircuit_eq_deletionRoundSemantics [Fact (CollisionResistant poseidon₂)]:
  gDeletionRound tree.root index item proof k ↔ deletionRoundSemantics index item tree proof (fun t => k t.root) := by
  unfold gDeletionRound
  unfold deletionRoundSemantics
  rw [Vector.exists_succ_iff_exists_snoc]
  simp only [Vector.getElem_snoc_before_length, Vector.getElem_snoc_at_length]
  conv =>
    pattern (occs := *) _ ::ᵥ _
    . change item ::ᵥ Vector.ofFn proof.get
    . change Vector.ofFn vs.get
    . change 0 ::ᵥ Vector.ofFn proof.get
  simp_rw [Vector.ofFn_get, Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt]
  unfold Fin.toBitsLE
  unfold Fin.toBitsBE
  cases Decidable.em (index.val < 2^(D+1)) with
  | inl hlt =>
    cases Nat.lt_or_ge index.val (2^D) with
    | inl hlt =>
      simp [*, VerifyProof_uncps', sub_eq_zero, MerkleTree.root_setAtFin_eq_recoverAtFin]
      apply Iff.intro <;> {
        intros; casesm* _ ∧ _; simp [*] at *; assumption
      }
    | inr hge =>
      have : ¬index.val < 2 ^ D := by linarith
      simp [*, VerifyProof_uncps, sub_eq_zero]
  | inr hge => simp [*]

def deletionRoundsSemantics {b : Nat}
  (indices : Vector F b)
  (items : Vector F b)
  (proofs : Vector (Vector F D) b)
  (tree : MerkleTree F poseidon₂ D)
  (k : F → Prop): Prop := match b with
  | Nat.zero => k tree.root
  | Nat.succ _ =>
    deletionRoundSemantics (indices.head) (items.head) tree (proofs.head) (fun t => deletionRoundsSemantics indices.tail items.tail proofs.tail t k)

theorem deletionProofCircuit_eq_deletionRoundsSemantics [Fact (CollisionResistant poseidon₂)]:
  gDeletionProof indices tree.root idComms proofs k ↔ deletionRoundsSemantics indices idComms proofs tree k := by
  unfold gDeletionProof
  repeat unfold deletionRoundsSemantics
  repeat (
    cases indices using Vector.casesOn; rename_i _ indices
    cases idComms using Vector.casesOn; rename_i _ idComms
    cases proofs using Vector.casesOn; rename_i _ proofs
  )
  simp_rw [deletionRoundCircuit_eq_deletionRoundSemantics]
  rfl

def treeTransformationSemantics {B : ℕ}
  (tree : MerkleTree F poseidon₂ D)
  (indices : Vector F B): Option (MerkleTree F poseidon₂ D) := match B with
  | 0 => some tree
  | _ + 1 => if h : indices.head.val < 2 ^ D
    then treeTransformationSemantics (tree.setAtFin ⟨indices.head.val, h⟩ 0) indices.tail
    else if indices.head.val < 2 ^ (D + 1)
      then treeTransformationSemantics tree indices.tail
      else none

theorem deletionRounds_rootTransformation {B : ℕ} {indices idComms : Vector F B} {proofs : Vector (Vector F D) B} {tree : MerkleTree F poseidon₂ D} {k : F → Prop}:
  deletionRoundsSemantics indices idComms proofs tree k →
  ∃postTree, treeTransformationSemantics tree indices = some postTree ∧ k postTree.root := by
  intro hp
  induction B generalizing tree with
  | zero => exists tree
  | succ B ih =>
    unfold deletionRoundsSemantics at hp
    unfold deletionRoundSemantics at hp
    split at hp
    . split at hp
      . rcases hp with ⟨-, -, hp⟩
        replace hp := ih hp
        unfold treeTransformationSemantics
        simp [*]
      . unfold treeTransformationSemantics
        replace hp := ih hp
        simp [*]
    . contradiction

theorem treeTransform_get_absent {B : ℕ} {i : F} {indices : Vector F B} {tree tree' : MerkleTree F poseidon₂ D}:
  treeTransformationSemantics tree indices = some tree' → i ∉ indices → tree'[i.val]? = tree[i.val]? := by
  intro hp hn
  induction B generalizing tree tree' with
  | zero => unfold treeTransformationSemantics at hp; injection hp; simp [*]
  | succ B ih =>
    unfold treeTransformationSemantics at hp
    have i_tail : i ∉ indices.tail := by
      intro h
      apply hn
      apply Vector.mem_of_mem_tail
      assumption
    split at hp
    . replace hp := ih hp i_tail
      rw [hp]; clear hp
      cases Nat.lt_or_ge i.val (2^D) with
      | inl _ =>
        repeat rw [getElem?_eq_some_getElem_of_valid_index] <;> try assumption
        apply congrArg
        apply MerkleTree.itemAtFin_setAtFin_invariant_of_neq
        intro hp
        apply hn
        injection hp with hp
        cases (Fin.eq_of_veq hp)
        apply Vector.head_mem
      | inr _ =>
        repeat rw [getElem?_none_of_invalid_index]
        all_goals (apply not_lt_of_ge; assumption)
    . split at hp
      . exact ih hp i_tail
      . contradiction

theorem treeTranform_get_present {B : ℕ} {i : F} {indices : Vector F B} {tree tree' : MerkleTree F poseidon₂ D}:
  treeTransformationSemantics tree indices = some tree' → i ∈ indices → tree'[i.val]! = 0 := by
  intro hp hi
  induction B generalizing tree tree' with
  | zero => cases indices using Vector.casesOn; cases hi
  | succ B ih =>
    unfold treeTransformationSemantics at hp
    cases indices using Vector.casesOn; rename_i hix tix
    split at hp
    . rename_i range
      cases Decidable.em (i ∈ tix.toList) with
      | inl h => exact ih hp h
      | inr h =>
        rw [getElem!_eq_getElem?_get!]
        rw [treeTransform_get_absent hp h]
        cases eq_or_ne i hix with
        | inl heq =>
          cases heq
          rw [getElem?_eq_some_getElem_of_valid_index] <;> try exact range
          simp [getElem]
        | inr hne => cases hi <;> contradiction
    . rename_i invalid
      cases List.eq_or_ne_mem_of_mem hi with
      | inl heq =>
        rw [getElem!_eq_getElem?_get!, getElem?_none_of_invalid_index]
        . rfl
        . rw [heq]; exact invalid
      | inr h =>
        rcases h with ⟨-, range⟩
        split at hp
        . exact ih hp range
        . contradiction

theorem exists_assignment {B : ℕ} {indices : Vector F B} {tree : MerkleTree F poseidon₂ D} (ixesOk : ∀i ∈ indices, i.val < 2 ^ (D+1)):
  ∃items proofs postRoot, deletionRoundsSemantics indices items proofs tree (fun t => t = postRoot):= by
  induction B generalizing tree with
  | zero => simp [deletionRoundsSemantics]
  | succ B ih =>
    cases indices using Vector.casesOn with | cons i indices =>
    simp [deletionRoundsSemantics, deletionRoundSemantics, ixesOk]
    split
    . have := ih (indices := indices) (tree := tree.setAtFin ⟨i.val, by assumption⟩ 0) (by
        intro i hi
        apply ixesOk
        apply Vector.mem_of_mem_tail
        simp
        exact hi
      )
      rcases this with ⟨items, proofs, postRoot, h⟩
      rw [Vector.exists_succ_iff_exists_cons]
      apply Exists.intro
      exists items
      rw [Vector.exists_succ_iff_exists_cons]
      apply Exists.intro
      exists proofs
      exists postRoot
      simp [←Vector.reverse_eq]
      exact ⟨by rfl, by rfl, h⟩
    . have := ih (indices := indices) (tree := tree) (by
        intro i hi
        apply ixesOk
        apply Vector.mem_of_mem_tail
        simp
        exact hi
      )
      rcases this with ⟨items, proofs, h⟩
      exists (0 ::ᵥ items)
      exists (Vector.replicate D 0 ::ᵥ proofs)

end Deletion
