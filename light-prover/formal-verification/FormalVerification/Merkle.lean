import ProvenZk
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Poseidon
import Mathlib

open LightProver (F Order)

def hashLevel (d : Bool) (s h : F): F := match d with
| false => poseidon₂ vec![h,s]
| true => poseidon₂ vec![s,h]

theorem hashLevel_def (d : Bool) (s h : F):
  hashLevel d s h = match d with
  | false => poseidon₂ vec![h,s]
  | true => poseidon₂ vec![s,h] := by rfl

@[simp]
lemma ProveParentHash_rw {d : Bool} {h s : F} {k : F → Prop}:
  LightProver.ProveParentHash d.toZMod h s k ↔
    (k $ hashLevel d s h)
  := by
  cases d <;> simp [LightProver.ProveParentHash, hashLevel]

lemma MerkleTree.recover_succ' {ix : Vector Bool (Nat.succ N)} {proof : Vector F (Nat.succ N)} :
  MerkleTree.recover poseidon₂ ix proof item = hashLevel ix.head proof.head (MerkleTree.recover poseidon₂ ix.tail proof.tail item) := Eq.refl _

@[simp]
theorem MerkleRootGadget_rw {h i : F} {p : Vector F 20} {k : F → Prop}:
  LightProver.MerkleRootGadget_20_20 h i p k ↔ ∃ (hi : i.val < 2^20), k (MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ p.reverse h) := by
  unfold LightProver.MerkleRootGadget_20_20
  simp_rw [Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt, ←exists_and_right]
  rw [exists_swap]
  apply exists_congr
  intro
  rw [←Vector.ofFn_get (v:=p)]
  simp [Vector.getElem_map, ProveParentHash_rw, MerkleTree.recoverAtFin, MerkleTree.recover_succ', Fin.toBitsLE, Fin.toBitsBE, -Vector.ofFn_get]
  rfl

lemma Membership.get_elem_helper {i n : ℕ} {r : Std.Range} (h₁ : i ∈ r) (h₂ : r.stop = n) :
    i < n := h₂ ▸ h₁.2

macro_rules
| `(tactic| get_elem_tactic_trivial) => `(tactic| (exact Membership.get_elem_helper (by assumption) (by rfl)))

lemma InclusionProofStep_rw {l i e r} {k : F → Prop}:
    (LightProver.MerkleRootGadget_20_20 l i e fun gate_0 => Gates.eq gate_0 r ∧ k gate_0) ↔
    (∃ (hi : i.val < 2^20), MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ e.reverse l = r) ∧ k r := by
  simp [MerkleRootGadget_rw]
  apply Iff.intro
  . rintro ⟨_, ⟨_⟩, _⟩; tauto
  . rintro ⟨⟨_, ⟨_⟩⟩⟩; tauto

def Std.Range.toList (r : Std.Range): List Nat := go r.start (r.stop - r.start)  where
  go start
  | 0 => []
  | i + 1 => start :: go (start + 1) i

theorem Std.Range.mem_toList_of_mem {r : Std.Range} (hp : i ∈ r) : i ∈ r.toList := by
  rcases hp with ⟨h₁, h₂⟩
  rcases r with ⟨start, stop, _⟩
  simp at h₁ h₂
  have h₃ : ∃d, stop = start + d := by
    exists stop - start
    apply Eq.symm
    apply Nat.add_sub_cancel'
    apply Nat.le_trans h₁ (Nat.le_of_lt h₂)
  rcases h₃ with ⟨d, ⟨_⟩⟩
  induction d generalizing start i with
  | zero => linarith
  | succ d ih =>
    simp [toList, toList.go]
    cases h₁ with
    | refl => tauto
    | @step m h₁ =>
      simp at h₁
      apply Or.inr
      simp [toList] at ih
      apply ih <;> linarith

@[simp]
lemma InclusionProof_rw {roots leaves inPathIndices inPathElements k}:
  LightProver.InclusionProof_10_10_10_20_10_10_20 roots leaves inPathIndices inPathElements k ↔
  k roots ∧
  ∀i (_: i ∈ [0:10]), ∃ (hi : (inPathIndices[i]).val < 2^20), MerkleTree.recoverAtFin poseidon₂ ⟨(inPathIndices[i]).val, hi⟩ (inPathElements[i]).reverse (leaves[i]) = roots[i] := by
  unfold LightProver.InclusionProof_10_10_10_20_10_10_20
  simp_rw [InclusionProofStep_rw]
  apply Iff.intro
  . intro hp
    repeat rcases hp with ⟨_, hp⟩
    apply And.intro (by rw [←Vector.ofFn_get (v:=roots)]; exact hp)
    intro i ir
    have hir : i ∈ ([0:10].toList) := Std.Range.mem_toList_of_mem ir
    conv at hir => skip -- bug in fin_cases
    fin_cases hir <;> assumption
  . rintro ⟨hk, hp⟩
    repeat apply And.intro (by apply hp _ ⟨by decide, by decide⟩)
    rw [←Vector.ofFn_get (v:=roots)] at hk
    exact hk

@[simp]
lemma MerkleTree.GetElem.def {tree : MerkleTree α H d} {i : ℕ} {ih : i < 2^d}:
  tree[i] = tree.itemAtFin ⟨i, ih⟩ := by rfl


theorem InclusionCircuit_correct {trees : Vector (MerkleTree F poseidon₂ 20) 10} {leaves inPathIndices} [Fact (CollisionResistant poseidon₂)]:
  (∃proofs, LightProver.InclusionCircuit_10_10_10_20_10_10_20 (trees.map (·.root)) leaves inPathIndices proofs) ↔
  ∀i (_: i∈[0:10]), ∃ (hi : (inPathIndices[i]).val < 2^20), trees[i][inPathIndices[i].val] = leaves[i] := by
  unfold LightProver.InclusionCircuit_10_10_10_20_10_10_20

  simp [InclusionProof_rw, MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct]
  apply Iff.intro
  . rintro ⟨_, hp⟩ i ir
    have := hp i ir
    rcases this with ⟨h, _, hp⟩
    exact ⟨h, Eq.symm hp⟩
  . intro hp
    use Vector.ofFn fun (i : Fin 10) => (Vector.reverse $ trees[i.val].proofAtFin ⟨inPathIndices[i].val, (hp i ⟨by simp, i.prop⟩).1⟩)
    intro i ir
    have := hp i ir
    rcases this with ⟨h, hp⟩
    use h
    simp [getElem] at hp
    simp [hp, getElem]
