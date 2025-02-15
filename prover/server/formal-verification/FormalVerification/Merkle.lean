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

lemma MerkleTree.recover_succ' {ix : List.Vector Bool (Nat.succ N)} {proof : List.Vector F (Nat.succ N)} :
  MerkleTree.recover poseidon₂ ix proof item = hashLevel ix.head proof.head (MerkleTree.recover poseidon₂ ix.tail proof.tail item) := Eq.refl _

theorem MerkleRootGadget_rw {h : F} {i : List.Vector Bool 26} {p : List.Vector F 26} {k : F → Prop}:
    LightProver.MerkleRootGadget_26_26_26 h (i.map Bool.toZMod) p k ↔ k (MerkleTree.recover poseidon₂ i.reverse p.reverse h) := by
  unfold LightProver.MerkleRootGadget_26_26_26
  simp only [List.Vector.getElem_map, ProveParentHash_rw]
  rw [←List.Vector.ofFn_get (v:=p), ←List.Vector.ofFn_get (v:=i)]
  rfl

theorem InclusionProofStep_rw {l i e r} {k : F → Prop}:
    (∃b, Gates.to_binary i 26 b ∧ LightProver.MerkleRootGadget_26_26_26 l b e fun o => Gates.eq o r ∧ k o) ↔
    (∃ (hi : i.val < 2^26), MerkleTree.recoverAtFin poseidon₂ ⟨i.val, hi⟩ e.reverse l = r) ∧ k r := by
  have : 2^26 < Order := by decide
  simp only [Gates, GatesGnark8, Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt this]
  simp only [←exists_and_right]
  rw [←exists_comm]
  simp only [exists_eq_left, MerkleRootGadget_rw, GatesDef.eq, MerkleTree.recoverAtFin, Fin.toBitsLE]
  apply Iff.intro
  · rintro ⟨_, _, _⟩
    simp_all
    tauto
  · rintro ⟨_, _⟩
    simp_all
    tauto

lemma InclusionProof_rw {roots leaves inPathIndices inPathElements k}:
  LightProver.InclusionProof_8_8_8_26_8_8_26 roots leaves inPathIndices inPathElements k ↔
  k roots ∧
  ∀i (_: i ∈ [0:8]), ∃ (hi : (inPathIndices[i]).val < 2^26), MerkleTree.recoverAtFin poseidon₂ ⟨(inPathIndices[i]).val, hi⟩ (inPathElements[i]).reverse (leaves[i]) = roots[i] := by
  unfold LightProver.InclusionProof_8_8_8_26_8_8_26
  simp_rw [InclusionProofStep_rw]
  apply Iff.intro
  . intro hp
    repeat rcases hp with ⟨_, hp⟩
    apply And.intro (by rw [←List.Vector.ofFn_get (v:=roots)]; exact hp)
    intro i ir
    have hir : i ∈ ([0:8].toList) := Std.Range.mem_toList_of_mem ir
    conv at hir => arg 1; simp [Std.Range.toList, Std.Range.toList.go]
    fin_cases hir <;> assumption
  . rintro ⟨hk, hp⟩
    repeat apply And.intro (by apply hp _ ⟨by decide, by decide⟩)
    rw [←List.Vector.ofFn_get (v:=roots)] at hk
    exact hk

theorem InclusionProof_correct [Fact (CollisionResistant poseidon₂)]  {trees : List.Vector (MerkleTree F poseidon₂ 26) 8} {leaves : List.Vector F 8}:
  (∃inPathIndices proofs, LightProver.InclusionProof_8_8_8_26_8_8_26 (trees.map (·.root)) leaves inPathIndices proofs k) ↔
  k (trees.map (·.root)) ∧ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i] := by
  simp [InclusionProof_rw, MerkleTree.recoverAtFin_eq_root_iff_proof_and_item_correct]
  intro
  apply Iff.intro
  . rintro ⟨_, _, hp⟩ i ir
    have := hp i ir
    rcases this with ⟨h, _, hp⟩
    exact Exists.intro _ (Eq.symm hp)
  . intro hp
    have ⟨ind, indhp⟩ := Vector.exists_ofElems.mp fun (i : Fin 8) => hp i.val ⟨by simp, And.intro i.prop (by simp [Nat.mod_one])⟩
    use ind.map fun i => (⟨i.val, Nat.lt_trans i.prop (by decide)⟩: F)
    use List.Vector.ofFn fun (i : Fin 8) => (List.Vector.reverse $ trees[i.val].proofAtFin ind[i])
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

lemma TwoInputsHashChain_rw {h₁ h₂: List.Vector F 8} {k : F → Prop}:
    LightProver.TwoInputsHashChain_8_8 h₁ h₂ k ↔ k (inputHash poseidon₂ poseidon₃ h₁ h₂) := by
  unfold LightProver.TwoInputsHashChain_8_8
  repeat cases h₁ using List.Vector.casesOn; rename_i _ h₁
  repeat cases h₂ using List.Vector.casesOn; rename_i _ h₂
  simp only [Poseidon3_iff_uniqueAssignment, Poseidon2_iff_uniqueAssignment, inputHash]
  simp only [List.Vector.zipWith_tail, List.Vector.zipWith_toList, List.Vector.toList_tail, List.Vector.toList_cons, List.tail, List.Vector.toList_nil, List.Vector.head_cons]
  apply Iff.of_eq
  rfl

theorem InclusionCircuit_rw:
    LightProver.InclusionCircuit_8_8_8_26_8_8_26 h roots leaves inPathIndices inPathElements ↔
    h = inputHash poseidon₂ poseidon₃ roots leaves ∧
    LightProver.InclusionProof_8_8_8_26_8_8_26 roots leaves inPathIndices inPathElements (fun _ => True) := by
  unfold LightProver.InclusionCircuit_8_8_8_26_8_8_26
  simp only [TwoInputsHashChain_rw, Gates, GatesGnark8, GatesDef.eq]

theorem InclusionCircuit_correct [Fact (CollisionResistant poseidon₂)] {ih : F} {trees : List.Vector (MerkleTree F poseidon₂ 26) 8} {leaves : List.Vector F 8}:
  (∃inPathIndices proofs, LightProver.InclusionCircuit_8_8_8_26_8_8_26 ih (trees.map (·.root)) leaves inPathIndices proofs) ↔
   ih = (inputHash poseidon₂ poseidon₃ (trees.map (·.root)) leaves) ∧ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i] := by
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

theorem MerkleRootGadget_eq_rw [Fact (CollisionResistant poseidon₂)] {h i : F} {p : List.Vector F 26} {tree : MerkleTree F poseidon₂ 26} {k : F → Prop}:
  (∃gate, Gates.to_binary i 26 gate ∧ LightProver.MerkleRootGadget_26_26_26 h gate p (fun r => Gates.eq r tree.root ∧ k r)) ↔ (∃(hi: i.val < 2^26), h = tree.itemAtFin ⟨i.val, hi⟩ ∧ p.reverse = tree.proofAtFin ⟨i.val, hi⟩) ∧ k tree.root := by
  rw [InclusionProofStep_rw]
  simp [and_comm]

lemma LeafHashGadget_hashing {p : F → Prop} : (LightProver.LeafHashGadget lo hi leaf p) → p (poseidon₂ vec![lo, hi]) := by
  simp [LightProver.LeafHashGadget]

-- lemma LeafHashGadget_in_tree [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {p : F → Prop} {tree : RangeVector (2^26)}
--     (p_in_tree : ∀ r, p r → ∃i, r = (Range.hashOpt <| tree.ranges i)) :
--     (LightProver.LeafHashGadget lo hi leaf p) → ∃(r:Range), lo = r.lo ∧ hi = r.hi := by
--   intro h
--   have := p_in_tree _ $ LeafHashGadget_hashing h
--   rcases this with ⟨i, heq⟩
--   sorry

-- lemma LeafHashGadget_in_tree' [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {p : F → Prop}
--     (h : LightProver.LeafHashGadget lo hi leaf p)
--     (hp : ∀ r, p r → ∃ rg, r = Range.hashOpt rg) :
--     lo.val < 2^248 ∧ hi.val < 2^248 ∧ lo.val < leaf.val ∧ leaf.val < hi.val := by
--   have := hp _ $ LeafHashGadget_hashing h
--   rcases this with ⟨rg, this⟩
--   cases rg
--   · simp only [Range.hashOpt, Option.map, Option.getD] at this
--     have ne : poseidon₂_no_zero_preimage := Fact.elim inferInstance
--     unfold poseidon₂_no_zero_preimage at ne
--     have := ne _ _ this
--     exfalso
--     assumption
--   · rename_i rg
--     cases rg
--     simp only [Range.hashOpt, Option.map, Option.getD, Range.hash, CollisionResistant_def, List.Vector.eq_cons, and_true] at this
--     cases this
--     subst_vars
--     simp

-- lemma LeafHashGadget_rw' : (LightProver.LeafHashGadget lo hi v k) ↔ (∃(r:Range), lo.val = r.lo.val ∧ hi.val = r.hi.val ∧ v.val ∈ r) := by
--   apply Iff.intro

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

theorem MerkleTreeRoot_LeafHashGadget_rw [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {lo hi leaf ind proof} {k : F → Prop } {ranges : RangeVector (2^26)}:
  (LightProver.LeafHashGadget lo hi leaf fun r =>
    ∃lv, Gates.to_binary ind 26 lv ∧
    LightProver.MerkleRootGadget_26_26_26 r lv proof fun root => Gates.eq root ranges.root ∧ k root)
  ↔ ∃(range : Range) (h: ind.val < 2^26), ranges.ranges ⟨ind.val, h⟩ = range ∧ lo = range.lo ∧ hi = range.hi ∧ proof.reverse = (rangeTree ranges).proofAtFin ⟨ind.val, h⟩ ∧ leaf.val ∈ range ∧ k ranges.root := by
  apply Iff.intro
  . intro h
    simp only [MerkleRootGadget_eq_rw, RangeVector.root, rangeTree, MerkleTree.ofFn_itemAtFin] at h
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
    rw [LeafHashGadget_rw, MerkleRootGadget_eq_rw]
    apply And.intro (by assumption)
    apply And.intro ?_ (by assumption)
    apply Exists.intro hind
    apply And.intro
    · rw [MerkleTree.ofFn_itemAtFin, hrget]
      rfl
    · assumption

def NonInclusionProof_rec {n : Nat} (lo hi leaf inds roots : List.Vector F n) (proofs : List.Vector (List.Vector F 26) n) (k : List.Vector F n → Prop): Prop :=
  match n with
  | 0 => k List.Vector.nil
  | _ + 1 => LightProver.LeafHashGadget lo.head hi.head leaf.head fun r =>
    ∃lv, Gates.to_binary inds.head 26 lv ∧
    LightProver.MerkleRootGadget_26_26_26 r lv proofs.head fun root =>
    Gates.eq root roots.head ∧ NonInclusionProof_rec lo.tail hi.tail leaf.tail inds.tail roots.tail proofs.tail fun rs => k (root ::ᵥ rs)

lemma NonInclusionProof_rec_equiv {lo hi leaf inds roots proofs k}:
  NonInclusionProof_rec lo hi leaf inds roots proofs k ↔
  LightProver.NonInclusionProof_8_8_8_8_8_26_8_8_26 roots leaf lo hi inds proofs k := by
  rw [ ←List.Vector.ofFn_get (v:=roots)
     , ←List.Vector.ofFn_get (v:=lo)
     , ←List.Vector.ofFn_get (v:=hi)
     , ←List.Vector.ofFn_get (v:=leaf)
     , ←List.Vector.ofFn_get (v:=inds)
     , ←List.Vector.ofFn_get (v:=proofs)
     ]
  rfl

theorem NonInclusionCircuit_rec_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {n : Nat} {trees : List.Vector (RangeVector (2^26)) n} {leaves : List.Vector F n} {k : List.Vector F n → Prop}:
  (∃lo hi inds proofs, NonInclusionProof_rec lo hi leaves inds (trees.map (·.root)) proofs k) ↔
  k (trees.map (·.root)) ∧ ∀i (_: i∈[0:n]), leaves[i].val ∈ trees[i] := by
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

theorem NonInclusionCircuit_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)] {trees : List.Vector (RangeVector (2^26)) 8} {leaves : List.Vector F 8}:
    (∃lo hi inds proofs, LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26 h (trees.map (·.root)) leaves lo hi inds proofs) ↔
    h = inputHash poseidon₂ poseidon₃ (trees.map (·.root)) leaves ∧ ∀i (_: i∈[0:8]), leaves[i].val ∈ trees[i] := by
  unfold LightProver.NonInclusionCircuit_8_8_8_8_8_26_8_8_26
  simp only [←NonInclusionProof_rec_equiv]
  simp only [TwoInputsHashChain_rw]
  simp [Gates, GatesGnark8, GatesDef.eq]
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

lemma InclusionProof_swap_ex {k : α → List.Vector F 8 → Prop} : (∃ a, LightProver.InclusionProof_8_8_8_26_8_8_26 x y z w fun r => k a r) ↔
  LightProver.InclusionProof_8_8_8_26_8_8_26 x y z w fun r => ∃a, k a r := by
  simp [InclusionProof_rw]

theorem CombinedCircuit_correct [Fact poseidon₂_no_zero_preimage] [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ 26) 8} { nonInclusionTrees : List.Vector (RangeVector (2^26)) 8}
  {inclusionLeaves nonInclusionLeaves : List.Vector F 8}:
  (∃a b c e f g, LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8 h (inclusionTrees.map (·.root)) inclusionLeaves a b (nonInclusionTrees.map (·.root)) nonInclusionLeaves c e f g) ↔
  h = poseidon₂ vec![inputHash poseidon₂ poseidon₃ (inclusionTrees.map (·.root)) inclusionLeaves, inputHash poseidon₂ poseidon₃ (nonInclusionTrees.map (·.root)) nonInclusionLeaves] ∧
  ∀i (_: i∈[0:8]), inclusionLeaves[i] ∈ inclusionTrees[i] ∧ nonInclusionLeaves[i].val ∈ nonInclusionTrees[i] := by
  unfold LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_26_8
  simp [InclusionProof_swap_ex, InclusionProof_correct, ←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct, TwoInputsHashChain_rw, Gates, GatesGnark8, GatesDef.eq]
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
      simp_all
    . exact fun i ir => (hp i ir).1

theorem MerkleRootUpdateGadget_rw [Fact (CollisionResistant poseidon₂)] {tree : MerkleTree F poseidon₂ 26}:
    (∃bin, Gates.to_binary ix 26 bin ∧
    LightProver.MerkleRootUpdateGadget_26_26_26 tree.root oldleaf newleaf bin proof k) ↔
    ∃(hi: ix.val < 2^26), proof.reverse = tree.proofAtFin ⟨ix.val, hi⟩ ∧ oldleaf = tree.itemAtFin ⟨ix.val, hi⟩ ∧ k (tree.setAtFin ⟨ix.val, hi⟩ newleaf).root := by
  unfold LightProver.MerkleRootUpdateGadget_26_26_26
  have : 2^26 < Order := by decide
  simp only [Gates, GatesGnark8, GatesDef.eq, Gates.to_binary_iff_eq_fin_to_bits_le_of_pow_length_lt this]
  apply Iff.intro
  · rintro ⟨_, ⟨hi, rfl⟩, h⟩
    simp only [MerkleRootGadget_rw] at h
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
    simp only [MerkleRootGadget_rw, MerkleTree.recover_eq_root_iff_proof_and_item_correct, Fin.toBitsLE, List.Vector.reverse_reverse]
    simp only [MerkleTree.proofAtFin] at hpr
    simp only [MerkleTree.itemAtFin] at hol
    simp only [MerkleTree.root_setAtFin_eq_recoverAtFin, MerkleTree.recoverAtFin, MerkleTree.proofAtFin] at hk
    simp_all

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
  sorry

theorem BatchAddressAppend_step_rw [Fact (CollisionResistant poseidon₂)] [Fact poseidon₂_no_zero_preimage] {rv : RangeVector (2^26)}:
    (LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_0 =>
    LightProver.Poseidon2 LowElementValue NewElementValue fun gate_1 =>
    ∃gate_2, Gates.to_binary LowElementIndices 26 gate_2 ∧
    LightProver.MerkleRootUpdateGadget_26_26_26 rv.root gate_0 gate_1 gate_2 LowElementProof fun gate_3 =>
    LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_4 =>
    ∃gate_5, gate_5 = Gates.add StartIndex offset ∧
    ∃gate_6, Gates.to_binary gate_5 26 gate_6 ∧
    LightProver.MerkleRootUpdateGadget_26_26_26 gate_3 (0:F) gate_4 gate_6 NewElementProof k) →
    ∃(hei : (StartIndex+offset).val < 2^26)
     (hli : LowElementIndices.val < 2^26)
     (currentIndex_valid : NewElementValue.val ∈ rv.ranges ⟨LowElementIndices.val, hli⟩)
     (emptyIndex_valid : rv.ranges ⟨(StartIndex+offset).val, hei⟩ = none),
     k (rv.remove NewElementValue.val ⟨LowElementIndices.val, hli⟩ ⟨(StartIndex+offset).val, hei⟩ currentIndex_valid emptyIndex_valid).root := by
  simp only [LightProver.LeafHashGadget, Poseidon2_iff_uniqueAssignment, RangeVector.root, MerkleRootUpdateGadget_rw]
  have := @Range.hashOpt_eq_poseidon_iff_is_some
  conv at this => enter [x,x,x,x,x,1]; rw [eq_comm]
  simp only [rangeTree, MerkleTree.ofFn_itemAtFin, this, Gates, GatesGnark8, GatesDef.add]
  -- apply Iff.intro
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
    convert hk
    simp only [RangeVector.remove, Range.remove, Range.remove.rlo, Range.remove.rhi]

    simp only [MerkleTree.ofFn_cond]
    rw [MerkleTree.setAtFin_comm_of_ne this]
    simp only [Range.hashOpt, Range.hash, Option.map, Option.getD, ←hlodef, ←hhidef]
    simp
  -- · sorry





  -- Range.hashOpt_eq_poseidon_iff_is_some

-- theorem BatchAddressAppend_step_rw {rv : RangeVector (2^26)}:
--     (LightProver.LeafHashGadget LowElementValue LowElementNextValue NewElementValue fun gate_2 =>
--     LightProver.Poseidon2 LowElementValue NewElementValue fun gate_3 =>
--     ∃gate_4, Gates.to_binary LowElementIndex 26 gate_4 ∧
--     LightProver.MerkleRootUpdateGadget_26_26_26 rv.root gate_2 gate_3 gate_4 LowElementProof fun gate_5 =>
--     LightProver.Poseidon2 NewElementValue LowElementNextValue fun gate_6 =>
--     ∃gate_1, Gates.to_binary StartIndex 26 gate_1 ∧
--     LightProver.MerkleRootUpdateGadget_26_26_26 gate_5 (0:F) gate_6 gate_1 NewElementProof k) ↔
--     ∃(hei : StartIndex.val < 2^26)
--      (hli : LowElementIndex.val < 2^26)
--      (currentIndex_valid : NewElementValue.val ∈ rv.ranges ⟨LowElementIndex.val, hli⟩)
--      (emptyIndex_valid : rv.ranges ⟨StartIndex.val, hei⟩ = none),
--     k (rv.remove NewElementValue.val ⟨LowElementIndex.val, hli⟩ ⟨StartIndex.val, hei⟩ currentIndex_valid emptyIndex_valid).root := by
--   simp only [LightProver.LeafHashGadget, Poseidon2_iff_uniqueAssignment]
