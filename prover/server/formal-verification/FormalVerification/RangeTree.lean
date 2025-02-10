import ProvenZk
import FormalVerification.Poseidon
import FormalVerification.Circuit
import FormalVerification.Lemmas

open LightProver (F)

abbrev Address := Fin (2^248)

structure Range : Type where
  lo : Address
  hi : Address
  valid : hi > lo

instance : Membership Address Range where
  mem r x := r.lo < x ∧ x < r.hi

instance : Membership Address (Option Range) where
  mem r x := match r with
    | none => false
    | some r => x ∈ r

instance : HasSubset Range where
  Subset r₁ r₂ := r₁.lo ≥ r₂.lo ∧ r₁.hi ≤ r₂.hi

def Range.disjoint (r₁ r₂ : Range) : Prop := r₁.hi ≤ r₂.lo ∨ r₂.hi ≤ r₁.lo

lemma Range.not_mem_of_mem_and_disjoint {r₁ r₂ : Range} {v : Address} (h₁ : v ∈ r₁) (h₂ : r₁.disjoint r₂): v ∉ r₂ := by
  intro hmem
  cases hmem
  cases h₁
  cases h₂
  · apply lt_irrefl (a := v)
    apply lt_trans
    assumption
    apply lt_of_le_of_lt
    assumption
    assumption
  · apply lt_irrefl (a := v)
    apply lt_trans
    case b => exact r₂.hi
    assumption
    apply lt_of_le_of_lt
    assumption
    assumption

lemma Range.disjoint_of_disjoint_subset {r r₁ r₂ : Range} (h₁ : r₁ ⊆ r₂) (h₂ : r.disjoint r₂): r.disjoint r₁ := by
  cases h₁
  unfold disjoint
  cases h₂
  · apply Or.inl
    apply le_trans <;> assumption
  · apply Or.inr
    apply le_trans <;> assumption

lemma isSome_of_mem {v : Address} {r : Option Range} (h: v ∈ r): r.isSome := by
  cases r
  simp [Membership.mem] at h
  rfl

lemma mem_of_mem {v : Address} {r : Range}: v ∈ some r → v ∈ r := by
  simp [Membership.mem]

lemma hi_ne_zero_of_mem {v:  Address} {r: Range} : v ∈ r → r.hi ≠ 0 := by
  intro ⟨_, _⟩
  apply Fin.ne_zero_of_lt
  assumption


structure RangeVector (l: ℕ): Type where
  ranges : Fin l → Option Range
  rangesDisjoint : ∀ (i j : Fin l), i ≠ j → match ranges i with
    | none => True
    | some ri => match ranges j with
      | none => True
      | some rj => ri.disjoint rj

instance {l : ℕ} : Membership Address (RangeVector l) where
  mem rv x := ∃(j: Fin l) (r:Range), x ∈ r ∧ some r = rv.ranges j

def Range.remove (r : Range) (v : Address) (hmem : v ∈ r) : (Range × Range) := (rlo, rhi) where
  rlo := {r with hi := v, valid := hmem.1}
  rhi := {r with lo := v, valid := hmem.2}

theorem Range.remove_1_subset : (Range.remove r v hmem).1 ⊆ r := by
  apply And.intro
  · cases hmem
    have := Fin.ne_zero_of_lt (b:=v) (by assumption)
    simp [remove, remove.rlo, *]
  · cases hmem
    simp [remove, remove.rlo, le_of_lt, *]

theorem Range.remove_2_subset : (Range.remove r v hmem).2 ⊆ r := by
  apply And.intro <;> {
    cases hmem
    simp [remove, remove.rhi, *, le_of_lt]
  }

theorem Range.disjoint_comm {r₁ r₂ : Range} : r₁.disjoint r₂ ↔ r₂.disjoint r₁ := by
  unfold disjoint
  apply Or.comm

theorem Range.remove_disjoint {r : Range} {v : Address} {hmem : v ∈ r} : (Range.remove r v hmem).1.disjoint (Range.remove r v hmem).2 := by
  simp only [Range.remove, remove.rlo, remove.rhi]
  cases hmem
  simp [disjoint]

def RangeVector.remove
    (r : RangeVector l)
    (v : Fin (2^248))
    (currentIndex : Fin l)
    (emptyIndex : Fin l)
    (currentIndex_valid : v ∈ r.ranges currentIndex)
    (emptyIndex_empty : r.ranges emptyIndex = none): RangeVector l :=
  let curRange := r.ranges currentIndex |>.get (isSome_of_mem currentIndex_valid)
  let lohi := curRange.remove v (by simp only [Membership.mem] at currentIndex_valid; split at currentIndex_valid; contradiction; simp [Membership.mem, curRange, *])
  RangeVector.mk
    (fun i => if i = currentIndex then some lohi.1 else if i = emptyIndex then some lohi.2 else r.ranges i)
    (fun i j ne => by
      simp only
      have curRange_def : r.ranges currentIndex = some curRange := by
        simp [curRange]
      have : emptyIndex ≠ currentIndex := by
        intro heq
        cases heq
        simp only [Membership.mem] at currentIndex_valid
        split at currentIndex_valid
        · contradiction
        rename_i heq
        rw [heq] at emptyIndex_empty
        cases emptyIndex_empty
      by_cases hicurr : i = currentIndex <;> by_cases hjcurr : j = currentIndex <;> by_cases hiemp : i = emptyIndex <;> by_cases hjemp : j = emptyIndex
      any_goals (simp_all [Range.remove_disjoint, lohi, Range.disjoint_comm]; done)
      any_goals (simp only [*, ite_true, ite_false, lohi, curRange_def])
      · have := r.rangesDisjoint currentIndex j (ne_comm.mp hjcurr)
        simp only [curRange_def] at this
        split at this
        · trivial
        · rw [Range.disjoint_comm]
          apply Range.disjoint_of_disjoint_subset
          · apply Range.remove_1_subset
          rw [Range.disjoint_comm]
          assumption
      · have := r.rangesDisjoint i currentIndex (hicurr)
        simp only [curRange_def] at this
        split at this
        · trivial
        · apply Range.disjoint_of_disjoint_subset
          · apply Range.remove_1_subset
          assumption
      · have := r.rangesDisjoint currentIndex j (ne_comm.mp hjcurr)
        simp only [curRange_def] at this
        split at this
        · trivial
        · rw [Range.disjoint_comm]
          apply Range.disjoint_of_disjoint_subset
          · apply Range.remove_2_subset
          rw [Range.disjoint_comm]
          assumption
      · have := r.rangesDisjoint i currentIndex (hicurr)
        simp only [curRange_def] at this
        split at this
        · trivial
        · apply Range.disjoint_of_disjoint_subset
          · apply Range.remove_2_subset
          assumption
      · have := r.rangesDisjoint i j ne
        assumption
    )

theorem RangeVector.not_member_remove {v} {r : RangeVector l} {ci ei civ eiv} : v ∉ r.remove v ci ei civ eiv := by
  intro hp
  rcases hp with ⟨j, rj, vin, rjdef⟩
  simp only [remove, Range.remove, Range.remove.rlo, Range.remove.rhi] at rjdef
  split at rjdef
  · cases rjdef
    cases vin
    rename_i h
    apply lt_irrefl _ h
  split at rjdef
  · cases rjdef
    cases vin
    rename_i h _
    apply lt_irrefl _ h
  have := r.rangesDisjoint j ci (by assumption)
  have := Option.isSome_iff_exists.mp (isSome_of_mem civ)
  cases this; rename_i w hp; cases w;
  simp only [←rjdef, hp] at this
  cases this
  · rename_i h
    rw [hp] at civ
    cases civ
    cases vin
    apply Fin.lt_irrefl v
    apply lt_trans
    assumption
    apply lt_of_le_of_lt
    assumption
    assumption
  · rename_i h
    rw [hp] at civ
    cases civ
    cases vin
    apply Fin.lt_irrefl v
    apply lt_trans
    rename_i h _ _
    exact h
    apply lt_of_le_of_lt
    assumption
    assumption

theorem RangeVector.members_same {v v'} {r : RangeVector l} {ci ei civ eiv} (hvv : v ≠ v') : v' ∈ r ↔ v' ∈ r.remove v ci ei civ eiv := by
  have heici : ei ≠ ci := by
    intro heq
    simp only [Membership.mem] at civ
    cases heq
    simp_all
  have := Option.isSome_iff_exists.mp (isSome_of_mem civ)
  cases this; rename_i w rcidef; cases w
  apply Iff.intro
  · intro hmem
    rcases hmem with ⟨j, r, v'mem, hlook⟩
    have jneemp : j ≠ ei := by
      intro heq
      cases heq
      rw [eiv] at hlook
      cases hlook
    by_cases h: j = ci
    · cases h
      by_cases hle : v' < v
      · exists ci
        simp only [remove, ite_true, Range.remove, Range.remove.rlo]
        apply Exists.intro
        apply And.intro ?_ rfl
        apply And.intro
        · simp [←hlook, v'mem.1]
        · simp [hle]
      · exists ei
        simp only [remove, ite_false, Range.remove, Range.remove.rhi, ite_true, heici]
        apply Exists.intro
        apply And.intro ?_ rfl
        apply And.intro
        · have := eq_or_lt_of_not_lt hle
          simp only [eq_comm, hvv, false_or] at this
          exact this
        · simp [←hlook, v'mem.2]
    · exists j, r, v'mem
      simp only [remove, Range.remove, Range.remove.rlo, Range.remove.rhi, h, jneemp, ite_false, hlook]
  · intro hmem
    rcases hmem with ⟨j, r, v'mem, hlook⟩
    by_cases hjci : j = ci
    · cases hjci
      simp only [remove, ite_true, Range.remove, Range.remove.rlo] at hlook
      cases hlook
      exists ci
      apply Exists.intro
      apply And.intro ?_ (by rw [←rcidef])
      simp only [rcidef, Option.get_some] at v'mem
      cases v'mem
      apply And.intro
      · assumption
      · rw [rcidef] at civ
        cases civ
        apply lt_trans
        assumption
        assumption
    · by_cases hjemp : j = ei
      · cases hjemp
        simp only [remove, ite_false, Range.remove, Range.remove.rhi, ite_true, heici] at hlook
        cases hlook
        exists ci
        apply Exists.intro
        apply And.intro ?_ (by rw [←rcidef])
        simp only [rcidef, Option.get_some] at v'mem
        cases v'mem
        apply And.intro
        · rw [rcidef] at civ
          cases civ
          apply lt_trans
          assumption
          assumption
        · assumption
      · exists j
        simp only [remove, ite_false, Range.remove, Range.remove.rlo, Range.remove.rhi, hjci, hjemp] at hlook
        apply Exists.intro
        apply And.intro ?_ (by rw [←hlook])
        assumption


def Range.hash : Range → F := fun r => poseidon₃ vec![r.lo, r.nextIndex, r.hi]

def RangeTree (d : ℕ) : Type := { t: MerkleTree F poseidon₂ d // ∀ (i : Fin (2^d)), ∃ range, t.itemAtFin i = Range.hash range }

def rangeTreeMem {d} : Range → RangeTree d → Prop := fun r t => r.hash ∈ t.val


instance {d} : Membership F (RangeTree d) where
  mem t x := ∃(r:Range), rangeTreeMem r t ∧ x ∈ r
