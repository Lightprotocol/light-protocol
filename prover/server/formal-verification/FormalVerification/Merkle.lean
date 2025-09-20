import «ProvenZk»
import FormalVerification.Circuit
import FormalVerification.Lemmas
import FormalVerification.Rangecheck
import FormalVerification.Poseidon
import FormalVerification.RangeTree
import Mathlib

open LightProver (F Order Gates)
open LightProver renaming MerkleRootGadget_32_32_32 → StateMerkleRootGadget,
                          MerkleRootGadget_40_40_40 → AddressMerkleRootGadget,
                          InclusionProof_8_8_8_32_8_8_32 → InclusionProof,
                          TwoInputsHashChain_8_8 → TwoInputsHashChain_B_B,
                          HashChain_8 → HashChain_B,
                          InclusionCircuit_8_8_8_32_8_8_32 → InclusionCircuit,
                          NonInclusionProof_8_8_8_8_8_40_8_8_40 → NonInclusionProof,
                          NonInclusionCircuit_8_8_8_8_8_40_8_8_40 → NonInclusionCircuit,
                          CombinedCircuit_8_8_8_32_8_8_8_8_8_8_40_8 → CombinedCircuit,
                          MerkleRootUpdateGadget_32_32_32 → StateMerkleRootUpdateGadget,
                          MerkleRootUpdateGadget_40_40_40 → AddressMerkleRootUpdateGadget,
                          BatchAppendCircuit_8_8_32_8_32_8 → BatchAppendWithProofsCircuit,
                          BatchUpdateCircuit_8_8_8_32_8_8_32_8 → BatchUpdateCircuit,
                          BatchAddressTreeAppendCircuit_8_8_8_40_8_8_40_8_8_40 → BatchAddressTreeAppendCircuit

private abbrev SD := 32
private abbrev AD := 40
private abbrev B := 8

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
  cases d <;> simp [LightProver.ProveParentHash, Gates, GatesGnark12, GatesGnark9, GatesGnark8, hashLevel]

lemma MerkleTree.recover_succ' {ix : List.Vector Bool (Nat.succ N)} {proof : List.Vector F (Nat.succ N)} :
  MerkleTree.recover poseidon₂ ix proof item = hashLevel ix.head proof.head (MerkleTree.recover poseidon₂ ix.tail proof.tail item) := Eq.refl _

theorem StateMerkleRootGadget_rw {h : F} {i : List.Vector Bool SD} {p : List.Vector F SD} {k : F → Prop}:
    StateMerkleRootGadget h (i.map Bool.toZMod) p k ↔ k (MerkleTree.recover poseidon₂ i.reverse p.reverse h) := by
  unfold StateMerkleRootGadget
  simp only [List.Vector.getElem_map, ProveParentHash_rw]
  rw [←List.Vector.ofFn_get (v:=p), ←List.Vector.ofFn_get (v:=i)]
  rfl

set_option maxRecDepth 10000 in
theorem AddressMerkleRootGadget_rw {h : F} {i : List.Vector Bool AD} {p : List.Vector F AD} {k : F → Prop}:
    AddressMerkleRootGadget h (i.map Bool.toZMod) p k ↔ k (MerkleTree.recover poseidon₂ i.reverse p.reverse h) := by
  unfold AddressMerkleRootGadget
  simp only [List.Vector.getElem_map, ProveParentHash_rw]
  rw [←List.Vector.ofFn_get (v:=p), ←List.Vector.ofFn_get (v:=i)]
  rfl

theorem StateInclusionProofStep_rw {l i e r} {k : F → Prop}:
    (∃b, Gates.to_binary i SD b ∧ StateMerkleRootGadget l b e fun o => Gates.eq o r ∧ k o) ↔
    (∃ (hi : i.val < 2^SD), MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ e.reverse l = r) ∧ k r := by
  have : 2^SD < Order := by decide
  simp only [Gates, GatesGnark12, GatesDef.to_binary_12, GatesGnark8, GatesGnark9]
  simp only [←exists_and_right]
  rw [←exists_comm]
  simp only [exists_eq_left, StateMerkleRootGadget_rw, GatesDef.eq, MerkleTree.recoverAtFin, Fin.toBitsLE]
  apply Iff.intro
  · rintro ⟨_, _, _⟩
    simp_all
  · rintro ⟨_, _⟩
    simp_all

lemma InclusionProof_rw {roots leaves inPathIndices inPathElements k}:
  InclusionProof roots leaves inPathIndices inPathElements k ↔
  k roots ∧
  ∀i (_: i ∈ [0:B]), ∃ (hi : (inPathIndices[i]).val < 2^SD), MerkleTree.recoverAtFin poseidon₂ ⟨(inPathIndices[i]).val, hi⟩ (inPathElements[i]).reverse (leaves[i]) = roots[i] := by
  unfold InclusionProof
  simp_rw [StateInclusionProofStep_rw]
  apply Iff.intro
  . intro hp
    repeat rcases hp with ⟨_, hp⟩
    apply And.intro (by rw [←List.Vector.ofFn_get (v:=roots)]; exact hp)
    intro i ir
    have hir : i ∈ ([0:B].toList) := Std.Range.mem_toList_of_mem ir
    conv at hir => arg 1; simp [Std.Range.toList, Std.Range.toList.go]
    fin_cases hir <;> assumption
  . rintro ⟨hk, hp⟩
    repeat apply And.intro (by apply hp _ ⟨by decide, by decide⟩)
    rw [←List.Vector.ofFn_get (v:=roots)] at hk
    exact hk

theorem InclusionProof_correct [Fact (CollisionResistant poseidon₂)]  {trees : List.Vector (MerkleTree F poseidon₂ SD) B} {leaves : List.Vector F B}:
  (∃inPathIndices proofs, InclusionProof (trees.map (·.root)) leaves inPathIndices proofs k) ↔
  k (trees.map (·.root)) ∧ ∀i (_: i∈[0:B]), leaves[i] ∈ trees[i] := by
  simp [InclusionProof_rw, MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct]
  intro
  apply Iff.intro
  . rintro ⟨_, _, hp⟩ i ir
    have := hp i ir
    rcases this with ⟨h, _, hp⟩
    exact Exists.intro _ (Eq.symm hp)
  . intro hp
    have ⟨ind, indhp⟩ := Vector.exists_ofElems.mp fun (i : Fin B) => hp i.val ⟨by simp, And.intro i.prop (by simp [Nat.mod_one])⟩
    use ind.map fun i => (⟨i.val, Nat.lt_trans i.prop (by decide)⟩: F)
    use List.Vector.ofFn fun (i : Fin B) => (List.Vector.reverse $ trees[i.val].proofAtFin ind[i])
    intro i ir
    use by
      simp only [List.Vector.getElem_map, ZMod.val, Order]
      apply Fin.prop
    simp [getElem]
    apply And.intro
    . rfl
    . have := indhp i ir.2.1
      simp [getElem] at this
      rw [←this]
      congr

def inputHash (h₂ : Hash F 2) (h₃ : Hash F 3) (l r : List.Vector F (d + 1)) : F :=
  l.zipWith (·,·) r |>.tail |>.toList |>.foldl (fun h (l, r) => h₃ vec![h, l, r]) (h₂ vec![l.head, r.head])

lemma inputHash_next_correct {h₃ d} {l₁ l₂ : List.Vector (F × F) d} {a₁ a₂ : F} [Fact (CollisionResistant h₃)]:
    l₁.toList.foldl (fun h (l, r) => h₃ vec![h, l, r]) a₁ = l₂.toList.foldl (fun h (l, r) => h₃ vec![h, l, r]) a₂ ↔
    a₁ = a₂ ∧ l₁ = l₂ := by
  induction d generalizing a₁ a₂ with
  | zero =>
    cases l₁ using List.Vector.casesOn
    cases l₂ using List.Vector.casesOn
    simp
  | succ d ih =>
    cases l₁ using List.Vector.casesOn with | cons h₁ tl₁ =>
    cases l₂ using List.Vector.casesOn with | cons h₂ tl₂ =>
    cases h₁
    cases h₂
    simp [ih]
    cases tl₁
    cases tl₂
    apply Iff.intro
    · intro ⟨l, _⟩
      injections l
      simp_all
    · intro ⟨_, r⟩
      injections r
      simp_all

lemma List.Vector.zipWith_prod_eq_iff_inputs_eq {l₁ l₂ r₁ r₂ : Vector α d}: l₁.zipWith (·,·) l₂ = r₁.zipWith (·,·) r₂ ↔ l₁ = r₁ ∧ l₂ = r₂ := by
  simp only [zipWith]
  rw [Subtype.eq_iff]
  simp only
  induction d with
  | zero =>
    cases l₁ using List.Vector.casesOn
    cases l₂ using List.Vector.casesOn
    cases r₁ using List.Vector.casesOn
    cases r₂ using List.Vector.casesOn
    simp
  | succ d ih =>
    cases l₁ using List.Vector.casesOn with | cons h₁ tl₁ =>
    cases l₂ using List.Vector.casesOn with | cons h₂ tl₂ =>
    cases r₁ using List.Vector.casesOn with | cons h₃ tl₁ =>
    cases r₂ using List.Vector.casesOn with | cons h₄ tl₂ =>
    simp [ih]
    apply Iff.intro
    . intro ⟨_, h⟩
      simp_all
    . intro ⟨h₁, h₂⟩
      injections h₁
      simp_all [Vector, Subtype.eq_iff]

theorem inputHash_correct {d h₂ h₃} {l₁ r₁ l₂ r₂ : List.Vector F (d + 1)} [Fact (CollisionResistant h₂)] [Fact (CollisionResistant h₃)]:
    inputHash h₂ h₃ l₁ r₁ = inputHash h₂ h₃ l₂ r₂ ↔ l₁ = l₂ ∧ r₁ = r₂ := by
  cases l₁ using List.Vector.casesOn with | cons _ l₁ =>
  cases l₂ using List.Vector.casesOn with | cons _ l₂ =>
  cases r₁ using List.Vector.casesOn with | cons _ r₁ =>
  cases r₂ using List.Vector.casesOn with | cons _ r₂ =>
  simp only [inputHash, inputHash_next_correct]
  simp [List.Vector.zipWith_prod_eq_iff_inputs_eq]
  apply Iff.intro
  · intro ⟨h, _⟩
    injections h
    simp_all
  · intro ⟨h, _⟩
    injections h
    simp_all [List.Vector, Subtype.eq_iff]

lemma TwoInputsHashChain_rw {h₁ h₂: List.Vector F B} {k : F → Prop}:
    TwoInputsHashChain_B_B h₁ h₂ k ↔ k (inputHash poseidon₂ poseidon₃ h₁ h₂) := by
  unfold TwoInputsHashChain_B_B
  repeat cases h₁ using List.Vector.casesOn; rename_i _ h₁
  repeat cases h₂ using List.Vector.casesOn; rename_i _ h₂
  simp only [Poseidon3_iff_uniqueAssignment, Poseidon2_iff_uniqueAssignment, inputHash]
  simp only [List.Vector.zipWith_tail, List.Vector.zipWith_toList, List.Vector.toList_tail, List.Vector.toList_cons, List.tail, List.Vector.toList_nil, List.Vector.head_cons]
  apply Iff.of_eq
  rfl

theorem InclusionCircuit_rw:
    InclusionCircuit h roots leaves inPathIndices inPathElements ↔
    h = inputHash poseidon₂ poseidon₃ roots leaves ∧
    InclusionProof roots leaves inPathIndices inPathElements (fun _ => True) := by
  unfold InclusionCircuit
  simp only [TwoInputsHashChain_rw, Gates, GatesGnark8, GatesGnark9, GatesGnark12, GatesDef.eq]

theorem InclusionCircuit_correct [Fact (CollisionResistant poseidon₂)] {ih : F} {trees : List.Vector (MerkleTree F poseidon₂ SD) B} {leaves : List.Vector F B}:
  (∃inPathIndices proofs, InclusionCircuit ih (trees.map (·.root)) leaves inPathIndices proofs) ↔
   ih = (inputHash poseidon₂ poseidon₃ (trees.map (·.root)) leaves) ∧ ∀i (_: i∈[0:B]), leaves[i] ∈ trees[i] := by
  simp [InclusionCircuit_rw, InclusionProof_correct]

lemma LeafHashGadget_rw {r : Range} {v : F} {k : F → Prop}:
  LightProver.LeafHashGadget r.lo r.hi v k ↔ v.val ∈ r ∧ k r.hash := by
  unfold LightProver.LeafHashGadget
  simp only [Poseidon2_iff_uniqueAssignment]
  apply Iff.intro
  . rintro ⟨lo, hi, cont⟩
    apply And.intro _ cont
    have lo' := AssertIsLess_range (by
      rw [ZMod.val_natCast, Nat.mod_eq_of_lt]
      . exact Fin.prop _
      . exact Nat.lt_trans (Fin.prop _) (by decide)
    ) ⟨lo, hi⟩
    simp_rw [ZMod.val_natCast] at lo'
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
      have : (((2:F)^248).cast : ℤ) = 2^248 := by native_decide
      rw [this]
      rw [ZMod.cast_eq_val, ZMod.val_cast_of_lt]
      . rw [Int.emod_eq_of_lt]
        . linarith
        . linarith [r.hi.prop]
        . have : 2^248 + 2^248 < (Order : ℤ) := by native_decide
          linarith [r.lo.prop]
      . exact Nat.lt_trans r.lo.prop (by decide)
    . rw [AssertIsLess_248_semantics]
      zify
      zify at lo hi
      simp at lo hi
      simp [ZMod.castInt_add, ZMod.castInt_sub]
      have : (((2:F)^248).cast : ℤ) = 2^248 := by native_decide
      rw [this]
      rw [ZMod.cast_eq_val (r.hi.val : F), ZMod.val_cast_of_lt]
      . rw [Int.emod_eq_of_lt]
        . linarith
        . linarith [r.hi.prop]
        . have : 2^248 + 2^248 < (Order : ℤ) := by native_decide
          linarith [r.lo.prop]
      . exact Nat.lt_trans r.hi.prop (by decide)

theorem AddressInclusionProofStep_rw {l i e r} {k : F → Prop}:
    (∃b, Gates.to_binary i AD b ∧ AddressMerkleRootGadget l b e fun o => Gates.eq o r ∧ k o) ↔
    (∃ (hi : i.val < 2^AD), MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ e.reverse l = r) ∧ k r := by
  have : 2^AD < Order := by decide
  simp only [Gates, GatesGnark12, GatesDef.to_binary_12, GatesGnark8, GatesGnark9]
  simp only [←exists_and_right]
  rw [←exists_comm]
  simp only [exists_eq_left, AddressMerkleRootGadget_rw, GatesDef.eq, MerkleTree.recoverAtFin, Fin.toBitsLE]
  apply Iff.intro
  · rintro ⟨_, _, _⟩
    simp_all
  · rintro ⟨_, _⟩
    simp_all

theorem AddressMerkleRootGadget_eq_rw [Fact (CollisionResistant poseidon₂)] {h i : F} {p : List.Vector F AD} {tree : MerkleTree F poseidon₂ AD} {k : F → Prop}:
  (∃gate, Gates.to_binary i AD gate ∧ AddressMerkleRootGadget h gate p (fun r => Gates.eq r tree.root ∧ k r)) ↔ (∃(hi: i.val < 2^AD), h = tree.itemAtFin ⟨i.val, hi⟩ ∧ p.reverse = tree.proofAtFin ⟨i.val, hi⟩) ∧ k tree.root := by
  rw [AddressInclusionProofStep_rw]
  simp [and_comm]

lemma LeafHashGadget_hashing {p : F → Prop} : (LightProver.LeafHashGadget lo hi leaf p) → p (poseidon₂ vec![lo, hi]) := by
  simp [LightProver.LeafHashGadget]

theorem Range.hashOpt_eq_poseidon_iff_is_some {lo hi} [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)]:
    (Range.hashOpt r = poseidon₂ vec![lo, hi]) ↔ ∃(h:r.isSome), lo.val = (r.get h).lo.val ∧ hi.val = (r.get h).hi.val := by
  have : poseidon₂_no_zero_preimage := Fact.elim inferInstance
  unfold poseidon₂_no_zero_preimage at this
  apply Iff.intro
  · intro h
    cases r
    · simp only [hashOpt, Option.map, Option.getD] at h
      rw [eq_comm] at h
      have := this _ _ h
      cases this
    · simp only [hashOpt, Option.map, Option.getD, hash, CollisionResistant_def, List.Vector.eq_cons, and_true] at h
      cases h
      subst_vars
      simp
      rw [Nat.mod_eq_of_lt, Nat.mod_eq_of_lt]
      · simp
      · apply lt_trans (Fin.prop _) (by decide)
      · apply lt_trans (Fin.prop _) (by decide)
  · rintro ⟨h, hlo, hhi⟩
    cases r
    · cases h
    simp only [Option.get] at hlo hhi
    simp only [hashOpt, Range.hash, Option.map, Option.getD]
    congr
    · rw [←hlo]
      simp
    · rw [←hhi]
      simp

theorem MerkleTreeRoot_LeafHashGadget_rw [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {lo hi leaf ind proof} {k : F → Prop } {ranges : RangeVector (2^AD)}:
  (LightProver.LeafHashGadget lo hi leaf fun r =>
    ∃lv, Gates.to_binary ind AD lv ∧
    AddressMerkleRootGadget r lv proof fun root => Gates.eq root ranges.root ∧ k root)
  ↔ ∃(range : Range) (h: ind.val < 2^AD), ranges.ranges ⟨ind.val, h⟩ = range ∧ lo = range.lo ∧ hi = range.hi ∧ proof.reverse = (rangeTree ranges).proofAtFin ⟨ind.val, h⟩ ∧ leaf.val ∈ range ∧ k ranges.root := by
  apply Iff.intro
  . intro h
    simp only [AddressMerkleRootGadget_eq_rw, RangeVector.root, rangeTree, MerkleTree.ofFn_itemAtFin] at h
    unfold LightProver.LeafHashGadget at h
    simp only [Poseidon2_iff_uniqueAssignment] at h
    rcases h with ⟨lv, hv, ⟨ilt, hhash, hproof⟩, hk⟩
    rw [eq_comm, Range.hashOpt_eq_poseidon_iff_is_some] at hhash
    rcases hhash with ⟨hsome, hlo, hhi⟩
    exists (ranges.ranges ⟨ind.val, ilt⟩).get hsome
    exists ilt
    apply And.intro
    · simp
    apply And.intro
    · rw [←hlo]
      simp
    apply And.intro
    · rw [←hhi]
      simp
    apply And.intro
    · rw [hproof]
      simp [rangeTree]
    apply And.intro
    · have := AssertIsLess_range ?_ ⟨lv, hv⟩
      · rcases this with ⟨lo, hi⟩
        rw [hlo] at lo
        rw [hhi] at hi
        apply And.intro <;> assumption
      · rw [hlo]
        exact Fin.prop _
    · exact hk
  . rintro ⟨r, hind, hrget, rfl, rfl, hproof, hleaf, hk⟩
    simp only [RangeVector.root, rangeTree]
    simp only [RangeVector.root, rangeTree] at hk
    rw [LeafHashGadget_rw, AddressMerkleRootGadget_eq_rw]
    apply And.intro (by assumption)
    apply And.intro ?_ (by assumption)
    apply Exists.intro hind
    apply And.intro
    · rw [MerkleTree.ofFn_itemAtFin, hrget]
      rfl
    · assumption

def NonInclusionProof_rec {n : Nat} (lo hi leaf inds roots : List.Vector F n) (proofs : List.Vector (List.Vector F AD) n) (k : List.Vector F n → Prop): Prop :=
  match n with
  | 0 => k List.Vector.nil
  | _ + 1 => LightProver.LeafHashGadget lo.head hi.head leaf.head fun r =>
    ∃lv, Gates.to_binary inds.head AD lv ∧
    AddressMerkleRootGadget r lv proofs.head fun root =>
    Gates.eq root roots.head ∧ NonInclusionProof_rec lo.tail hi.tail leaf.tail inds.tail roots.tail proofs.tail fun rs => k (root ::ᵥ rs)

lemma NonInclusionProof_rec_equiv {lo hi leaf inds roots proofs k}:
  NonInclusionProof_rec lo hi leaf inds roots proofs k ↔
  NonInclusionProof roots leaf lo hi inds proofs k := by
  rw [ ←List.Vector.ofFn_get (v:=roots)
     , ←List.Vector.ofFn_get (v:=lo)
     , ←List.Vector.ofFn_get (v:=hi)
     , ←List.Vector.ofFn_get (v:=leaf)
     , ←List.Vector.ofFn_get (v:=inds)
     , ←List.Vector.ofFn_get (v:=proofs)
     ]
  rfl

theorem NonInclusionCircuit_rec_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {n : Nat} {trees : List.Vector (RangeVector (2^AD)) n} {leaves : List.Vector F n} {k : List.Vector F n → Prop}:
  (∃lo hi inds proofs, NonInclusionProof_rec lo hi leaves inds (trees.map (·.root)) proofs k) ↔
  k (trees.map (·.root)) ∧ ∀i (_: i∈[0:n]), leaves[i].val ∈ trees[i] := by
  unfold  AD at *
  induction n with
  | zero =>
    cases trees using List.Vector.casesOn
    simp [NonInclusionProof_rec]
    intro _ _ k
    linarith [k.2]
  | succ n ih =>
    apply Iff.intro
    . intro ⟨lo, hi, inds, proofs, hp⟩
      cases lo using List.Vector.casesOn with | cons hlo tlo =>
      cases hi using List.Vector.casesOn with | cons hhi thi =>
      cases leaves using List.Vector.casesOn with | cons hleaf tleaf =>
      cases inds using List.Vector.casesOn with | cons hinds tinds =>
      cases proofs using List.Vector.casesOn with | cons hproof tproof =>
      cases trees using List.Vector.casesOn with | cons htree ttree =>
      simp [NonInclusionProof_rec, MerkleTreeRoot_LeafHashGadget_rw] at hp
      rcases hp with ⟨range, _, hsome, ⟨_⟩, ⟨_⟩, hproof, hmem, hp⟩
      have := ih.mp $ Exists.intro _ $ Exists.intro _ $ Exists.intro _ $ Exists.intro _ hp
      rcases this with ⟨hl, hr⟩
      apply And.intro
      . simpa [*];
      . intro i ir
        cases i with
        | zero =>
          simp [Membership.mem]
          apply Exists.intro ⟨hinds.val, by assumption⟩
          apply Exists.intro range
          apply And.intro ?_ (eq_comm.mp hsome)
          simp only [Membership.mem] at hmem
          assumption
        | succ i =>
          rcases ir with ⟨l, r⟩
          simp
          exact hr i ⟨by simp, by simp [Nat.mod_one]; linarith⟩
    . intro ⟨hk, hmem⟩
      cases trees using List.Vector.casesOn with | cons htree ttree =>
      cases leaves using List.Vector.casesOn with | cons hleaf tleaf =>
      have := (ih (trees := ttree) (leaves := tleaf) (k := fun roots => k $ htree.root ::ᵥ roots)).mpr $ by
        simp at hk
        apply And.intro hk
        intro i ir
        have := hmem (i+1) ⟨by simp, by simp [Nat.mod_one]; linarith [ir.2]⟩
        simp at this
        exact this
      rcases this with ⟨lo, hi, inds, proofs, hp⟩
      have := hmem 0 ⟨by simp, by simp⟩
      simp at this
      simp [NonInclusionProof_rec, MerkleTreeRoot_LeafHashGadget_rw]
      rcases this with ⟨ix, r, hmem, hsome⟩
      use r.lo ::ᵥ lo
      use r.hi ::ᵥ hi
      use ix ::ᵥ inds
      use ((rangeTree htree).proofAtFin ix).reverse ::ᵥ proofs
      use r
      have : (ZMod.val (ix.val : F)) = ix.val := by
        rw [ZMod.val_natCast, Nat.mod_eq_of_lt]
        exact Nat.lt_trans ix.prop (by decide)
      apply Exists.intro
      simp [*]
      convert hp
      simp only [ZMod.val_natCast, List.Vector.head_cons]
      rw [Nat.mod_eq_of_lt]
      · exact Fin.prop _
      · apply Nat.lt_trans
        . exact ix.prop
        . decide

theorem NonInclusionCircuit_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {trees : List.Vector (RangeVector (2^AD)) B} {leaves : List.Vector F B}:
    (∃lo hi inds proofs, NonInclusionCircuit h (trees.map (·.root)) leaves lo hi inds proofs) ↔
    h = inputHash poseidon₂ poseidon₃ (trees.map (·.root)) leaves ∧ ∀i (_: i∈[0:B]), leaves[i].val ∈ trees[i] := by
  unfold NonInclusionCircuit
  simp only [←NonInclusionProof_rec_equiv]
  simp only [TwoInputsHashChain_rw]
  simp [Gates, GatesGnark8, GatesGnark9, GatesGnark12, GatesDef.eq]
  intro
  apply Iff.intro
  · rintro ⟨_, _, _, _, hp⟩
    apply And.right
    apply NonInclusionCircuit_rec_correct.mp
    repeat apply Exists.intro
    exact hp
  · intro hp
    apply NonInclusionCircuit_rec_correct.mpr
    simp only [true_and]
    exact hp

lemma InclusionProof_swap_ex {k : α → List.Vector F B → Prop} : (∃ a, InclusionProof x y z w fun r => k a r) ↔
  InclusionProof x y z w fun r => ∃a, k a r := by
  simp [InclusionProof_rw]

theorem CombinedCircuit_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ SD) B} { nonInclusionTrees : List.Vector (RangeVector (2^AD)) B}
  {inclusionLeaves nonInclusionLeaves : List.Vector F B}:
  (∃a b c e f g, CombinedCircuit h (inclusionTrees.map (·.root)) inclusionLeaves a b (nonInclusionTrees.map (·.root)) nonInclusionLeaves c e f g) ↔
  h = poseidon₂ vec![inputHash poseidon₂ poseidon₃ (inclusionTrees.map (·.root)) inclusionLeaves, inputHash poseidon₂ poseidon₃ (nonInclusionTrees.map (·.root)) nonInclusionLeaves] ∧
  ∀i (_: i∈[0:B]), inclusionLeaves[i] ∈ inclusionTrees[i] ∧ nonInclusionLeaves[i].val ∈ nonInclusionTrees[i] := by
  unfold CombinedCircuit B AD SD at *
  simp [InclusionProof_swap_ex, InclusionProof_correct, ←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct, TwoInputsHashChain_rw, Gates, GatesGnark8, GatesGnark9, GatesGnark12, GatesDef.eq]
  rintro _
  apply Iff.intro
  . rintro ⟨l, r⟩
    intro i hi
    apply And.intro (r i hi)
    have := NonInclusionCircuit_rec_correct.mp l
    exact this.2 i hi
  . intro hp
    apply And.intro
    . apply NonInclusionCircuit_rec_correct.mpr
      exact ⟨trivial, fun i ir => (hp i ir).2⟩
    . exact fun i ir => (hp i ir).1

theorem StateMerkleRootUpdateGadget_rw [Fact (CollisionResistant poseidon₂)] {tree : MerkleTree F poseidon₂ SD}:
    (∃bin, Gates.to_binary ix SD bin ∧
    StateMerkleRootUpdateGadget tree.root oldleaf newleaf bin proof k) ↔
    ∃(hi: ix.val < 2^SD), proof.reverse = tree.proofAtFin ⟨ix.val, hi⟩ ∧ oldleaf = tree.itemAtFin ⟨ix.val, hi⟩ ∧ k (tree.setAtFin ⟨ix.val, hi⟩ newleaf).root := by
  unfold StateMerkleRootUpdateGadget
  have : 2^SD < Order := by decide
  simp only [Gates, GatesGnark12, GatesDef.to_binary_12, GatesGnark8, GatesGnark9, GatesDef.eq]
  apply Iff.intro
  · rintro ⟨_, ⟨hi, rfl⟩, h⟩
    simp only [StateMerkleRootGadget_rw] at h
    simp only [MerkleTree.recover_eq_root_iff_proof_and_item_correct, Fin.toBitsLE, List.Vector.reverse_reverse] at h
    simp only [MerkleTree.proofAtFin, MerkleTree.root_setAtFin_eq_recoverAtFin, MerkleTree.recoverAtFin, MerkleTree.itemAtFin]
    rcases h with ⟨⟨h₁, h₂⟩, h₃⟩
    apply Exists.intro hi
    apply And.intro h₁
    apply And.intro h₂
    rw [←h₁]
    exact h₃
  · rintro ⟨hi, hpr, hol, hk⟩
    apply Exists.intro
    apply And.intro
    · exists hi
    simp only [StateMerkleRootGadget_rw, MerkleTree.recover_eq_root_iff_proof_and_item_correct, Fin.toBitsLE, List.Vector.reverse_reverse]
    simp only [MerkleTree.proofAtFin] at hpr
    simp only [MerkleTree.itemAtFin] at hol
    simp only [MerkleTree.root_setAtFin_eq_recoverAtFin, MerkleTree.recoverAtFin, MerkleTree.proofAtFin] at hk
    simp_all

def hashChain : List.Vector F (d + 1) → F := fun v =>
  v.tail.toList.foldl (fun h l => poseidon₂ vec![h, l]) v.head

lemma hashChain_body_inj [Fact (CollisionResistant poseidon₂)] {d : Nat} {a₁ a₂} {v₁ v₂ : List.Vector F d}:
  v₁.toList.foldl (fun h l => poseidon₂ vec![h, l]) a₁ = v₂.toList.foldl (fun h l => poseidon₂ vec![h, l]) a₂ ↔
  a₁ = a₂ ∧ v₁ = v₂ := by
  induction d generalizing a₁ a₂ with
  | zero =>
    cases v₁ using List.Vector.casesOn
    cases v₂ using List.Vector.casesOn
    simp
  | succ d ih =>
    cases v₁ using List.Vector.casesOn
    cases v₂ using List.Vector.casesOn
    simp [ih, List.Vector.eq_cons]
    tauto

theorem hashChain_injective [Fact (CollisionResistant poseidon₂)] {d:Nat} {v₁ v₂ : List.Vector F d.succ}:
  hashChain v₁ = hashChain v₂ ↔ v₁ = v₂ := by
  cases v₁ using List.Vector.casesOn
  cases v₂ using List.Vector.casesOn
  simp [hashChain, hashChain_body_inj, List.Vector.eq_cons]

theorem HashChain_4_rw : LightProver.HashChain_4 v k ↔ k (hashChain v) := by
  unfold LightProver.HashChain_4
  simp only [Poseidon2_iff_uniqueAssignment]
  rw [←List.Vector.ofFn_get (v:=v)]
  rfl

theorem HashChain_B_rw : HashChain_B v k ↔ k (hashChain v) := by
  unfold HashChain_B
  simp only [Poseidon2_iff_uniqueAssignment]
  rw [←List.Vector.ofFn_get (v:=v)]
  rfl

theorem HashChain_3_rw : LightProver.HashChain_3 v k ↔ k (hashChain v) := by
  unfold LightProver.HashChain_3
  simp only [Poseidon2_iff_uniqueAssignment]
  rw [←List.Vector.ofFn_get (v:=v)]
  rfl

def treeAppends : MerkleTree F poseidon₂ D → Nat → List F → Option (MerkleTree F poseidon₂ D)
| tree, _, [] => some tree
| tree, i, (v :: vs) => if h : i < 2^D then
  let tree' := tree.setAtFin ⟨i, h⟩ (if tree.itemAtFin ⟨i, h⟩ = 0 then v else tree.itemAtFin ⟨i, h⟩)
  treeAppends tree' (i+1) vs
else none

lemma iszero_rw (k : F → Prop) : (∃ gate_2,
        Gates.is_zero oleaf gate_2 ∧
          ∃ gate_3,
            Gates.select gate_2 leaf oleaf gate_3 ∧ k gate_3) ↔ k (if oleaf = 0 then leaf else oleaf) := by
  simp [Gates, GatesGnark8, GatesGnark12, GatesGnark9]

lemma ex_add_rw (k : F → Prop) : (∃g, g = Gates.add a b ∧ k g) ↔ k (a + b) := by
  simp [Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.add]

def AppendWithProofs_rec {D} (or si : F) (ol l : List.Vector F D) (mps : List.Vector (List.Vector F SD) D) (k : F → Prop): Prop := match D with
  | 0 => k or
  | _ + 1 =>
    ∃bin, Gates.to_binary si SD bin ∧
    StateMerkleRootUpdateGadget or ol.head (if ol.head = 0 then l.head else ol.head) bin mps.head fun or =>
    AppendWithProofs_rec or (si + 1) ol.tail l.tail mps.tail fun r => k r

lemma double_eq_ex {f : F → F} : (∃x, Gates.eq (f x) y ∧ Gates.eq z x ∧ k) ↔ y = (f z) ∧ k := by
  simp [Gates, GatesGnark8, GatesGnark12, GatesGnark9]
  apply Iff.intro
  · rintro ⟨x, _⟩
    simp_all
  · rintro ⟨x, y⟩
    simp_all

def AppendWithProofs' (pih : F) (or nr si : F) (ol l : List.Vector F B) (mps : List.Vector (List.Vector F SD) B): Prop :=
  pih = hashChain vec![or, nr, hashChain l, si] ∧
  AppendWithProofs_rec or si ol l mps fun r => r = nr

theorem AppendWithProofs_rw1 {pih} {or nr si ol l mps}:
    (∃lhh, BatchAppendWithProofsCircuit pih or nr lhh si ol l mps) ↔
    (AppendWithProofs' pih or nr si ol l mps) := by
  unfold BatchAppendWithProofsCircuit
  simp only [HashChain_4_rw, iszero_rw, HashChain_B_rw, ex_add_rw, double_eq_ex, AppendWithProofs']
  simp [AppendWithProofs_rec, add_assoc]
  rw [←List.Vector.ofFn_get (v:=mps), ←List.Vector.ofFn_get (v:=ol), ←List.Vector.ofFn_get (v:=l)]
  intro
  rfl

theorem AppendWithProofs_rec_rw {D} [Fact (CollisionResistant poseidon₂)] {tree : MerkleTree F poseidon₂ SD} {l : List.Vector F D}:
    (∃ol mps, AppendWithProofs_rec tree.root si ol l mps k) ↔ (∃newRoot, some newRoot = (treeAppends tree si.val l.toList).map (·.root) ∧ k newRoot) := by
  induction D generalizing tree si k with
  | zero =>
    cases l using List.Vector.casesOn
    simp [AppendWithProofs_rec, treeAppends]
  | succ D ih =>
    cases l using List.Vector.casesOn
    have sisucc : (si.val < 2^SD) → si.val + 1 = (si + 1).val := by
      intro h
      simp [ZMod.val_add]
      rw [Nat.mod_eq_of_lt]
      simp [ZMod.val_one]
      simp [ZMod.val_one]
      apply lt_trans
      apply Nat.add_lt_add_right
      exact h
      decide
    apply Iff.intro
    · rintro ⟨ol, mps, awp⟩
      simp only [AppendWithProofs_rec] at awp
      simp only [StateMerkleRootUpdateGadget_rw, List.Vector.tail_cons] at awp
      rcases awp with ⟨h, _, _, hp⟩
      have := ih.mp (Exists.intro ol.tail (Exists.intro mps.tail hp))
      rcases this with ⟨newRoot, hnewRoot, hk⟩
      exists newRoot
      apply And.intro ?_ hk
      rw [hnewRoot]
      simp only [List.Vector.toList_cons, treeAppends, h, dite_true]
      rename ol.head = _ => hol
      rw [hol, List.Vector.head_cons]
      congr
      rw [sisucc h]
    · rintro ⟨newRoot, hnewRoot, hk⟩
      simp only [List.Vector.toList_cons, treeAppends] at hnewRoot
      split at hnewRoot
      · rename_i h
        rw [sisucc h] at hnewRoot
        have := ih.mpr (Exists.intro _ ⟨hnewRoot, hk⟩)
        rcases this with ⟨ol, mps, hp⟩
        simp only [AppendWithProofs_rec, StateMerkleRootUpdateGadget_rw]
        exists (tree.itemAtFin ⟨si.val, h⟩ ::ᵥ ol)
        exists ((tree.proofAtFin ⟨si.val, h⟩ |>.reverse) ::ᵥ mps)
        apply Exists.intro h
        simp [hp]
      · cases hnewRoot

theorem AppendWithProofs_rw [Fact (CollisionResistant poseidon₂)]  {pih} {tree : MerkleTree F poseidon₂ SD} {si : F} {l} :
    (∃ ol mps lhh, BatchAppendWithProofsCircuit pih tree.root nr lhh si ol l mps)
    ↔ (pih = hashChain vec![tree.root, nr, hashChain l, si] ∧ some nr = (treeAppends tree si.val l.toList).map (·.root)) := by
  simp [AppendWithProofs_rw1, AppendWithProofs', AppendWithProofs_rec_rw]

theorem treeAppends_sound_and_complete {v : List.Vector F (d+1)} {tree newTree : MerkleTree F poseidon₂ D}:
    treeAppends tree startIndex v.toList = some newTree ↔
    (startIndex + d < 2^D) ∧
    ∀i: Fin (2^D),
      (i.val ∈ [startIndex:(startIndex + (d + 1))] → newTree[i] = if tree[i] = 0 then v[i.val - startIndex]! else tree[i]) ∧
      (i.val ∉ [startIndex:(startIndex + (d + 1))] → newTree[i] = tree[i]) := by
  induction d generalizing tree startIndex newTree with
  | zero =>
    cases v using List.Vector.casesOn with | cons h t =>
    cases t using List.Vector.casesOn
    simp [treeAppends]
    apply Iff.intro
    · rintro ⟨h, rfl⟩
      apply And.intro h
      intro i
      apply And.intro
      · intro h
        have : i.val = startIndex := by
          cases i
          simp_all [Membership.mem]
          linarith
        cases this
        simp [getElem!, decidableGetElem?]
      · intro h
        have : i.val ≠ startIndex := by
          cases i
          simp only at *
          intro h
          cases h
          simp_all [Membership.mem]
        rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq]
        · intro h
          cases h
          exact this rfl
    · rintro ⟨h₁, h₂⟩
      apply Exists.intro h₁
      rw [eq_comm]
      ext j
      by_cases hp : j = ⟨startIndex, h₁⟩
      · cases hp
        simp
        have := (h₂ ⟨startIndex, h₁⟩).1 ⟨by simp, by simp, by simp⟩
        simp [getElem!, decidableGetElem?] at this
        exact this
      · rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq hp]
        have := (h₂ j).2
        apply this
        intro hj
        have : j.val = startIndex := by
          cases hj
          linarith
        cases this
        exact hp rfl
  | succ d ih =>
    cases v using List.Vector.casesOn with | cons h t =>
    simp only [treeAppends, List.Vector.toList_cons]
    apply Iff.intro
    · intro h
      split at h
      · rename_i hr
        rw [ih] at h
        rcases h with ⟨hr, h⟩
        apply And.intro
        · linarith
        · intro i
          apply And.intro
          · by_cases hi : i.val = startIndex
            · intro _

              have := (h i).2
              simp [getElem, hi] at this
              simp [getElem, hi]
              apply this
              intro hp
              have := hp.1
              linarith
            · rintro ⟨hl, hh, _⟩
              simp only at *
              have : startIndex + 1 ≤ i.val := by
                cases i
                cases hl
                · contradiction
                · simp_all
              have := (h i).1
              have := this ⟨by linarith, by linarith, by simp [Nat.mod_one]⟩
              simp only [getElem] at this
              rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq] at this
              · simp only [getElem]
                rw [this]
                congr 1
                simp [getElem!, decidableGetElem?]
                have : i.val - (startIndex + 1) < d + 1 ↔ i.val - startIndex < d + 1 + 1 := by
                  zify [*]
                  apply Iff.intro <;> {intro; linarith}
                simp_rw [this]
                have : i.val - startIndex = i.val - (startIndex + 1) + 1 := by
                  zify [*]
                  ring
                simp_rw [this]
                simp
              · cases i
                intro h
                simp at h
                cases h
                apply hi
                simp
          · have := (h i).2
            intro hi
            simp only [getElem] at this
            rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq] at this
            apply this
            rintro ⟨h₁, h₂, _⟩
            apply hi
            apply And.intro
            · linarith
            · apply And.intro (by linarith) (by simp [Nat.mod_one])
            intro his
            simp at his
            cases his
            apply hi ⟨by linarith, by linarith, by simp [Nat.mod_one]⟩
      · cases h
    · rintro ⟨hsi, helt⟩
      have : startIndex < 2^D := by linarith
      simp only [this, dite_true, treeAppends, List.Vector.toList_cons, ih]
      apply And.intro (by linarith)
      intro i
      apply And.intro
      · intro hi
        rcases hi with ⟨hlo, hhi, _⟩
        have : i.val > startIndex := by linarith
        have : i.val ≠ startIndex := by linarith
        rw [(helt i).1]
        · simp only [getElem]
          rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq (Fin.ne_of_val_ne this)]
          have : i.val - startIndex = i.val - (startIndex + 1) + 1 := by
            zify [*]
            ring
          simp [this, getElem!, decidableGetElem?]
        · apply And.intro (by linarith)
          apply And.intro (by linarith)
          simp [Nat.mod_one]
      · intro hi
        by_cases hp : i.val = startIndex
        · simp only [getElem, hp, MerkleTree.itemAtFin_setAtFin_eq_self]
          simp only [getElem] at helt
          rw [(helt ⟨startIndex, this⟩).1]
          · simp [getElem!, decidableGetElem?]
          · apply And.intro (by linarith)
            apply And.intro (by linarith)
            simp [Nat.mod_one]
        · rw [(helt i).2]
          · simp only [getElem]
            rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq (Fin.ne_of_val_ne hp)]
          · intro hp
            rcases hp with ⟨hlo, hhi, _⟩
            cases i
            cases hlo
            · exact hp rfl
            · apply hi
              apply And.intro
              · simp_all
              · apply And.intro
                · linarith
                · simp [Nat.mod_one]

def hashChain3 (a b c : List.Vector F (d+1)) : F := hashChain (a.zipWith (·,·) b |>.zipWith (fun (a, b) c => poseidon₃ vec![a, b, c]) c)

lemma list_cons_eq : a :: b = c :: d ↔ a = c ∧ b = d := by
  simp

theorem hashChain3_body_injective [Fact (CollisionResistant poseidon₃)] {a₁ a₂ b₁ b₂ c₁ c₂ : List.Vector F d} :
    (a₁.zipWith (·,·) b₁ |>.zipWith (fun (a, b) c => poseidon₃ vec![a, b, c]) c₁) = (a₂.zipWith (·,·) b₂ |>.zipWith (fun (a, b) c => poseidon₃ vec![a, b, c]) c₂) ↔
    a₁ = a₂ ∧ b₁ = b₂ ∧ c₁ = c₂ := by
  induction d with
  | zero =>
    cases a₁ using List.Vector.casesOn
    cases b₁ using List.Vector.casesOn
    cases c₁ using List.Vector.casesOn
    cases a₂ using List.Vector.casesOn
    cases b₂ using List.Vector.casesOn
    cases c₂ using List.Vector.casesOn
    simp
  | succ d ih =>
    cases a₁ using List.Vector.casesOn
    cases b₁ using List.Vector.casesOn
    cases c₁ using List.Vector.casesOn
    cases a₂ using List.Vector.casesOn
    cases b₂ using List.Vector.casesOn
    cases c₂ using List.Vector.casesOn
    simp only [List.Vector.zipWith, List.Vector.eq_iff, List.Vector.toList] at ih
    simp [List.Vector.zipWith, List.zipWith, List.Vector.eq_iff, ih]
    tauto

theorem hashChain3_injective [Fact (CollisionResistant poseidon₂)] [Fact (CollisionResistant poseidon₃)] : hashChain3 a b c = hashChain3 a' b' c' ↔ a = a' ∧ b = b' ∧ c = c' := by
  simp [hashChain3, hashChain_injective, hashChain3_body_injective]

def batchUpdates {D l} (tree : MerkleTree F poseidon₂ D) (leafs indices txHashes : List.Vector F l): Option (MerkleTree F poseidon₂ D) :=
  match l with
  | 0 => tree
  | _ + 1 => if h : indices.head.val < 2^D then
    let tree' := tree.setAtFin ⟨indices.head.val, h⟩ (poseidon₃ vec![leafs.head, indices.head, txHashes.head])
    batchUpdates tree' leafs.tail indices.tail txHashes.tail
  else none

def batchUpdate_rec (root : F) (leaves oldLeaves indices txHashes : List.Vector F l) (proofs : List.Vector (List.Vector F SD) l) (k : F → Prop): Prop :=
  match l with
  | 0 => k root
  | _ + 1 =>
    ∃bin, Gates.to_binary indices.head SD bin ∧
    StateMerkleRootUpdateGadget root oldLeaves.head (poseidon₃ vec![leaves.head, indices.head, txHashes.head]) bin proofs.head fun root =>
    batchUpdate_rec root leaves.tail oldLeaves.tail indices.tail txHashes.tail proofs.tail k

theorem BatchUpdateCircuit_rw1 {pih or nr txh l ol mps} :
    (∃lhh, BatchUpdateCircuit pih or nr lhh txh l ol mps pis) ↔
    pih = hashChain vec![or, nr, hashChain3 l pis txh] ∧ batchUpdate_rec or l ol pis txh mps fun nr' => nr' = nr := by
  unfold BatchUpdateCircuit
  rw [←List.Vector.ofFn_get (v:=mps), ←List.Vector.ofFn_get (v:=ol), ←List.Vector.ofFn_get (v:=l), ←List.Vector.ofFn_get (v:=pis), ←List.Vector.ofFn_get (v:=txh)]
  simp only [batchUpdate_rec, HashChain_B_rw, Poseidon3_iff_uniqueAssignment, Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.eq, HashChain_3_rw]
  apply Iff.intro
  · rintro ⟨lhh, pihdef, lhhdef, h⟩
    cases pihdef
    cases lhhdef
    apply And.intro
    · rw [←List.Vector.ofFn_get (v:=txh), ←List.Vector.ofFn_get (v:=l), ←List.Vector.ofFn_get (v:=pis)]
      rfl
    · simp [getElem] at h
      rw [←List.Vector.ofFn_get (v:=txh), ←List.Vector.ofFn_get (v:=l), ←List.Vector.ofFn_get (v:=pis), ←List.Vector.ofFn_get (v:=mps), ←List.Vector.ofFn_get (v:=ol)]
      simp [-List.Vector.ofFn_get]
      exact h
  · rintro ⟨phdef, rest⟩
    use hashChain3 l pis txh
    rw [phdef]
    rw [←List.Vector.ofFn_get (v:=txh), ←List.Vector.ofFn_get (v:=l), ←List.Vector.ofFn_get (v:=pis), ←List.Vector.ofFn_get (v:=mps), ←List.Vector.ofFn_get (v:=ol)]
    simp [hashChain3, -List.Vector.ofFn_get]
    apply And.intro
    · rfl
    apply And.intro
    · rfl
    rw [←List.Vector.ofFn_get (v:=txh), ←List.Vector.ofFn_get (v:=l), ←List.Vector.ofFn_get (v:=pis), ←List.Vector.ofFn_get (v:=mps), ←List.Vector.ofFn_get (v:=ol)] at rest
    simp [-List.Vector.ofFn_get] at rest
    simp [getElem, rest]

theorem batchUpdate_rec_rw [Fact (CollisionResistant poseidon₂)] {l} {leaves txHashes indices : List.Vector F l} {tree : MerkleTree F poseidon₂ SD}:
    (∃proofs oldLeaves, batchUpdate_rec tree.root leaves oldLeaves indices txHashes proofs k) ↔
    ∃newTree, some newTree = (batchUpdates tree leaves indices txHashes) ∧ k newTree.root := by
  induction l generalizing tree with
  | zero =>
    simp [batchUpdate_rec, batchUpdates]
  | succ l ih =>
    simp [batchUpdate_rec, batchUpdates, StateMerkleRootUpdateGadget_rw]
    apply Iff.intro
    · rintro ⟨_, _, h, _, _, hbu⟩
      have := ih.mp (Exists.intro _ (Exists.intro _ hbu))
      simp_all
    · rintro ⟨newTree, ⟨hr, hnt⟩, hk⟩
      have := ih.mpr (Exists.intro _ ⟨hnt, hk⟩)
      rcases this with ⟨proofs, oldLeaves, h⟩
      simp_all
      use (tree.proofAtFin ⟨indices.head.val, hr⟩ |>.reverse) ::ᵥ proofs
      simp
      use (tree.itemAtFin ⟨indices.head.val, hr⟩ ::ᵥ oldLeaves)
      simp_all

theorem batchUpdates_sem_of_distinct {indices : List.Vector F l} (hdis: ∀(i j : Fin l), i ≠ j → indices[i] ≠ indices[j]):
    (some newTree = batchUpdates tree leaves indices txHashes) ↔
    ∃(hr : ∀ i, indices[i].val < 2^SD),
    (∀i: Fin l, newTree[indices[i].val]'(hr i) = poseidon₃ vec![leaves[i], indices[i], txHashes[i]]) ∧
    (∀i: Fin (2^SD), ↑i.val ∉ indices → newTree[i] = tree[i]) := by
  induction l generalizing tree newTree with
  | zero =>
    simp [batchUpdates, MerkleTree.ext_iff, getElem]
  | succ l ih =>
    simp [batchUpdates]
    have : ∀ (i j : Fin l), i ≠ j → indices.tail[i] ≠ indices.tail[j] := by
      intro i j hne heq
      have := hdis (Fin.succ i) (Fin.succ j) (by simp_all)
      rw [←List.Vector.cons_head_tail (v:=indices)] at this
      simp [-List.Vector.cons_head_tail] at this
      simp_all
    simp only [ih this]
    apply Iff.intro
    · rintro ⟨hdr, restr, hm, hnm⟩
      apply And.intro
      · apply Exists.intro
        · intro i
          cases i using Fin.cases
          · simp [getElem]
            simp [getElem] at hnm
            rw [hnm]
            · simp
            · simp
              intro hp
              rw [←List.Vector.toList_tail, List.Vector.mem_iff_get] at hp
              rcases hp with ⟨i, heq⟩
              apply hdis 0 i.succ (by simp [Fin.succ_ne_zero, eq_comm])
              rw [←List.Vector.cons_head_tail (v:=indices)]
              simp [-List.Vector.cons_head_tail]
              rw [←heq]
              rfl
          · conv => rhs; rw [←List.Vector.cons_head_tail (v:=indices), ←List.Vector.cons_head_tail (v:=leaves), ←List.Vector.cons_head_tail (v:=txHashes)]
            conv => rhs; simp [-List.Vector.cons_head_tail]
            conv => rhs; simp [getElem]
            simp [getElem] at hm
            rw [←hm]
            congr
        · intro i
          cases i using Fin.cases
          · simp [getElem, hdr]
          · rw [←List.Vector.cons_head_tail (v:=indices)]
            simp [-List.Vector.cons_head_tail]
            apply restr
      · intro i
        intro hi
        rw [List.Vector.mem_succ_iff] at hi
        simp only [not_or] at hi
        simp only [List.Vector.mem_def] at hnm
        cases hi with | _ hi =>
        have := hnm i (by assumption)
        simp [getElem] at this
        rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq] at this
        · exact this
        intro hp
        cases hp
        apply hi
        simp
    · rintro ⟨⟨hrch, hmem⟩, hnm⟩
      apply Exists.intro
      · apply Exists.intro
        · apply And.intro
          · intro i
            simp only [getElem, List.Vector.get_tail_succ, Fin.succ]
            have := hmem i.succ
            simp only [getElem, Fin.succ] at this
            exact this
          · intro i inemem
            by_cases h: i = indices.head
            · have : i.val = indices.head.val := by
                cases i
                rw [←h]
                simp
                rw [Nat.mod_eq_of_lt]
                apply lt_trans (by assumption) (by decide)
              simp [getElem, this]
              have := hmem 0
              simp [getElem] at this
              exact this
            · have := hnm i (by
                rw [List.Vector.mem_succ_iff]
                simp_all
              )
              simp only [getElem]
              rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq]
              exact this
              simp
              intro hh
              cases hh
              apply h
              simp
              have := hrch 0
              simp [getElem] at this
              exact this
        · intro i
          have := hrch i.succ
          simp only [getElem, List.Vector.get_tail_succ]
          apply this

lemma Range.none_of_hashOpt_zero {r: Option Range} [Fact poseidon₂_no_zero_preimage]: 0 = Range.hashOpt r ↔ r = none := by
  apply Iff.intro
  · intro h
    cases r
    · trivial
    · simp [Range.hashOpt, Option.map, Option.getD, hash] at h
      have : poseidon₂_no_zero_preimage := Fact.elim inferInstance
      unfold poseidon₂_no_zero_preimage at this
      have := this _ _ (Eq.symm h)
      cases this
  · rintro rfl; rfl

lemma MerkleTree.setAtFin_comm_of_ne {t : MerkleTree H α d} (hp : i₁ ≠ i₂) :
    (t.setAtFin i₁ v₁ |>.setAtFin i₂ v₂) = (t.setAtFin i₂ v₂ |>.setAtFin i₁ v₁) := by
  ext i
  by_cases h₁ : i = i₁
  · cases h₁
    simp [MerkleTree.itemAtFin_setAtFin_invariant_of_neq, hp]
  · by_cases h₂ : i = i₂
    · cases h₂
      simp [MerkleTree.itemAtFin_setAtFin_invariant_of_neq, hp, h₁]
    · simp [MerkleTree.itemAtFin_setAtFin_invariant_of_neq, h₁, h₂]

theorem AddressMerkleRootUpdateGadget_rw [Fact (CollisionResistant poseidon₂)] {tree : MerkleTree F poseidon₂ AD}:
    (∃bin, Gates.to_binary ix AD bin ∧
    AddressMerkleRootUpdateGadget tree.root oldleaf newleaf bin proof k) ↔
    ∃(hi: ix.val < 2^AD), proof.reverse = tree.proofAtFin ⟨ix.val, hi⟩ ∧ oldleaf = tree.itemAtFin ⟨ix.val, hi⟩ ∧ k (tree.setAtFin ⟨ix.val, hi⟩ newleaf).root := by
  unfold AddressMerkleRootUpdateGadget
  have : 2^AD < Order := by decide
  simp only [Gates, GatesGnark8, GatesDef.eq, GatesGnark12, GatesGnark9]
  apply Iff.intro
  · rintro ⟨_, ⟨hi, rfl⟩, h⟩
    simp only [AddressMerkleRootGadget_rw] at h
    simp only [MerkleTree.recover_eq_root_iff_proof_and_item_correct, Fin.toBitsLE, List.Vector.reverse_reverse] at h
    simp only [MerkleTree.proofAtFin, MerkleTree.root_setAtFin_eq_recoverAtFin, MerkleTree.recoverAtFin, MerkleTree.itemAtFin]
    rcases h with ⟨⟨h₁, h₂⟩, h₃⟩
    apply Exists.intro hi
    apply And.intro h₁
    apply And.intro h₂
    rw [←h₁]
    exact h₃
  · rintro ⟨hi, hpr, hol, hk⟩
    apply Exists.intro
    apply And.intro
    · exists hi
    simp only [AddressMerkleRootGadget_rw, MerkleTree.recover_eq_root_iff_proof_and_item_correct, Fin.toBitsLE, List.Vector.reverse_reverse]
    simp only [MerkleTree.proofAtFin] at hpr
    simp only [MerkleTree.itemAtFin] at hol
    simp only [MerkleTree.root_setAtFin_eq_recoverAtFin, MerkleTree.recoverAtFin, MerkleTree.proofAtFin] at hk
    simp_all

theorem BatchAddressAppend_step_complete [Fact (CollisionResistant poseidon₂)] {rv : RangeVector (2^AD)} {elt : F} {i st off : Nat} {k}
    (hlt : st + off < 2^AD)
    (hilt : i < 2^AD)
    (helt : elt.val ∈ rv.ranges ⟨i, hilt⟩)
    (hei : rv.ranges ⟨st + off, hlt⟩ = none)
    (hk : k (rv.remove elt.val ⟨i, hilt⟩ ⟨st + off, hlt⟩ helt hei).root):
    ∃LowElementValue LowElementNextValue LowElementProof LowElementIndices NewElementProof,
    LightProver.LeafHashGadget LowElementValue LowElementNextValue elt fun gate_0 =>
    LightProver.Poseidon2 LowElementValue elt fun gate_1 =>
    ∃gate_2, Gates.to_binary LowElementIndices AD gate_2 ∧
    AddressMerkleRootUpdateGadget rv.root gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
    LightProver.Poseidon2 elt LowElementNextValue fun gate_4 =>
    ∃gate_5, gate_5 = Gates.add st off ∧
    ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
    AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProof k := by
  have : ∃r, some r = rv.ranges ⟨i, hilt⟩ := by
    simp [Membership.mem] at helt
    split at helt <;> simp_all
  rcases this with ⟨r, hr⟩
  use r.lo, r.hi, ((rangeTree rv).proofAtFin ⟨i, hilt⟩).reverse, i, ((rangeTree rv |>.setAtFin ⟨i, hilt⟩ (poseidon₂ vec![r.lo, elt])).proofAtFin ⟨st + off, hlt⟩).reverse
  have : i % Order = i := by
    rw [Nat.mod_eq_of_lt]
    apply lt_trans (by assumption) (by decide)
  have : ((st : F) + (off : F)).val = st + off := by
    simp [ZMod.val_add]
    rw [Nat.mod_eq_of_lt]
    have : st + off < 2^AD + 2^AD := by linarith
    apply lt_trans this (by decide)
  have : Fin.mk i (by assumption) ≠ Fin.mk (st + off) (by assumption) := by
    intro h
    simp at h
    cases h
    rw [hei] at helt
    simp [Membership.mem] at helt
  simp [LeafHashGadget_rw, RangeVector.root, AddressMerkleRootUpdateGadget_rw, *]
  simp [Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.add, *]
  apply And.intro
  · rw [←hr] at helt
    exact helt
  · apply And.intro
    · simp [rangeTree, MerkleTree.ofFn_itemAtFin, ←hr]
      rfl
    · apply And.intro
      · rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq]
        simp [rangeTree, MerkleTree.ofFn_itemAtFin, hei]
        rfl
        simp_all [eq_comm]
      · simp [rangeTree, RangeVector.remove, RangeVector.root, ←hr] at hk
        rw [←hr] at helt
        simp [Membership.mem] at helt
        have : poseidon₂ vec![r.lo, elt] = Range.hashOpt (some (Range.mk r.lo ⟨elt.val, lt_trans helt.2 (Fin.prop _)⟩ helt.1)) := by simp [Range.hashOpt, Range.hash]
        simp [rangeTree]
        rw [this, ←MerkleTree.ofFn_cond]
        have : poseidon₂ vec![elt, r.hi] = Range.hashOpt (some (Range.mk ⟨elt.val, lt_trans helt.2 (Fin.prop _)⟩ r.hi helt.2)) := by simp [Range.hashOpt, Range.hash]
        rw [this, ←MerkleTree.ofFn_cond]
        convert hk using 3
        funext a
        by_cases h : a = ⟨st + off, by linarith⟩
        · cases h
          simp_all [eq_comm, Range.remove, Range.remove.rhi]
        · simp_all [Range.remove, Range.remove.rlo]


theorem BatchAddressAppend_step_rw [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^AD)}:
    (LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_0 =>
    LightProver.Poseidon2 LowElementValue NewElementValue fun gate_1 =>
    ∃gate_2, Gates.to_binary LowElementIndices AD gate_2 ∧
    AddressMerkleRootUpdateGadget rv.root gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
    LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_4 =>
    ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
    ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
    AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProof k) →
    ∃(hei : (StartIndex+offset).val < 2^AD)
     (hli : LowElementIndices.val < 2^AD)
     (currentIndex_valid : NewElementValue.val ∈ rv.ranges ⟨LowElementIndices.val, hli⟩)
     (emptyIndex_valid : rv.ranges ⟨(StartIndex+offset).val, hei⟩ = none),
     LowElementProof = ((rangeTree rv).proofAtFin ⟨LowElementIndices.val, hli⟩).reverse ∧
     NewElementProof = ((rangeTree rv |>.setAtFin ⟨LowElementIndices.val, hli⟩ (poseidon₂ vec![LowElementValue, NewElementValue])).proofAtFin ⟨(StartIndex+offset).val, hei⟩).reverse ∧
     k (rv.remove NewElementValue.val ⟨LowElementIndices.val, hli⟩ ⟨(StartIndex+offset).val, hei⟩ currentIndex_valid emptyIndex_valid).root := by
  simp only [LightProver.LeafHashGadget, Poseidon2_iff_uniqueAssignment, RangeVector.root, AddressMerkleRootUpdateGadget_rw]
  have := @Range.hashOpt_eq_poseidon_iff_is_some
  conv at this => enter [x,x,x,x,x,1]; rw [eq_comm]
  simp only [rangeTree, MerkleTree.ofFn_itemAtFin, this, Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.add]
  · rintro ⟨hlo, hhi, hli, hlep, ⟨hrsome, hlodef, hhidef⟩, _, rfl, hemp, hnep, heltZ, hk⟩
    apply Exists.intro hemp
    apply Exists.intro hli
    have := AssertIsLess_range (by rw [hlodef]; apply Fin.prop) ⟨hlo, hhi⟩
    rw [hlodef, hhidef] at this
    apply Exists.intro (by
      rw [Option.eq_some_of_isSome hrsome]
      exact this
    )
    have : Fin.mk LowElementIndices.val hli ≠ Fin.mk (StartIndex+offset).val hemp := by
      intro h
      rw [h, MerkleTree.itemAtFin_setAtFin_eq_self] at heltZ
      have : poseidon₂_no_zero_preimage := Fact.elim inferInstance
      exact this _ _ (Eq.symm heltZ)
    rw [MerkleTree.itemAtFin_setAtFin_invariant_of_neq (ne_comm.mp this), MerkleTree.ofFn_itemAtFin] at heltZ
    simp [Range.none_of_hashOpt_zero] at heltZ
    apply Exists.intro heltZ
    apply And.intro ?_ (And.intro ?_ ?last)
    case last =>
      convert hk
      simp only [RangeVector.remove, Range.remove, Range.remove.rlo, Range.remove.rhi]
      simp only [MerkleTree.ofFn_cond]
      rw [MerkleTree.setAtFin_comm_of_ne this]
      simp only [Range.hashOpt, Range.hash, Option.map, Option.getD, ←hlodef, ←hhidef]
      simp
    · simp_all [List.Vector.reverse_eq]
    · simp_all [List.Vector.reverse_eq]

def BatchAddressAppendStep (OldRoot LowElementValue LowElementNextValue NewElementValue LowElementIndices StartIndex offset : F) (LowElementProof  NewElementProof : List.Vector F AD) (k : F → Prop): Prop :=
  LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_0 =>
  LightProver.Poseidon2 LowElementValue NewElementValue fun gate_1 =>
  ∃gate_2, Gates.to_binary LowElementIndices AD gate_2 ∧
  AddressMerkleRootUpdateGadget OldRoot gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
  LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_4 =>
  ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
  ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
  AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProof k

lemma BatchAddressAppendStep_def :
  BatchAddressAppendStep OldRoot LowElementValue LowElementNextValue NewElementValue LowElementIndices StartIndex offset LowElementProof  NewElementProof k ↔
  LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_0 =>
  LightProver.Poseidon2 LowElementValue NewElementValue fun gate_1 =>
  ∃gate_2, Gates.to_binary LowElementIndices AD gate_2 ∧
  AddressMerkleRootUpdateGadget OldRoot gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
  LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_4 =>
  ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
  ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
  AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProof k := by rfl

theorem BatchAddressAppend_step_sound [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^AD)}:
    (LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_0 =>
    LightProver.Poseidon2 LowElementValue NewElementValue fun gate_1 =>
    ∃gate_2, Gates.to_binary LowElementIndices AD gate_2 ∧
    AddressMerkleRootUpdateGadget rv.root gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
    LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_4 =>
    ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
    ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
    AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProof k) →
    ∃(nrv : RangeVector (2^AD)), k (nrv.root) ∧ (NewElementValue.val ∉ nrv) ∧ (∀i, NewElementValue.val ≠ i → (i ∈ rv ↔ i ∈ nrv)) := by
  intro hp
  have hp := BatchAddressAppend_step_rw hp
  casesm* Exists _
  rename_i hp
  rcases hp with ⟨_, _, hp⟩
  apply Exists.intro
  apply And.intro hp
  apply And.intro
  · exact RangeVector.not_member_remove
  · intro i
    apply RangeVector.members_same

theorem exists_BatchAddressAppend_step_comm {k : α → F → Prop}:
  (∃x, BatchAddressAppendStep root lev lenv nev lei si o lep nep (k x)) ↔ BatchAddressAppendStep root lev lenv nev lei si o lep nep fun r => ∃x, k x r := by
  simp [BatchAddressAppendStep, LightProver.LeafHashGadget, AddressMerkleRootUpdateGadget]
  simp [Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.to_binary_12, GatesDef.add]
  intros
  apply Iff.intro
  · simp_all [AddressMerkleRootGadget_rw]
    intros
    apply Exists.intro
    assumption
  · simp_all [AddressMerkleRootGadget_rw]


def BatchAddressLoop {l} (OldRoot: F) (StartIndex: F) (offset : F) (LowElementValues: List.Vector F l) (LowElementNextValues: List.Vector F l) (LowElementIndices: List.Vector F l) (LowElementProofs: List.Vector (List.Vector F AD) l) (NewElementValues: List.Vector F l) (NewElementProofs: List.Vector (List.Vector F AD) l) (k : F → Prop): Prop :=
  match l with
  | 0 => k OldRoot
  | _ + 1 =>
    LightProver.LeafHashGadget LowElementValues.head LowElementNextValues.head NewElementValues.head fun gate_0 =>
    LightProver.Poseidon2 LowElementValues.head NewElementValues.head fun gate_1 =>
    ∃gate_2, Gates.to_binary LowElementIndices.head AD gate_2 ∧
    AddressMerkleRootUpdateGadget OldRoot gate_0 gate_1 gate_2 LowElementProofs.head fun gate_3 =>
    LightProver.Poseidon2 NewElementValues.head LowElementNextValues.head fun gate_4 =>
    ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
    ∃gate_6, Gates.to_binary gate_5 AD gate_6 ∧
    AddressMerkleRootUpdateGadget gate_3 (0:F) gate_4 gate_6 NewElementProofs.head fun OldRoot =>
    BatchAddressLoop OldRoot StartIndex (offset + 1) LowElementValues.tail LowElementNextValues.tail LowElementIndices.tail LowElementProofs.tail NewElementValues.tail NewElementProofs.tail k

theorem exists_BatchAddressLoop_comm [Fact (CollisionResistant poseidon₂)] {l}  {k : α → F → Prop} {si of lev lenv lei lep nev nep}:
    (∃x, BatchAddressLoop (l:=l) root si of lev lenv lei lep nev nep (k x)) ↔
    (BatchAddressLoop (l:=l) root si of lev lenv lei lep nev nep fun r => ∃x, k x r) := by
  induction l generalizing root si of with
  | zero => simp [BatchAddressLoop]
  | succ l ih =>
    simp only [BatchAddressLoop, ←BatchAddressAppendStep_def, exists_BatchAddressAppend_step_comm, ih]

theorem BatchAddressLoop_rw1 :
    BatchAddressTreeAppendCircuit pih oldRoot newRoot hch si lev lenv lei lep elements nep ↔
    BatchAddressLoop oldRoot si 0 lev lenv lei lep elements nep fun nr =>
      Gates.eq newRoot nr ∧
      LightProver.HashChain_8 elements fun gate_65 =>
      Gates.eq hch gate_65 ∧
      LightProver.HashChain_4 vec![oldRoot, newRoot, hch, si] fun gate_67 =>
      Gates.eq pih gate_67 ∧
      True := by
  unfold BatchAddressTreeAppendCircuit
  simp only [BatchAddressLoop]
  rw [←List.Vector.ofFn_get (v:=elements), ←List.Vector.ofFn_get (v:=lep), ←List.Vector.ofFn_get (v:=lenv), ←List.Vector.ofFn_get (v:=lev), ←List.Vector.ofFn_get (v:=lei), ←List.Vector.ofFn_get (v:=nep)]
  simp [-List.Vector.ofFn_get, getElem]
  rfl

theorem BatchAddressLoop_sound [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^AD)} {elements : List.Vector F l}:
    (∃ si offset lev lenv lei lep nep, BatchAddressLoop rv.root si offset lev lenv lei lep elements nep k) →
    ∃(nrv : RangeVector (2^AD)), k nrv.root ∧ (∀i, (i ∈ elements) → i.val ∉ nrv) ∧ (∀i, ↑i ∉ elements → (i ∈ rv ↔ i ∈ nrv)) := by
  induction l generalizing rv k with
  | zero =>
    simp [BatchAddressLoop]
    intro
    exists rv
    simp_all
  | succ l ih =>
    unfold BatchAddressLoop
    rintro ⟨_, _, _, _, _, _, _, hp⟩
    have := BatchAddressAppend_step_sound hp
    rcases this with ⟨nrv, hp₁, hp₂, hp₃⟩
    have := ih (elements := elements.tail) (rv := nrv) (k := k) $ by
      repeat apply Exists.intro
      exact hp₁
    rcases this with ⟨nrv, hp₄, hp₅, hp₆⟩
    exists nrv
    apply And.intro hp₄
    apply And.intro
    · intro i hpi
      simp at hpi
      rw [List.Vector.mem_succ_iff] at hpi
      cases hpi
      · by_cases i ∈ elements.tail
        · simp_all
        · simp_all
      · simp_all
    · intro i hpi
      rw [hp₃, hp₆]
      · simp_all [List.Vector.mem_succ_iff]
      · intro hp
        cases hp
        simp [List.Vector.mem_succ_iff] at hpi

theorem BatchAddressLoop_complete
    [Fact (CollisionResistant poseidon₂)]
    {l} {rv : RangeVector (2^AD)} {elements : List.Vector F l} {si off : Nat}
    (hk: ∀root, k root)
    (hdiff : ∀ i j, i ≠ j → elements.get i ≠ elements.get j)
    (hpresent : ∀ i, i ∈ elements → i.val ∈ rv)
    (hix : si + off + l < 2^AD)
    (hemps : ∀i, (h: i ∈ [si + off : si + off + l]) → rv.ranges ⟨i, by linarith [h.2]⟩ = none):
    ∃lev lenv lei lep nep, BatchAddressLoop rv.root si off lev lenv lei lep elements nep k := by
  induction l generalizing rv k si off with
  | zero =>
    simp [BatchAddressLoop]
    apply hk
  | succ l ih =>
    simp only [BatchAddressLoop]
    simp only [←BatchAddressAppendStep_def]
    simp only [List.Vector.exists_succ_iff_exists_cons (d:=l)]
    simp only [List.Vector.head_cons, List.Vector.tail_cons]
    convert_to (∃lev lenv lep lei nep levs lenvs leis leps neps, BatchAddressAppendStep rv.root lev lenv elements.head lei si off lep nep fun OldRoot =>
      BatchAddressLoop OldRoot si (off + 1) levs lenvs leis leps elements.tail neps k) using 0
    · simp
      apply Iff.intro
      · intro
        casesm* Exists _
        repeat apply Exists.intro
        assumption
      · intro
        casesm* Exists _
        repeat apply Exists.intro
        assumption
    simp only [exists_BatchAddressAppend_step_comm]
    have := hpresent elements.head (List.Vector.head_mem _)
    rcases this with ⟨head_ix, hhead⟩
    apply BatchAddressAppend_step_complete (i := head_ix) (hilt := head_ix.prop) (hlt := by linarith)
    case helt =>
      rcases hhead with ⟨r, hir, rirs⟩
      rw [←rirs]
      exact hir
    case hei =>
      apply hemps
      apply And.intro (by linarith) (And.intro (by linarith) (by simp [Nat.mod_one]))
    apply ih
    case hix =>
      have : off < Order := by
        apply lt_trans (b := 2^AD)
        · linarith
        · decide
      unfold Order at this
      simp [Nat.mod_eq_of_lt this]
      linarith
    · assumption
    · simp_all
    · intro i hi
      rw [←RangeVector.members_same]
      · apply hpresent
        simp [List.Vector.mem_succ_iff]
        simp at hi
        apply Or.inr hi
      · intro heq
        simp only [List.Vector.mem_def] at hi
        simp only [List.Vector.mem_iff_get, List.Vector.get_tail_succ] at hi
        rcases hi with ⟨_, hi⟩
        have heq := ZMod.eq_of_veq heq
        rw [←List.Vector.get_zero] at heq
        rw [←heq] at hi
        apply hdiff _ _ _ hi
        simp [Fin.succ_ne_zero]
    · have : off < Order := by
        apply lt_trans (b := 2^AD)
        · linarith
        · decide
      unfold Order at this
      simp [Nat.mod_eq_of_lt this]
      rintro i ⟨hil, hih, _⟩
      simp [RangeVector.remove]
      have := hemps i (And.intro (by linarith) (And.intro (by linarith) (by simp [Nat.mod_one])))
      split
      · rename_i h
        rcases hhead with ⟨_, _, heq⟩
        rw [←h, this] at heq
        cases heq
      · have : i ≠ si + off := by linarith
        simp only [this, ite_false]
        apply hemps i (And.intro (by linarith) (And.intro (by linarith) (by simp [Nat.mod_one])))

theorem BatchAddressLoop_skip_tree {elements : List.Vector F l}:
    BatchAddressLoop rv si off lev lenv lei lep elements nep k → ∃r, k r := by
  induction l generalizing rv si off with
  | zero =>
    simp [BatchAddressLoop]
    intro h
    use rv
  | succ l ih =>
    simp [BatchAddressLoop, LightProver.LeafHashGadget, Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.to_binary_12, AddressMerkleRootUpdateGadget]
    intros
    simp_all [AddressMerkleRootGadget_rw]
    casesm* Exists _, _ ∧ _
    simp_all [AddressMerkleRootGadget_rw]
    casesm* _∧_
    apply ih
    assumption

theorem BatchAdressAppend_sound [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^AD)}:
    (∃pih hch si lev lenv lei lep nep, BatchAddressTreeAppendCircuit pih rv.root newRoot hch si lev lenv lei lep elements nep) →
    ∃(nrv : RangeVector (2^AD)), nrv.root = newRoot ∧ (∀i (_: i ∈ elements), i.val ∉ nrv) ∧ (∀i, ↑i ∉ elements → (i ∈ rv ↔ i ∈ nrv)) := by
  simp only [BatchAddressLoop_rw1]
  intro h
  casesm* Exists _
  have := BatchAddressLoop_sound $ by
    repeat apply Exists.intro
    assumption
  simp only [HashChain_B_rw, HashChain_4_rw, Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.eq] at this
  rcases this with ⟨nrv, this⟩
  use nrv
  simp_all


theorem BatchAddressAppend_complete [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^AD)} {elements} {si : Nat}
    (si_small : si + B < 2^AD)
    (h_distinct : ∀ i j, i ≠ j → elements.get i ≠ elements.get j)
    (h_mems : ∀ i ∈ elements, i.val ∈ rv)
    (h_emps : ∀i, (h: i ∈ [si:si+B]) → rv.ranges ⟨i, by linarith [h.2]⟩ = none):
    ∃lev lenv lei lep nep newRoot hch pih, BatchAddressTreeAppendCircuit pih rv.root newRoot hch si lev lenv lei lep elements nep := by
  simp [BatchAddressLoop_rw1, exists_BatchAddressLoop_comm, HashChain_B_rw, HashChain_4_rw, Gates, GatesGnark8, GatesGnark12, GatesGnark9, GatesDef.eq]
  apply BatchAddressLoop_complete (hk := fun _ => trivial)
  · exact h_distinct
  · exact h_mems
  · exact h_emps
  · exact si_small
