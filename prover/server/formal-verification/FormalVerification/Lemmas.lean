import Mathlib
import ProvenZk
import FormalVerification.Circuit

open LightProver (F Order)

axiom bn254_Fr_prime : Nat.Prime Order
instance : Fact (Nat.Prime Order) := Fact.mk bn254_Fr_prime

instance : Membership α (MerkleTree α H d) where
  mem t x := ∃i, t.itemAtFin i = x

namespace ZMod

lemma castInt_lt [NeZero N] {n : ZMod N}: (n.cast:ℤ) < N := by
  rw [cast_eq_val, Nat.cast_lt]
  apply ZMod.val_lt

lemma castInt_nonneg [NeZero N] {n : ZMod N}: (0:ℤ) ≤ n.cast := by
  rw [cast_eq_val]
  apply Int.ofNat_nonneg

lemma castInt_neg [NeZero N] {n : ZMod N}: ((-n).cast : ℤ) = -(n.cast:ℤ) % N := by
  rw [cast_eq_val, neg_val]
  split
  . simp [*]
  . rw [Nat.cast_sub]
    . rw [←Int.add_emod_self_left, Int.emod_eq_of_lt]
      . simp; rfl
      . linarith [castInt_lt (N:=N) (n:=n)]
      . simp_arith
        rw [ZMod.cast_eq_val, ←Int.ofNat_zero, Int.ofNat_lt]
        apply Nat.zero_lt_of_ne_zero
        simp [*]
    . exact Nat.le_of_lt (ZMod.val_lt _)


lemma castInt_add [NeZero N] {n m : ZMod N}: ((n + m).cast : ℤ) = ((n.cast:ℤ) + (m.cast:ℤ)) % N := by
  rw [ZMod.cast_eq_val, val_add]
  simp

lemma castInt_sub [NeZero N] {n m : ZMod N}: ((n - m).cast : ℤ) = ((n.cast:ℤ) - (m.cast:ℤ)) % N := by
  rw [sub_eq_add_neg, castInt_add, castInt_neg]
  simp
  rfl

end ZMod

namespace Int

lemma ofNat_pow {a b : ℕ} : (a^b : ℤ) = (OfNat.ofNat a)^b := by simp [OfNat.ofNat]

theorem negSucc_le_negSucc (m n : Nat) : negSucc m ≤ negSucc n ↔ n ≤ m := by
  rw [le_def]
  apply Iff.intro
  . conv => lhs; arg 1; whnf
    split
    . rename_i h; intro; rw [Nat.succ_sub_succ_eq_sub] at h; exact Nat.le_of_sub_eq_zero h
    . intro; contradiction
  . intro hp;
    conv => arg 1; whnf
    split
    . apply NonNeg.mk
    . rename_i hpc
      linarith [Nat.lt_of_sub_eq_succ hpc]

-- theorem emod_negSucc (m : Nat) (n : Int) :
  -- negSucc m % n = subNatNat (natAbs n) (Nat.succ (m % natAbs n)) := rfl

theorem emod_eq_add_self_of_neg_and_lt_neg_self {a : ℤ} {mod : ℤ}: a < 0 → a ≥ -mod → a % mod = a + mod := by
  intro hlt hge
  rw [←add_emod_self]
  apply emod_eq_of_lt
  . linarith
  . linarith

end Int

macro_rules
| `(tactic| get_elem_tactic_trivial) => `(tactic| (exact Membership.get_elem_helper (by assumption) (by rfl)))

def Std.Range.toList (r : Std.Range): List Nat := go r.step r.step_pos r.start (r.stop - r.start) where
  step_pos := r.step_pos
  go step step_pos start
  | 0 => []
  | i + 1 => start :: go step step_pos (start + step) (i + 1 - step)
    termination_by fuel => fuel
    decreasing_by
      simpa

theorem Std.Range.mem_toList_of_mem {r : Std.Range} (hp : i ∈ r) : i ∈ r.toList := by
  rcases r with ⟨start, stop, step, step_pos⟩
  rcases hp with ⟨h₁, h₂, h₃⟩
  simp at h₁ h₂
  have h₃ : ∃d d', stop = start + d * step + d' ∧ d' < step := by
    exists (stop - start) / step
    exists (stop - start) % step
    apply And.intro
    · have : start < stop := Nat.lt_of_le_of_lt h₁ h₂
      zify [this]
      rw [Int.add_assoc, Int.ediv_add_emod']
      simp
    · apply Nat.mod_lt; simpa
  rcases h₃ with ⟨d, d', ⟨_⟩, _⟩
  induction d generalizing start i d' with
  | zero =>
    simp at h₂
    simp at h₃
    have : i ≤ start := by
      rw [Nat.mod_eq_of_lt] at h₃
      · apply Nat.le_of_sub_eq_zero
        assumption
      · apply Nat.lt_trans (m := d')
        · apply Nat.sub_lt_left_of_lt_add <;> assumption
        · assumption
    have : i = start := by
      apply Nat.le_antisymm <;> assumption
    cases this
    unfold toList toList.go
    simp
    split
    · linarith
    · simp
  | succ d ih =>
    simp [toList, toList.go]
    unfold toList.go
    have : start + (d + 1) * step + d' - start ≠ 0 := by
      rw [Nat.add_comm, ←Nat.add_assoc, Nat.add_comm, ←Nat.add_assoc]
      simp
      rintro rfl
      contradiction
    split
    · contradiction
    rename_i heq
    cases h₁ with
    | refl => simp
    | @step m h₁ =>
      simp
      apply Or.inr
      simp [toList] at ih
      simp at h₃
      simp only [Nat.add_one, ←heq]
      conv =>
        lhs; arg 4
        calc
          _ = start + d.succ * step + d' - (start + step) := Nat.sub_sub _ _ _
          _ = start + step + d * step + d' - (start + step) := by rw [Nat.succ_mul]; ring_nf

      have h₄ : start + step ≤ m.succ := by
        have := Nat.dvd_of_mod_eq_zero h₃
        cases this
        rename_i dd h
        cases dd
        · simp_arith at h
          have := Nat.le_of_sub_eq_zero h
          have := Nat.lt_of_le_of_lt h₁ (Nat.lt_add_one m)
          linarith
        · rename_i n
          have : m.succ = start + step * (n + 1) := by
            rw [add_comm]
            apply Nat.eq_add_of_sub_eq
            · apply Nat.le_succ_of_le
              assumption
            · assumption
          rw [this]
          simp [Nat.mul_succ]

      apply ih
      · assumption
      · assumption
      · calc
          _ < _ := h₂
          _ = start + step + d * step + d' := by simp_arith [Nat.succ_mul]
      · apply Nat.mod_eq_zero_of_dvd
        rw [←Nat.sub_sub]
        apply Nat.dvd_sub
        apply Nat.le_sub_of_add_le
        · rw [Nat.add_comm]
          assumption
        · apply Nat.dvd_of_mod_eq_zero
          assumption
        · simp

@[simp]
lemma MerkleTree.GetElem.def {tree : MerkleTree α H d} {i : ℕ} {ih : i < 2^d}:
  tree[i] = tree.itemAtFin ⟨i, ih⟩ := by rfl

theorem Vector.exists_ofElems {p : Fin n → α → Prop} : (∀ (i : Fin n), ∃j, p i j) ↔ ∃(v : List.Vector α n), ∀i (_: i<n), p ⟨i, by assumption⟩ v[i] := by
  apply Iff.intro
  . intro h
    induction n with
    | zero =>
      exists List.Vector.nil
      intro i h
      linarith [h]
    | succ n ih =>
      rw [List.Vector.exists_succ_iff_exists_snoc]
      have hp_init := ih fun (i : Fin n) => h (Fin.castLE (by linarith) i)
      rcases hp_init with ⟨vinit, hpinit⟩
      exists vinit
      have hp_last := h (Fin.last n)
      rcases hp_last with ⟨vlast, hplast⟩
      exists vlast
      intro i ihp
      cases Nat.lt_succ_iff_lt_or_eq.mp ihp with
      | inl ihp =>
        simp [ihp]
        apply hpinit
      | inr ihp =>
        simp [ihp]
        apply hplast
  . rintro ⟨v, h⟩ i
    exact ⟨v[i], h i i.2⟩
