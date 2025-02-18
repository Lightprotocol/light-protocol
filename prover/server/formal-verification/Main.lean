import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Merkle

open LightProver (F)

theorem poseidon₂_testVector :
  poseidon₂ vec![1, 2] = 7853200120776062878684798364095072458815029376092732009249414926327459813530 := rfl

theorem poseidon₃_testVector :
  poseidon₃ vec![1, 2, 3] = 6542985608222806190361240322586112750744169038454362455181422643027100751666 := rfl

axiom poseidon₂_collisionResistant : CollisionResistant poseidon₂
instance : Fact (CollisionResistant poseidon₂) := ⟨poseidon₂_collisionResistant⟩

axiom poseidon₃_collisionResistant : CollisionResistant poseidon₃
instance : Fact (CollisionResistant poseidon₃) := ⟨poseidon₃_collisionResistant⟩

axiom poseidon₂_nez : poseidon₂_no_zero_preimage
instance : Fact poseidon₂_no_zero_preimage := ⟨poseidon₂_nez⟩

namespace InclusionCircuit

theorem sound_and_complete
  {trees : List.Vector (MerkleTree F poseidon₂ 26) 8}
  {leaves : List.Vector F 8}:
    (∃ih p₁ p₂, LightProver.InclusionCircuit_8_8_8_26_8_8_26 ih (trees.map (·.root)) leaves p₁ p₂)
    ↔ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i]
  := by simp [InclusionCircuit_correct]

theorem inputHash_deterministic:
    LightProver.InclusionCircuit_8_8_8_26_8_8_26 h₁ trees leaves i₁ p₁ ∧
    LightProver.InclusionCircuit_8_8_8_26_8_8_26 h₂ trees leaves i₂ p₂ →
    h₁ = h₂ := by
  simp only [InclusionCircuit_rw]
  intros
  simp_all

theorem inputHash_injective:
    LightProver.InclusionCircuit_8_8_8_26_8_8_26 h trees₁ leaves₁ i₁ p₁ →
    LightProver.InclusionCircuit_8_8_8_26_8_8_26 h trees₂ leaves₂ i₂ p₂ →
    trees₁ = trees₂ ∧ leaves₁ = leaves₂ := by
  simp only [InclusionCircuit_rw]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  cases h₁
  exact inputHash_correct.mp h₂

end InclusionCircuit

namespace NonInclusionCircuit

theorem sound_and_complete
  {trees : List.Vector (RangeVector (2^26)) 8}
  {leaves : List.Vector F 8}:
    (∃ih p₁ p₂ p₃ p₄,
      LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 ih (trees.map (·.root)) leaves p₁ p₂ p₃ p₄)
    ↔ ∀i (_: i∈[0:8]), leaves[i].val ∈ trees[i]
  := by
    conv => lhs; arg 1; intro ih; rw [NonInclusionCircuit_correct]
    simp

theorem inputHash_deterministic:
    LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 h₁ trees leaves lo₁ hi₁ i₁ p₁ →
    LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 h₂ trees leaves lo₂ hi₂ i₂ p₂ →
    h₁ = h₂ := by
  unfold LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26
  simp_all [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8]

theorem inputHash_injective:
    LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 h trees₁ leaves₁ lo₁ hi₁ i₁ p₁ →
    LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 h trees₂ leaves₂ lo₂ hi₂ i₂ p₂ →
    trees₁ = trees₂ ∧ leaves₁ = leaves₂ := by
  unfold LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26
  simp only [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  cases h₁
  exact inputHash_correct.mp h₂

end NonInclusionCircuit

namespace CombinedCircuit

theorem sound_and_complete
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ 26) 8}
  {nonInclusionTrees : List.Vector (RangeVector (2^26)) 8}
  {inclusionLeaves nonInclusionLeaves : List.Vector F 8}:
    (∃ih p₁ p₂ p₃ p₄ p₅ p₆,
      LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8
        ih
        (inclusionTrees.map (·.root)) inclusionLeaves p₁ p₂
        (nonInclusionTrees.map (·.root)) nonInclusionLeaves p₃ p₄ p₅ p₆)
    ↔ ∀i (_: i∈[0:8]), inclusionLeaves[i] ∈ inclusionTrees[i]
                      ∧ nonInclusionLeaves[i].val ∈ nonInclusionTrees[i]
  := by
    conv => lhs; arg 1; intro ih; rw [CombinedCircuit_correct]
    simp

theorem inputHash_deterministic:
    LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8 h₁ inclusionTrees inclusionLeaves p₁ p₂ nonInclusionTrees nonInclusionLeaves p₃ p₄ p₅ p₆ →
    LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8 h₂ inclusionTrees inclusionLeaves q₁ q₂ nonInclusionTrees nonInclusionLeaves q₃ q₄ q₅ q₆ →
    h₁ = h₂ := by
  unfold LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8
  simp_all [TwoInputsHashChain_rw, LightProver.Gates, GatesGnark8]

theorem inputHash_injective:
    LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8 h inclusionTrees₁ inclusionLeaves₁ p₁ p₂ nonInclusionTrees₁ nonInclusionLeaves₁ p₃ p₄ p₅ p₆ →
    LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8 h inclusionTrees₂ inclusionLeaves₂ q₁ q₂ nonInclusionTrees₂ nonInclusionLeaves₂ q₃ q₄ q₅ q₆ →
    inclusionTrees₁ = inclusionTrees₂ ∧ inclusionLeaves₁ = inclusionLeaves₂ ∧ nonInclusionTrees₁ = nonInclusionTrees₂ ∧ nonInclusionLeaves₁ = nonInclusionLeaves₂ := by
  unfold LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8
  simp only [TwoInputsHashChain_rw, Poseidon2_iff_uniqueAssignment, LightProver.Gates, GatesGnark8, GatesDef.eq]
  rintro ⟨h₁, _⟩ ⟨h₂, _⟩
  rw [h₁] at h₂
  simp only [CollisionResistant_def, List.Vector.eq_cons, inputHash_correct] at h₂
  simp_all

end CombinedCircuit

def main : IO Unit := pure ()
