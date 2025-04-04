import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Merkle

open LightProver (F)

open LightProver renaming InclusionCircuit_8_8_8_32_8_8_32 → InclusionCircuit,
                          NonInclusionCircuit_8_8_8_8_8_40_8_8_40 → NonInclusionCircuit,
                          CombinedCircuit_8_8_8_32_8_8_8_8_8_8_40_8 → CombinedCircuit,
                          BatchAppendCircuit_8_8_32_8_32_8 → BatchAppendWithProofsCircuit,
                          BatchUpdateCircuit_8_8_8_32_8_8_32_8 → BatchUpdateCircuit,
                          BatchAddressTreeAppendCircuit_8_8_8_40_8_8_40_8_8_40 → BatchAddressTreeAppendCircuit

abbrev SD := 32
abbrev AD := 40
abbrev B := 8

theorem poseidon₂_testVector :
  poseidon₂ vec![1, 2] = 7853200120776062878684798364095072458815029376092732009249414926327459813530 := by native_decide

theorem poseidon₃_testVector :
  poseidon₃ vec![1, 2, 3] = 6542985608222806190361240322586112750744169038454362455181422643027100751666 := by native_decide

axiom poseidon₂_collisionResistant : CollisionResistant poseidon₂
instance : Fact (CollisionResistant poseidon₂) := ⟨poseidon₂_collisionResistant⟩

axiom poseidon₃_collisionResistant : CollisionResistant poseidon₃
instance : Fact (CollisionResistant poseidon₃) := ⟨poseidon₃_collisionResistant⟩

axiom poseidon₂_nez : poseidon₂_no_zero_preimage
instance : Fact poseidon₂_no_zero_preimage := ⟨poseidon₂_nez⟩

namespace InclusionCircuit

theorem sound_and_complete
  {trees : List.Vector (MerkleTree F poseidon₂ SD) B}
  {leaves : List.Vector F B}:
    (∃ih p₁ p₂, InclusionCircuit ih (trees.map (·.root)) leaves p₁ p₂)
    ↔ ∀i (_: i∈[0:B]), leaves[i] ∈ trees[i]
  := by simp [InclusionCircuit_correct]

theorem inputHash_deterministic:
    InclusionCircuit h₁ trees leaves i₁ p₁ ∧
    InclusionCircuit h₂ trees leaves i₂ p₂ →
    h₁ = h₂ := by
  simp only [InclusionCircuit_rw]
  intros
  simp_all

theorem inputHash_injective:
    InclusionCircuit h trees₁ leaves₁ i₁ p₁ →
    InclusionCircuit h trees₂ leaves₂ i₂ p₂ →
    trees₁ = trees₂ ∧ leaves₁ = leaves₂ := by
  simp only [InclusionCircuit_rw]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  cases h₁
  exact inputHash_correct.mp h₂

end InclusionCircuit

namespace NonInclusionCircuit

theorem sound_and_complete
  {trees : List.Vector (RangeVector (2^AD)) B}
  {leaves : List.Vector F B}:
    (∃ih p₁ p₂ p₃ p₄,
      NonInclusionCircuit ih (trees.map (·.root)) leaves p₁ p₂ p₃ p₄)
    ↔ ∀i (_: i∈[0:B]), leaves[i].val ∈ trees[i]
  := by
    conv => lhs; arg 1; intro ih; rw [NonInclusionCircuit_correct]
    simp

theorem inputHash_deterministic:
    NonInclusionCircuit h₁ trees leaves lo₁ hi₁ i₁ p₁ →
    NonInclusionCircuit h₂ trees leaves lo₂ hi₂ i₂ p₂ →
    h₁ = h₂ := by
  unfold NonInclusionCircuit
  simp_all [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8, GatesGnark12, GatesGnark9]

theorem inputHash_injective:
    NonInclusionCircuit h trees₁ leaves₁ lo₁ hi₁ i₁ p₁ →
    NonInclusionCircuit h trees₂ leaves₂ lo₂ hi₂ i₂ p₂ →
    trees₁ = trees₂ ∧ leaves₁ = leaves₂ := by
  unfold NonInclusionCircuit
  simp only [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  cases h₁
  exact inputHash_correct.mp h₂

end NonInclusionCircuit

namespace CombinedCircuit

theorem sound_and_complete
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ SD) B}
  {nonInclusionTrees : List.Vector (RangeVector (2^AD)) B}
  {inclusionLeaves nonInclusionLeaves : List.Vector F B}:
    (∃ih p₁ p₂ p₃ p₄ p₅ p₆,
      CombinedCircuit
        ih
        (inclusionTrees.map (·.root)) inclusionLeaves p₁ p₂
        (nonInclusionTrees.map (·.root)) nonInclusionLeaves p₃ p₄ p₅ p₆)
    ↔ ∀i (_: i∈[0:B]), inclusionLeaves[i] ∈ inclusionTrees[i]
                      ∧ nonInclusionLeaves[i].val ∈ nonInclusionTrees[i]
  := by
    conv => lhs; arg 1; intro ih; rw [CombinedCircuit_correct]
    simp

theorem inputHash_deterministic:
    CombinedCircuit h₁ inclusionTrees inclusionLeaves p₁ p₂ nonInclusionTrees nonInclusionLeaves p₃ p₄ p₅ p₆ →
    CombinedCircuit h₂ inclusionTrees inclusionLeaves q₁ q₂ nonInclusionTrees nonInclusionLeaves q₃ q₄ q₅ q₆ →
    h₁ = h₂ := by
  unfold CombinedCircuit
  simp_all [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8, GatesGnark12, GatesGnark9]

theorem inputHash_injective:
    CombinedCircuit h inclusionTrees₁ inclusionLeaves₁ p₁ p₂ nonInclusionTrees₁ nonInclusionLeaves₁ p₃ p₄ p₅ p₆ →
    CombinedCircuit h inclusionTrees₂ inclusionLeaves₂ q₁ q₂ nonInclusionTrees₂ nonInclusionLeaves₂ q₃ q₄ q₅ q₆ →
    inclusionTrees₁ = inclusionTrees₂ ∧ inclusionLeaves₁ = inclusionLeaves₂ ∧ nonInclusionTrees₁ = nonInclusionTrees₂ ∧ nonInclusionLeaves₁ = nonInclusionLeaves₂ := by
  unfold CombinedCircuit
  simp only [TwoInputsHashChain_rw, Poseidon2_iff_uniqueAssignment, LightProver.Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.eq]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  rw [h₁] at h₂
  simp only [CollisionResistant_def, List.Vector.eq_cons, inputHash_correct] at h₂
  simp_all

end CombinedCircuit

namespace BatchAppendWithProofsCircuit

theorem sound_and_complete
  {tree : MerkleTree F poseidon₂ SD} {newRoot startIndex : F} {leaves : List.Vector F B}:
    (∃ih oldLeaves merkleProofs lh,
      BatchAppendWithProofsCircuit
        ih tree.root newRoot lh startIndex oldLeaves leaves merkleProofs)
    ↔
    (startIndex.val + (B - 1) < 2^SD) ∧
    ∃(newTree : MerkleTree F poseidon₂ SD), newRoot = newTree.root ∧
    ∀(i : Fin (2^SD)),
      (i.val ∈ [startIndex.val:(startIndex.val + B)] → newTree[i] = if tree[i] = 0 then leaves[i.val - startIndex.val]! else tree[i]) ∧
      (i.val ∉ [startIndex.val:(startIndex.val + B)] → newTree[i] = tree[i])
  := by
  simp [AppendWithProofs_rw]
  rw [eq_comm, Option.map_eq_some']
  simp [treeAppends_sound_and_complete]
  apply Iff.intro
  · tauto
  · rintro ⟨_, t, _⟩
    use t
    tauto

theorem inputHash_deterministic:
    BatchAppendWithProofsCircuit h₁ oldRoot newRoot p₁ startIndex p₂ leaves p₃ ∧
    BatchAppendWithProofsCircuit h₂ oldRoot newRoot q₁ startIndex q₂ leaves q₃ →
    h₁ = h₂ := by
  intro ⟨hp₁, hp₂⟩
  have := (AppendWithProofs_rw1.mp (Exists.intro _ hp₁)).1
  have := (AppendWithProofs_rw1.mp (Exists.intro _ hp₂)).1
  simp_all

theorem inputHash_injective:
    BatchAppendWithProofsCircuit h oldRoot₁ newRoot₁ p₁ startIndex₁ p₂ leaves₁ p₃ ∧
    BatchAppendWithProofsCircuit h oldRoot₂ newRoot₂ q₁ startIndex₂ q₂ leaves₂ q₃ →
    oldRoot₁ = oldRoot₂ ∧ newRoot₁ = newRoot₂ ∧ startIndex₁ = startIndex₂ ∧ leaves₁ = leaves₂ := by
  intro ⟨hp₁, hp₂⟩
  have := (AppendWithProofs_rw1.mp (Exists.intro _ hp₁)).1
  have := (AppendWithProofs_rw1.mp (Exists.intro _ hp₂)).1
  simp_all [hashChain_injective, List.Vector.eq_cons]

end BatchAppendWithProofsCircuit

namespace BatchUpdateCircuit

theorem sound_and_complete
  {tree : MerkleTree F poseidon₂ SD} {newRoot : F} {leaves txHashes indices : List.Vector F B} (indices_distinct: ∀(i j : Fin B), i ≠ j → indices[i] ≠ indices[j]):
  (∃ih ps ols lhh, BatchUpdateCircuit ih tree.root newRoot lhh txHashes leaves ols ps indices) ↔
    ∃(newTree : MerkleTree F poseidon₂ SD), newRoot = newTree.root ∧
    ∃(hr : ∀ (i:Fin B), indices[i].val < 2^SD),
    (∀i: Fin B, newTree[indices[i].val]'(hr i) = poseidon₃ vec![leaves[i], indices[i], txHashes[i]]) ∧
    (∀i: Fin (2^SD), ↑i.val ∉ indices → newTree[i] = tree[i]) := by
  simp [BatchUpdateCircuit_rw1, batchUpdate_rec_rw, batchUpdates_sem_of_distinct indices_distinct]
  apply Iff.intro <;> {
    rintro ⟨nt, _⟩
    use nt
    simp_all
  }

theorem inputHash_deterministic :
    BatchUpdateCircuit h₁ oldRoot newRoot lhh₁ txHashes leaves ols₁ ps₁ indices ∧
    BatchUpdateCircuit h₂ oldRoot newRoot lhh₂ txHashes leaves ols₂ ps₂ indices →
    h₁ = h₂ := by
  intro ⟨hp₁, hp₂⟩
  have := (BatchUpdateCircuit_rw1.mp (Exists.intro _ hp₁)).1
  have := (BatchUpdateCircuit_rw1.mp (Exists.intro _ hp₂)).1
  simp_all

theorem inputHash_injective :
    BatchUpdateCircuit h oldRoot₁ newRoot₁ lhh₁ txHashes₁ leaves₁ ols₁ ps₁ indices₁ ∧
    BatchUpdateCircuit h oldRoot₂ newRoot₂ lhh₂ txHashes₂ leaves₂ ols₂ ps₂ indices₂ →
    oldRoot₁ = oldRoot₂ ∧ newRoot₁ = newRoot₂ ∧ txHashes₁ = txHashes₂ ∧ leaves₁ = leaves₂ ∧ indices₁ = indices₂ := by
  intro ⟨hp₁, hp₂⟩
  have t₁ := (BatchUpdateCircuit_rw1.mp (Exists.intro _ hp₁)).1
  have := (BatchUpdateCircuit_rw1.mp (Exists.intro _ hp₂)).1
  rw [t₁] at this
  simp_all [hashChain_injective, hashChain3_injective, List.Vector.eq_cons]

end BatchUpdateCircuit

namespace BatchAddressAppendTreeCircuit

theorem sound {ranges : RangeVector (2^AD)} {elements : List.Vector F B} {newRoot : F}:
    (∃pih hch si lev lenv lei lep nep, BatchAddressTreeAppendCircuit pih ranges.root newRoot hch si lev lenv lei lep elements nep) →
    ∃newRanges : RangeVector (2^AD), newRanges.root = newRoot ∧ (∀i ∈ elements, i.val ∉ newRanges) ∧ (∀i, ↑i ∉ elements → (i ∈ ranges ↔ i ∈ newRanges)) := by
  exact BatchAdressAppend_sound

theorem complete {rv : RangeVector (2^AD)} {elements startIndex}
    (startIndex_small : startIndex + B < 2^AD)
    (elements_distinct : ∀(i j : Fin B), i ≠ j → elements[i] ≠ elements[j])
    (elements_mem : ∀ i ∈ elements, i.val ∈ rv)
    (indices_empty : ∀ i ∈ [startIndex:(startIndex + B)], rv.ranges i = none):
    ∃lev lenv lei lep nep newRoot hch pih, BatchAddressTreeAppendCircuit pih rv.root newRoot hch startIndex lev lenv lei lep elements nep := by
  apply BatchAddressAppend_complete <;> simp_all [getElem]
  intro i hi
  have := indices_empty i hi
  simp [←this]
  congr
  rw [Nat.mod_eq_of_lt]
  linarith [hi.2]

theorem inputHash_deterministic:
    BatchAddressTreeAppendCircuit h₁ oldRoot newRoot lhh₁ startIndex lev₁ lenv₁ lei₁ lep₁ elements nep₁ ∧
    BatchAddressTreeAppendCircuit h₂ oldRoot newRoot lhh₂ startIndex lev₂ lenv₂ lei₂ lep₂ elements nep₂ →
    h₁ = h₂ := by
  intro ⟨hp₁, hp₂⟩
  have h₁ := BatchAddressLoop_skip_tree (BatchAddressLoop_rw1.mp hp₁)
  have h₂ := BatchAddressLoop_skip_tree (BatchAddressLoop_rw1.mp hp₂)
  simp [HashChain_B_rw, LightProver.Gates, GatesGnark8, GatesGnark12, GatesGnark9, HashChain_4_rw] at h₁ h₂
  simp_all

theorem inputHash_injective:
    BatchAddressTreeAppendCircuit h oldRoot₁ newRoot₁ lhh₁ startIndex₁ lev₁ lenv₁ lei₁ lep₁ elements₁ nep₁ ∧
    BatchAddressTreeAppendCircuit h oldRoot₂ newRoot₂ lhh₂ startIndex₂ lev₂ lenv₂ lei₂ lep₂ elements₂ nep₂ →
    oldRoot₁ = oldRoot₂ ∧ newRoot₁ = newRoot₂ ∧ startIndex₁ = startIndex₂ ∧ elements₁ = elements₂ := by
  intro ⟨hp₁, hp₂⟩
  have h₁ := BatchAddressLoop_skip_tree (BatchAddressLoop_rw1.mp hp₁)
  have h₂ := BatchAddressLoop_skip_tree (BatchAddressLoop_rw1.mp hp₂)
  simp [HashChain_B_rw, LightProver.Gates, GatesGnark8, GatesGnark12, GatesGnark9, HashChain_4_rw, hashChain_injective] at h₁ h₂
  rcases h₁ with ⟨rfl, h₁⟩
  rcases h₂ with ⟨rfl, h₂⟩
  rw [h₁] at h₂
  simp_all [hashChain_injective, List.Vector.eq_cons]

end BatchAddressAppendTreeCircuit

def main : IO Unit := pure ()
