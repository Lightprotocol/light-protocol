import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Merkle

open LightProver (F)

theorem poseidon₂_testVector :
  poseidon₂ vec![1, 2] = 7853200120776062878684798364095072458815029376092732009249414926327459813530 := rfl

theorem poseidon₃_testVector :
  poseidon₃ vec![1, 2, 3] = 6542985608222806190361240322586112750744169038454362455181422643027100751666 := rfl

theorem InclusionCircuit.correct
  [Fact (CollisionResistant poseidon₂)]
  {trees : List.Vector (MerkleTree F poseidon₂ 26) 8}
  {leaves : List.Vector F 8}:
    (∃p₁ p₂, LightProver.InclusionCircuit_8_8_8_26_8_8_26 (trees.map (·.root)) leaves p₁ p₂)
    ↔ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i]
  := InclusionCircuit_correct

theorem NonInclusionCircuit.correct
  [Fact (CollisionResistant poseidon₃)]
  [Fact (CollisionResistant poseidon₂)]
  {trees : List.Vector (RangeTree 26) 8}
  {leaves : List.Vector F 8}:
    (∃p₁ p₂ p₃ p₄ p₅,
      LightProver.NonInclusionCircuit_8_8_8_8_8_8_26_8_8_26 (trees.map (·.val.root)) leaves p₁ p₂ p₃ p₄ p₅)
    ↔ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i]
  := NonInclusionCircuit_correct

theorem CombinedCircuit.correct
  [Fact (CollisionResistant poseidon₃)]
  [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ 26) 8}
  {nonInclusionTrees : List.Vector (RangeTree 26) 8}
  {inclusionLeaves nonInclusionLeaves : List.Vector F 8}:
    (∃p₁ p₂ p₃ p₄ p₅ p₆ p₇,
      LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_8_26_8
        (inclusionTrees.map (·.root)) inclusionLeaves p₁ p₂
        (nonInclusionTrees.map (·.val.root)) nonInclusionLeaves p₃ p₄ p₅ p₆ p₇)
    ↔ ∀i (_: i∈[0:8]), inclusionLeaves[i] ∈ inclusionTrees[i]
                      ∧ nonInclusionLeaves[i] ∈ nonInclusionTrees[i]
  := CombinedCircuit_correct

def main : IO Unit := pure ()
