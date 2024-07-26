import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Rangecheck
import FormalVerification.Poseidon
import FormalVerification.RangeTree
import Mathlib

open LightProver (F Order Gates)

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
  cases d <;> simp [LightProver.ProveParentHash, Gates, GatesGnark8, hashLevel]

lemma MerkleTree.recover_succ' {ix : Vector Bool (Nat.succ N)} {proof : Vector F (Nat.succ N)} :
  MerkleTree.recover poseidon₂ ix proof item = hashLevel ix.head proof.head (MerkleTree.recover poseidon₂ ix.tail proof.tail item) := Eq.refl _

theorem MerkleRootGadget_rw {h i : F} {p : Vector F 20} {k : F → Prop}:
  LightProver.MerkleRootGadget_20_20 h i p k ↔ ∃ (hi : i.val < 2^20), k (MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ p.reverse h) := by
  unfold LightProver.MerkleRootGadget_20_20
  simp_rw [Gates, GatesGnark8, Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt, ←exists_and_right]
  rw [exists_swap]
  apply exists_congr
  intro
  rw [←Vector.ofFn_get (v:=p)]
  simp [Vector.getElem_map, ProveParentHash_rw, MerkleTree.recoverAtFin, MerkleTree.recover_succ', Fin.toBitsLE, Fin.toBitsBE, -Vector.ofFn_get]
  rfl

lemma InclusionProofStep_rw {l i e r} {k : F → Prop}:
    (LightProver.MerkleRootGadget_20_20 l i e fun gate_0 => Gates.eq gate_0 r ∧ k gate_0) ↔
    (∃ (hi : i.val < 2^20), MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ e.reverse l = r) ∧ k r := by
  simp [MerkleRootGadget_rw]
  apply Iff.intro
  . rintro ⟨_, ⟨_⟩, _⟩; tauto
  . rintro ⟨⟨_, ⟨_⟩⟩⟩; tauto

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
    fin_cases hir <;> assumption
  . rintro ⟨hk, hp⟩
    repeat apply And.intro (by apply hp _ ⟨by decide, by decide⟩)
    rw [←Vector.ofFn_get (v:=roots)] at hk
    exact hk

theorem InclusionProof_correct [Fact (CollisionResistant poseidon₂)]  {trees : Vector (MerkleTree F poseidon₂ 20) 10} {leaves : Vector F 10}:
  (∃inPathIndices proofs, LightProver.InclusionProof_10_10_10_20_10_10_20 (trees.map (·.root)) leaves inPathIndices proofs k) ↔
  k (trees.map (·.root)) ∧ ∀i (_: i∈[0:10]), leaves[i] ∈ trees[i] := by
  simp [InclusionProof_rw, MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct]
  intro
  apply Iff.intro
  . rintro ⟨_, _, hp⟩ i ir
    have := hp i ir
    rcases this with ⟨h, _, hp⟩
    exact Exists.intro _ (Eq.symm hp)
  . intro hp
    have ⟨ind, indhp⟩  := Vector.exists_ofElems.mp fun (i : Fin 10) => hp i.val ⟨by simp, i.prop⟩
    use ind.map fun i => (⟨i.val, Nat.lt_trans i.prop (by decide)⟩: F)
    use Vector.ofFn fun (i : Fin 10) => (Vector.reverse $ trees[i.val].proofAtFin ind[i])
    intro i ir
    use by
      simp only [Vector.getElem_map, ZMod.val, Order]
      apply Fin.prop
    simp [getElem]
    apply And.intro
    . rfl
    . have := indhp i ir.2
      simp [getElem] at this
      rw [←this]
      congr

theorem InclusionCircuit_correct [Fact (CollisionResistant poseidon₂)] {trees : Vector (MerkleTree F poseidon₂ 20) 10} {leaves : Vector F 10}:
  (∃inPathIndices proofs, LightProver.InclusionCircuit_10_10_10_20_10_10_20 (trees.map (·.root)) leaves inPathIndices proofs) ↔
  ∀i (_: i∈[0:10]), leaves[i] ∈ trees[i] := by
  unfold LightProver.InclusionCircuit_10_10_10_20_10_10_20
  simp [InclusionProof_correct]

lemma LeafHashGadget_rw {r : Range} {v : F} {k : F → Prop}:
  LightProver.LeafHashGadget r.lo r.index r.hi v k ↔ v ∈ r ∧ k r.hash := by
  unfold LightProver.LeafHashGadget
  simp only [Poseidon3_iff_uniqueAssignment]
  apply Iff.intro
  . rintro ⟨lo, hi, cont⟩
    apply And.intro _ cont
    have lo' := AssertIsLess_range (by
      rw [ZMod.val_nat_cast, Nat.mod_eq_of_lt]
      . exact Fin.prop _
      . exact Nat.lt_trans (Fin.prop _) (by decide)
    ) ⟨lo, hi⟩
    simp_rw [ZMod.val_nat_cast] at lo'
    repeat rw [Nat.mod_eq_of_lt] at lo'
    . exact lo'
    . exact Nat.lt_trans r.hi.prop (by decide)
    . exact Nat.lt_trans r.lo.prop (by decide)
  . rintro ⟨⟨lo, hi⟩, cont⟩
    refine ⟨?_, ?_, cont⟩
    . rw [AssertIsLess_248_semantics]
      zify
      zify at lo hi
      simp at lo hi
      simp [ZMod.castInt_add, ZMod.castInt_sub]
      have : (((2:F)^248).cast : ℤ) = 2^248 := by rfl
      rw [this]
      rw [ZMod.cast_eq_val, ZMod.val_cast_of_lt]
      . rw [Int.emod_eq_of_lt]
        . linarith
        . linarith [r.hi.prop]
        . have : 2^248 + 2^248 < (Order : ℤ) := by decide
          linarith [r.lo.prop]
      . exact Nat.lt_trans r.lo.prop (by decide)
    . rw [AssertIsLess_248_semantics]
      zify
      zify at lo hi
      simp at lo hi
      simp [ZMod.castInt_add, ZMod.castInt_sub]
      have : (((2:F)^248).cast : ℤ) = 2^248 := by rfl
      rw [this]
      rw [ZMod.cast_eq_val (r.hi.val : F), ZMod.val_cast_of_lt]
      . rw [Int.emod_eq_of_lt]
        . linarith
        . linarith [r.hi.prop]
        . have : 2^248 + 2^248 < (Order : ℤ) := by decide
          linarith [r.lo.prop]
      . exact Nat.lt_trans r.hi.prop (by decide)

theorem MerkleRootGadget_eq_rw [Fact (CollisionResistant poseidon₂)] {h i : F} {p : Vector F 20} {tree : MerkleTree F poseidon₂ 20} {k : F → Prop}:
  LightProver.MerkleRootGadget_20_20 h i p (fun r => Gates.eq r tree.root ∧ k r) ↔ (∃(hi: i.val < 2^20), h = tree.itemAtFin ⟨i.val, hi⟩ ∧ p.reverse = tree.proofAtFin ⟨i.val, hi⟩) ∧ k tree.root := by
  simp [MerkleRootGadget_rw]
  rw [←exists_and_right]
  apply exists_congr
  simp [Gates, GatesGnark8, -MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct]
  intro i
  apply Iff.intro
  . intro ⟨l, r⟩
    rw [l] at r
    simp at l
    rcases l with ⟨_, l⟩
    simp [*]
  . intro ⟨l, r⟩
    have l' := l
    rw [And.comm, ←MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct] at l'
    rw [l']
    simp [*]

lemma LeafHashGadget_hashing {p : F → Prop} : (LightProver.LeafHashGadget lo nxt hi leaf p) → p (poseidon₃ vec![lo, nxt, hi]) := by
  simp [LightProver.LeafHashGadget]

lemma LeafHashGadget_in_tree [Fact (CollisionResistant poseidon₃)] {p : F → Prop} {tree : RangeTree 20} (p_in_tree : ∀ r, p r → ∃i, r = tree.val.itemAtFin i) :
  (LightProver.LeafHashGadget lo nxt hi leaf p) → ∃(r:Range), lo = r.lo ∧ hi = r.hi ∧ nxt = r.index := by
  intro h
  have := p_in_tree _ $ LeafHashGadget_hashing h
  rcases this with ⟨i, heq⟩
  rcases tree.prop i with ⟨r, h⟩
  rw [h] at heq
  simp [Range.hash, Vector.eq_cons] at heq
  apply Exists.intro r
  simp [heq]

theorem MerkleTreeRoot_LeafHashGadget_rw [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {lo hi nxt leaf ind proof} {k : F → Prop } {tree : RangeTree 20}:
  (LightProver.LeafHashGadget lo nxt hi leaf fun r =>
    LightProver.MerkleRootGadget_20_20 r ind proof fun root => Gates.eq root tree.val.root ∧ k root)
  ↔ ∃(range : Range) (h: ind.val < 2^20), tree.val.itemAtFin ⟨ind.val, h⟩ = range.hash ∧ lo = range.lo ∧ nxt = range.index ∧ hi = range.hi ∧ proof.reverse = tree.val.proofAtFin ⟨ind.val, h⟩ ∧ leaf ∈ range ∧ k tree.val.root := by
  apply Iff.intro
  . intro h
    simp only [MerkleRootGadget_eq_rw] at h
    have := LeafHashGadget_in_tree (tree := tree) (by
      simp
      intro r hp r_eq _ _
      apply Exists.intro ⟨ind.val, hp⟩
      exact r_eq
    ) h
    rcases this with ⟨r, ⟨_⟩, ⟨_⟩, ⟨_⟩⟩
    rw [LeafHashGadget_rw] at h
    rcases h with ⟨_, ⟨hlt, _, _⟩ , _⟩
    apply Exists.intro r
    apply Exists.intro hlt
    simp [*]
  . rintro ⟨r, h, _, ⟨_⟩, ⟨_⟩, ⟨_⟩, _, _, _⟩
    rw [LeafHashGadget_rw, MerkleRootGadget_eq_rw]
    simp [*]

def NonInclusionProof_rec {n : Nat} (lo nxt hi leaf inds roots : Vector F n) (proofs : Vector (Vector F 20) n) (k : Vector F n → Prop): Prop :=
  match n with
  | 0 => k Vector.nil
  | _ + 1 => LightProver.LeafHashGadget lo.head nxt.head hi.head leaf.head fun r =>
    LightProver.MerkleRootGadget_20_20 r inds.head proofs.head fun root =>
    Gates.eq root roots.head ∧ NonInclusionProof_rec lo.tail nxt.tail hi.tail leaf.tail inds.tail roots.tail proofs.tail fun rs => k (root ::ᵥ rs)

lemma NonInclusionProof_rec_equiv {lo nxt hi leaf inds roots proofs k}:
  NonInclusionProof_rec lo nxt hi leaf inds roots proofs k ↔
  LightProver.NonInclusionProof_10_10_10_10_10_10_20_10_10_20 roots leaf lo hi nxt inds proofs k := by
  rw [ ←Vector.ofFn_get (v:=roots)
     , ←Vector.ofFn_get (v:=lo)
     , ←Vector.ofFn_get (v:=nxt)
     , ←Vector.ofFn_get (v:=hi)
     , ←Vector.ofFn_get (v:=leaf)
     , ←Vector.ofFn_get (v:=inds)
     , ←Vector.ofFn_get (v:=proofs)
     ]
  rfl

theorem NonInclusionCircuit_rec_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {n : Nat} {trees : Vector (RangeTree 20) n} {leaves : Vector F n} {k : Vector F n → Prop}:
  (∃lo hi nxt inds proofs, NonInclusionProof_rec lo nxt hi leaves inds (trees.map (·.val.root)) proofs k) ↔
  k (trees.map (·.val.root)) ∧ ∀i (_: i∈[0:n]), leaves[i] ∈ trees[i] := by
  induction n with
  | zero =>
    cases trees using Vector.casesOn
    simp [NonInclusionProof_rec]
    intro _ _ k
    linarith [k.2]
  | succ n ih =>
    apply Iff.intro
    . intro ⟨lo, hi, nxt, inds, proofs, hp⟩
      cases lo using Vector.casesOn with | cons hlo tlo =>
      cases hi using Vector.casesOn with | cons hhi thi =>
      cases nxt using Vector.casesOn with | cons hnxt tnxt =>
      cases leaves using Vector.casesOn with | cons hleaf tleaf =>
      cases inds using Vector.casesOn with | cons hinds tinds =>
      cases proofs using Vector.casesOn with | cons hproof tproof =>
      cases trees using Vector.casesOn with | cons htree ttree =>
      simp [NonInclusionProof_rec, MerkleTreeRoot_LeafHashGadget_rw] at hp
      rcases hp with ⟨range, _, hinc, ⟨_⟩, ⟨_⟩, ⟨_⟩, _, hlr, hp⟩
      have := ih.mp $ Exists.intro _ $ Exists.intro _ $ Exists.intro _ $ Exists.intro _ $ Exists.intro _ hp
      apply And.intro
      . simp [*]
      . intro i ir
        cases i with
        | zero =>
          simp [Membership.mem, rangeTreeMem]
          simp [Membership.mem] at hlr
          apply Exists.intro range
          apply And.intro
          . exact Exists.intro _ hinc
          . assumption
        | succ i =>
          rcases ir with ⟨l, r⟩
          simp
          exact this.2 i ⟨by simp, by linarith⟩
    . intro ⟨hk, hmem⟩
      cases trees using Vector.casesOn with | cons htree ttree =>
      cases leaves using Vector.casesOn with | cons hleaf tleaf =>
      have := (ih (trees := ttree) (leaves := tleaf) (k := fun roots => k $ htree.val.root ::ᵥ roots)).mpr $ by
        simp at hk
        apply And.intro hk
        intro i ir
        have := hmem (i+1) ⟨by simp, by linarith [ir.2]⟩
        simp at this
        exact this
      rcases this with ⟨lo, hi, nxt, inds, proofs, hp⟩
      have := hmem 0 ⟨by simp, by simp⟩
      simp at this
      simp [NonInclusionProof_rec, MerkleTreeRoot_LeafHashGadget_rw]
      rcases this with ⟨r, ⟨ix, hitem⟩, hlo, hhi⟩
      use r.lo ::ᵥ lo
      use r.hi ::ᵥ hi
      use r.index ::ᵥ nxt
      use ix ::ᵥ inds
      use (htree.val.proofAtFin ix).reverse ::ᵥ proofs
      use r
      have : (ZMod.val (ix.val : F)) = ix.val := by
        rw [ZMod.val_nat_cast, Nat.mod_eq_of_lt]
        exact Nat.lt_trans ix.prop (by decide)
      apply Exists.intro
      simp [*, Membership.mem]
      exact hp
      simp [this]

theorem NonInclusionCircuit_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {trees : Vector (RangeTree 20) 10} {leaves : Vector F 10}:
  (∃lo hi nxt inds proofs, LightProver.NonInclusionCircuit_10_10_10_10_10_10_20_10_10_20 (trees.map (·.val.root)) leaves lo hi nxt inds proofs) ↔
  ∀i (_: i∈[0:10]), leaves[i] ∈ trees[i] := by
  unfold LightProver.NonInclusionCircuit_10_10_10_10_10_10_20_10_10_20
  simp [←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct, Gates, GatesGnark8]

lemma InclusionProof_swap_ex {k : α → Vector F 10 → Prop} : (∃ a, LightProver.InclusionProof_10_10_10_20_10_10_20 x y z w fun r => k a r) ↔
  LightProver.InclusionProof_10_10_10_20_10_10_20 x y z w fun r => ∃a, k a r := by
  simp [InclusionProof_rw]


theorem CombinedCircuit_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : Vector (MerkleTree F poseidon₂ 20) 10} { nonInclusionTrees : Vector (RangeTree 20) 10}
  {inclusionLeaves nonInclusionLeaves : Vector F 10}:
  (∃a b c d e f g, LightProver.CombinedCircuit_10_10_10_20_10_10_10_10_10_10_10_20_10 (inclusionTrees.map (·.root)) inclusionLeaves a b (nonInclusionTrees.map (·.val.root)) nonInclusionLeaves c d e f g) ↔
  ∀i (_: i∈[0:10]), inclusionLeaves[i] ∈ inclusionTrees[i] ∧ nonInclusionLeaves[i] ∈ nonInclusionTrees[i] := by
  unfold LightProver.CombinedCircuit_10_10_10_20_10_10_10_10_10_10_10_20_10
  simp [InclusionProof_swap_ex, InclusionProof_correct, ←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct]
  apply Iff.intro
  . tauto
  . intro hp
    apply And.intro
    . exact fun i ir => (hp i ir).2
    . exact fun i ir => (hp i ir).1
