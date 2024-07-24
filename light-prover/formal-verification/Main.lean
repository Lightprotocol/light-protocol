import ProvenZK
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
  {trees : Vector (MerkleTree F poseidon₂ 20) 10}
  {leaves : Vector F 10}:
    (∃p₁ p₂, LightProver.InclusionCircuit_10_10_10_20_10_10_20 (trees.map (·.root)) leaves p₁ p₂)
    ↔ ∀i (_: i∈[0:10]), leaves[i] ∈ trees[i]
  := InclusionCircuit_correct

theorem NonInclusionCircuit.correct
  [Fact (CollisionResistant poseidon₃)]
  [Fact (CollisionResistant poseidon₂)]
  {trees : Vector (RangeTree 20) 10}
  {leaves : Vector F 10}:
    (∃p₁ p₂ p₃ p₄ p₅,
      LightProver.NonInclusionCircuit_10_10_10_10_10_10_20_10_10_20 (trees.map (·.val.root)) leaves p₁ p₂ p₃ p₄ p₅)
    ↔ ∀i (_: i∈[0:10]), leaves[i] ∈ trees[i]
  := NonInclusionCircuit_correct

theorem CombinedCircuit.correct
  [Fact (CollisionResistant poseidon₃)]
  [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : Vector (MerkleTree F poseidon₂ 20) 10}
  {nonInclusionTrees : Vector (RangeTree 20) 10}
  {inclusionLeaves nonInclusionLeaves : Vector F 10}:
    (∃p₁ p₂ p₃ p₄ p₅ p₆ p₇,
      LightProver.CombinedCircuit_10_10_10_20_10_10_10_10_10_10_10_20_10
        (inclusionTrees.map (·.root)) inclusionLeaves p₁ p₂
        (nonInclusionTrees.map (·.val.root)) nonInclusionLeaves p₃ p₄ p₅ p₆ p₇)
    ↔ ∀i (_: i∈[0:10]), inclusionLeaves[i] ∈ inclusionTrees[i]
                      ∧ nonInclusionLeaves[i] ∈ nonInclusionTrees[i]
  := CombinedCircuit_correct

def main : IO Unit := pure ()
