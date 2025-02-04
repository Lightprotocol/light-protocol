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
  LightProver.LeafHashGadget r.lo r.index r.hi v k ↔ v ∈ r ∧ k r.hash := by
  unfold LightProver.LeafHashGadget
  simp only [Poseidon3_iff_uniqueAssignment]
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

lemma LeafHashGadget_hashing {p : F → Prop} : (LightProver.LeafHashGadget lo nxt hi leaf p) → p (poseidon₃ vec![lo, nxt, hi]) := by
  simp [LightProver.LeafHashGadget]

lemma LeafHashGadget_in_tree [Fact (CollisionResistant poseidon₃)] {p : F → Prop} {tree : RangeTree 26} (p_in_tree : ∀ r, p r → ∃i, r = tree.val.itemAtFin i) :
  (LightProver.LeafHashGadget lo nxt hi leaf p) → ∃(r:Range), lo = r.lo ∧ hi = r.hi ∧ nxt = r.index := by
  intro h
  have := p_in_tree _ $ LeafHashGadget_hashing h
  rcases this with ⟨i, heq⟩
  rcases tree.prop i with ⟨r, h⟩
  rw [h] at heq
  simp [Range.hash, List.Vector.eq_cons] at heq
  apply Exists.intro r
  simp [heq]

theorem MerkleTreeRoot_LeafHashGadget_rw [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {lo hi nxt leaf ind proof} {k : F → Prop } {tree : RangeTree 26}:
  (LightProver.LeafHashGadget lo nxt hi leaf fun r =>
    ∃lv, Gates.to_binary ind 26 lv ∧
    LightProver.MerkleRootGadget_26_26_26 r lv proof fun root => Gates.eq root tree.val.root ∧ k root)
  ↔ ∃(range : Range) (h: ind.val < 2^26), tree.val.itemAtFin ⟨ind.val, h⟩ = range.hash ∧ lo = range.lo ∧ nxt = range.index ∧ hi = range.hi ∧ proof.reverse = tree.val.proofAtFin ⟨ind.val, h⟩ ∧ leaf ∈ range ∧ k tree.val.root := by
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
    apply h

def NonInclusionProof_rec {n : Nat} (lo nxt hi leaf inds roots : List.Vector F n) (proofs : List.Vector (List.Vector F 26) n) (k : List.Vector F n → Prop): Prop :=
  match n with
  | 0 => k List.Vector.nil
  | _ + 1 => LightProver.LeafHashGadget lo.head nxt.head hi.head leaf.head fun r =>
    ∃lv, Gates.to_binary inds.head 26 lv ∧
    LightProver.MerkleRootGadget_26_26_26 r lv proofs.head fun root =>
    Gates.eq root roots.head ∧ NonInclusionProof_rec lo.tail nxt.tail hi.tail leaf.tail inds.tail roots.tail proofs.tail fun rs => k (root ::ᵥ rs)

lemma NonInclusionProof_rec_equiv {lo nxt hi leaf inds roots proofs k}:
  NonInclusionProof_rec lo nxt hi leaf inds roots proofs k ↔
  LightProver.NonInclusionProof_8_8_8_8_8_8_26_8_8_26 roots leaf lo hi nxt inds proofs k := by
  rw [ ←List.Vector.ofFn_get (v:=roots)
     , ←List.Vector.ofFn_get (v:=lo)
     , ←List.Vector.ofFn_get (v:=nxt)
     , ←List.Vector.ofFn_get (v:=hi)
     , ←List.Vector.ofFn_get (v:=leaf)
     , ←List.Vector.ofFn_get (v:=inds)
     , ←List.Vector.ofFn_get (v:=proofs)
     ]
  rfl

theorem NonInclusionCircuit_rec_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {n : Nat} {trees : List.Vector (RangeTree 26) n} {leaves : List.Vector F n} {k : List.Vector F n → Prop}:
  (∃lo hi nxt inds proofs, NonInclusionProof_rec lo nxt hi leaves inds (trees.map (·.val.root)) proofs k) ↔
  k (trees.map (·.val.root)) ∧ ∀i (_: i∈[0:n]), leaves[i] ∈ trees[i] := by
  induction n with
  | zero =>
    cases trees using List.Vector.casesOn
    simp [NonInclusionProof_rec]
    intro _ _ k
    linarith [k.2]
  | succ n ih =>
    apply Iff.intro
    . intro ⟨lo, hi, nxt, inds, proofs, hp⟩
      cases lo using List.Vector.casesOn with | cons hlo tlo =>
      cases hi using List.Vector.casesOn with | cons hhi thi =>
      cases nxt using List.Vector.casesOn with | cons hnxt tnxt =>
      cases leaves using List.Vector.casesOn with | cons hleaf tleaf =>
      cases inds using List.Vector.casesOn with | cons hinds tinds =>
      cases proofs using List.Vector.casesOn with | cons hproof tproof =>
      cases trees using List.Vector.casesOn with | cons htree ttree =>
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
          exact this.2 i ⟨by simp, by simp [Nat.mod_one]; linarith⟩
    . intro ⟨hk, hmem⟩
      cases trees using List.Vector.casesOn with | cons htree ttree =>
      cases leaves using List.Vector.casesOn with | cons hleaf tleaf =>
      have := (ih (trees := ttree) (leaves := tleaf) (k := fun roots => k $ htree.val.root ::ᵥ roots)).mpr $ by
        simp at hk
        apply And.intro hk
        intro i ir
        have := hmem (i+1) ⟨by simp, by simp [Nat.mod_one]; linarith [ir.2]⟩
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
        rw [ZMod.val_natCast, Nat.mod_eq_of_lt]
        exact Nat.lt_trans ix.prop (by decide)
      apply Exists.intro
      simp [*, Membership.mem]
      simp
      rw [Nat.mod_eq_of_lt]
      · exact Fin.prop _
      · apply Nat.lt_trans
        . exact ix.prop
        . decide

theorem NonInclusionCircuit_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)] {trees : List.Vector (RangeTree 26) 8} {leaves : List.Vector F 8}:
    (∃lo hi nxt inds proofs, LightProver.NonInclusionCircuit_8_8_8_8_8_8_26_8_8_26 h (trees.map (·.val.root)) leaves lo hi nxt inds proofs) ↔
    h = inputHash poseidon₂ poseidon₃ (trees.map (·.val.root)) leaves ∧ ∀i (_: i∈[0:8]), leaves[i] ∈ trees[i] := by
  unfold LightProver.NonInclusionCircuit_8_8_8_8_8_8_26_8_8_26
  simp [←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct, Gates, GatesGnark8, TwoInputsHashChain_rw]

lemma InclusionProof_swap_ex {k : α → List.Vector F 8 → Prop} : (∃ a, LightProver.InclusionProof_8_8_8_26_8_8_26 x y z w fun r => k a r) ↔
  LightProver.InclusionProof_8_8_8_26_8_8_26 x y z w fun r => ∃a, k a r := by
  simp [InclusionProof_rw]

theorem CombinedCircuit_correct [Fact (CollisionResistant poseidon₃)] [Fact (CollisionResistant poseidon₂)]
  {inclusionTrees : List.Vector (MerkleTree F poseidon₂ 26) 8} { nonInclusionTrees : List.Vector (RangeTree 26) 8}
  {inclusionLeaves nonInclusionLeaves : List.Vector F 8}:
  (∃a b c d e f g, LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_8_26_8 h (inclusionTrees.map (·.root)) inclusionLeaves a b (nonInclusionTrees.map (·.val.root)) nonInclusionLeaves c d e f g) ↔
  h = poseidon₂ vec![inputHash poseidon₂ poseidon₃ (inclusionTrees.map (·.root)) inclusionLeaves, inputHash poseidon₂ poseidon₃ (nonInclusionTrees.map (·.val.root)) nonInclusionLeaves] ∧
  ∀i (_: i∈[0:8]), inclusionLeaves[i] ∈ inclusionTrees[i] ∧ nonInclusionLeaves[i] ∈ nonInclusionTrees[i] := by
  unfold LightProver.CombinedCircuit_8_8_8_26_8_8_8_8_8_8_8_26_8
  simp [InclusionProof_swap_ex, InclusionProof_correct, ←NonInclusionProof_rec_equiv, NonInclusionCircuit_rec_correct, TwoInputsHashChain_rw, Gates, GatesGnark8, GatesDef.eq]
  rintro _
  apply Iff.intro
  . tauto
  . intro hp
    apply And.intro
    . exact fun i ir => (hp i ir).2
    . exact fun i ir => (hp i ir).1
