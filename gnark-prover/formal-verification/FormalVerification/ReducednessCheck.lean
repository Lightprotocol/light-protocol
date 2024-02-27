import ProvenZk
import FormalVerification
import FormalVerification.Common

open SemaphoreMTB (F Order)

abbrev orderBinaryLE : Vector Bool 256 := vec![true,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,false,true,true,true,true,true,true,false,false,true,false,false,true,true,false,true,false,true,true,true,true,true,false,false,false,false,true,true,true,true,true,false,false,false,false,true,false,true,false,false,false,true,false,false,true,false,false,false,false,true,true,true,false,true,false,false,true,true,true,false,true,true,false,false,true,true,true,true,false,false,false,false,true,false,false,true,false,false,false,false,true,false,true,true,true,true,true,false,false,true,true,false,false,false,false,false,true,false,true,false,false,true,false,true,true,true,false,true,false,false,false,false,true,true,false,true,false,true,false,false,false,false,false,false,true,true,false,false,false,false,false,false,true,false,true,true,false,true,true,false,true,true,false,true,false,false,false,true,false,false,false,false,false,true,false,true,false,false,false,false,true,true,true,false,true,true,false,false,true,false,true,false,false,false,false,false,false,false,true,false,true,true,false,false,false,true,true,false,false,true,false,false,false,false,true,true,true,false,true,false,false,true,true,true,false,false,true,true,true,false,false,true,false,false,false,true,false,false,true,true,false,false,false,false,false,true,true,false,false]

def binaryComparisonCircuit
  (base : Vector Bool n)
  (arg : Vector F n)
  (start_ix : ℕ)
  (ix_ok : start_ix < n)
  (succeeded : F)
  (failed : F): Prop :=
  Gates.is_bool arg[start_ix] ∧
  match base[start_ix] with
  | false =>
    ∃or, Gates.or arg[start_ix] failed or ∧
    ∃failed, Gates.select succeeded 0 or failed ∧
    match start_ix with
    | 0  => Gates.eq succeeded 1
    | Nat.succ ix => binaryComparisonCircuit base arg ix (Nat.lt_of_succ_lt ix_ok) succeeded failed
  | true =>
    ∃bit_neg, bit_neg = Gates.sub 1 arg[start_ix] ∧
    ∃or, Gates.or bit_neg succeeded or ∧
    ∃succeeded, Gates.select failed 0 or succeeded ∧
    match start_ix with
    | 0 => Gates.eq succeeded 1
    | Nat.succ start_ix => binaryComparisonCircuit base arg start_ix (Nat.lt_of_succ_lt ix_ok) succeeded failed

theorem ReducedModRCheck_256_Fold {v : Vector F 256} :
  binaryComparisonCircuit orderBinaryLE v 255 (by decide) 0 0 ↔ SemaphoreMTB.ReducedModRCheck_256 v := by
  repeat (first | intro _ | apply and_congr_right' | apply exists_congr)
  tauto

def binaryComparison (base arg : Vector Bool n) (start_ix : Fin n) (succeeded failed : Bool): Prop :=
  let (succeeded, failed) := match base[start_ix] with
  | false =>
    let or := arg[start_ix] || failed
    let failed := if succeeded then false else or
    (succeeded, failed)
  | true =>
    let bit_neg := !arg[start_ix]
    let or := bit_neg || succeeded
    let succeeded := if failed then false else or
    (succeeded, failed)
  match start_ix with
  | ⟨0, _⟩ => succeeded = true
  | ⟨Nat.succ ix, p⟩ => binaryComparison base arg ⟨ix, by linarith⟩ succeeded failed

theorem binaryComparison_iff_binaryComparisonCircuit {base arg : Vector Bool n} {ix : Nat} {ix_ok : ix < n} {succeeded failed : Bool}:
    binaryComparisonCircuit base (arg.map Bool.toZMod) ix ix_ok succeeded.toZMod failed.toZMod ↔
    binaryComparison base arg ⟨ix, by linarith⟩ succeeded failed := by
  induction ix generalizing succeeded failed with
  | zero =>
    simp [binaryComparison, binaryComparisonCircuit, getElem]
    split <;> simp [*]
  | succ n ih =>
    unfold binaryComparison binaryComparisonCircuit
    simp [getElem, ←ih]
    split <;> simp [*]

lemma binaryComparison_failed_always_fails {base arg : Vector Bool n} {i : Fin n}:
    ¬binaryComparison base arg i false true := by
  rcases i with ⟨i, p⟩
  induction i <;> {
    unfold binaryComparison
    simp
    split <;> { simp [*] }
  }

lemma binaryComparison_succeeded_always_succeeds {base arg : Vector Bool n} {i : Fin n}:
    binaryComparison base arg i true false := by
  rcases i with ⟨i, p⟩
  induction i <;> {
    unfold binaryComparison
    simp
    split <;> simp [*]
  }

lemma binaryComparison_unused_snoc {a b s f : Bool} { base arg : Vector Bool (Nat.succ n) } {i : ℕ} (hp : i < Nat.succ n):
    binaryComparison (base.snoc b) (arg.snoc a) ⟨i, by linarith⟩ s f ↔
    binaryComparison base arg ⟨i, hp⟩ s f := by
  induction i generalizing s f with
  | zero =>
    unfold binaryComparison
    simp [Vector.getElem_snoc_before_length]
  | succ i ih =>
    unfold binaryComparison
    have : i < n.succ := by linarith
    simp [Vector.getElem_snoc_before_length hp, Vector.getElem_snoc_at_length, ih (hp := this)]

theorem binaryComparison_is_comparison {base arg : Vector Bool (Nat.succ n)}:
    binaryComparison base arg ⟨n, by simp⟩ false false ↔
    (Fin.ofBitsLE base).val > (Fin.ofBitsLE arg).val := by
  induction n with
  | zero =>
    simp only [←Bool.toZMod_zero]
    cases base using Vector.casesOn; rename_i bhd btl; cases btl using Vector.casesOn
    cases arg using Vector.casesOn; rename_i ahd atl; cases atl using Vector.casesOn
    unfold binaryComparison
    cases ahd <;> cases bhd <;> simp
  | succ n ih =>
    cases base using Vector.revCasesOn with | snoc binit blast =>
    cases arg using Vector.revCasesOn with | snoc ainit alast =>
    simp only [←Bool.toZMod_zero]
    unfold Fin.ofBitsLE Fin.ofBitsBE binaryComparison
    simp [Vector.getElem_snoc_at_length, binaryComparison_unused_snoc]
    cases alast <;> cases blast
    . simp [ih, Fin.ofBitsLE]
    . simp [binaryComparison_succeeded_always_succeeds, Bool.toNat]
      apply Nat.lt_of_lt_of_le (Fin.is_lt _)
      simp
    . simp [binaryComparison_failed_always_fails, Bool.toNat]
      apply Nat.le_trans (Nat.le_of_lt (Fin.is_lt _))
      simp
    . simp [ih, Fin.ofBitsLE]

theorem fin_ofBitsLE_orderBinaryLE_eq_order :
  Fin.ofBitsLE orderBinaryLE = Order := by rfl

theorem ReducedModRCheck_256_semantics {v : Vector Bool 256}:
    SemaphoreMTB.ReducedModRCheck_256 (v.map Bool.toZMod) ↔ (Fin.ofBitsLE v).val < Order := by
  rw [ ←ReducedModRCheck_256_Fold
     , ←Bool.toZMod_zero
     , binaryComparison_iff_binaryComparisonCircuit
     , binaryComparison_is_comparison
     , fin_ofBitsLE_orderBinaryLE_eq_order]
  simp [Nat.mod_eq_of_lt]
