import ProvenZk
import FormalVerification.Poseidon
import FormalVerification.Circuit
import FormalVerification.Lemmas

open LightProver (F)

abbrev Address := Fin (2^248)

structure Range : Type where
  lo : Address
  hi : Address
  valid : hi > lo ∨ hi = 0

instance : LT (Range) where
  lt r₁ r₂ := r₁.hi ≠ 0 ∧ r₁.hi ≤ r₂.lo

instance : LE (Range) where
  le r₁ r₂ := r₁ = r₂ ∨ r₁ < r₂

instance : Preorder (Range) where
  le_refl r := Or.inl rfl
  le_trans a b c := by
    intro h₁ h₂
    cases a; cases b; cases c
    cases h₁
    · cases h₂
      · simp_all [LE.le]
      · simp_all [LE.le]
    · cases h₂
      · simp_all [LE.le]
      · apply Or.inr
        simp only [LT.lt, Address] at *
        casesm* _ ∧ _
        apply And.intro (by assumption)
        simp_all
        apply Fin.le_trans (by assumption) (Fin.le_trans (Fin.le_of_lt (by assumption)) (by assumption))
  lt_iff_le_not_le a b := by
    simp only [LE.le]
    apply Iff.intro
    · intro h
      apply And.intro (Or.inr h)
      intro h'
      cases h'
      · subst_vars
        cases h
        cases b
        casesm _ ∨ _
        · simp_all [not_le_of_lt]
        · simp_all
      · simp only [LT.lt] at *
        cases a
        cases b
        casesm* _ ∧ _
        casesm* _ ∨ _
        all_goals (try contradiction)
        simp_all [Address, Fin.lt_asymm]
        apply Fin.lt_irrefl
        apply Fin.lt_of_lt_of_le (by assumption)
        apply Fin.le_trans (by assumption)
        apply Fin.le_trans (Fin.le_of_lt $ by assumption)
        assumption
    · rintro ⟨h₁ | h₁, h₂⟩
      · simp_all
      · assumption

instance : PartialOrder Range where
  le_antisymm a b h₁ h₂ := by
    cases h₁
    · simp_all
    · cases h₂
      · simp_all
      · simp only [LT.lt] at *
        casesm* _ ∧ _
        cases a
        cases b
        casesm* _ ∨ _
        all_goals (try contradiction)
        by_contra
        apply Fin.lt_irrefl
        apply Fin.lt_of_lt_of_le (by assumption)
        apply Fin.le_trans (by assumption)
        apply Fin.le_trans (Fin.le_of_lt $ by assumption)
        assumption

instance : Membership Address Range where
  mem r x := r.lo < x ∧ x < r.hi

instance : Membership Address (Option (Range × Fin l)) where
  mem r x := match r with
    | none => false
    | some (r,_) => x ∈ r

instance : HasSubset Range where
  Subset r₁ r₂ := (r₁.hi = 0 → r₂.hi = 0) ∧ r₁.lo ≥ r₂.lo ∧ r₁.hi ≤ r₂.hi

lemma Range.subset_mono_gt {r r₁ r₂ : Range} : r₁ ⊆ r₂ → r < r₂ → r < r₁ := by
  intro hsub hlt
  cases r; cases r₁; cases r₂;
  cases hsub
  cases hlt
  casesm* _ ∧ _
  apply And.intro
  · simp_all
  · simp only at *
    apply le_trans
    assumption
    assumption

lemma Range.subset_mono_lt {r r₁ r₂ : Range} : r₁ ⊆ r₂ → r₂ < r → r₁ < r := by
  intro hsub hlt
  cases r; cases r₁; cases r₂;
  cases hsub
  cases hlt
  simp only at *
  apply And.intro
  · simp_all
  · simp only at *
    casesm* _ ∧ _
    apply le_trans <;> assumption


lemma isSome_of_mem {v : Address} {r : Option (Range × Fin l)} (h: v ∈ r): r.isSome := by
  cases r
  simp [Membership.mem] at h
  rfl

lemma mem_of_mem {l} {v : Address} {r : Range} {i : Fin l}: v ∈ some (r, i) → v ∈ r := by
  simp [Membership.mem]

lemma hi_ne_zero_of_mem {v:  Address} {r: Range} : v ∈ r → r.hi ≠ 0 := by
  intro ⟨_, _⟩
  apply Fin.ne_zero_of_lt
  assumption

lemma Range.mem_mono {x} {l r : Range} : (l.lo ≤ r.lo) → (r.hi ≤ l.hi) → x ∈ r → x ∈ l := by
  intros
  cases l; cases r
  simp_all [Membership.mem]
  casesm _ ∧ _
  apply And.intro
  · apply Fin.lt_of_le_of_lt (by assumption) (by assumption)
  · apply Fin.lt_of_lt_of_le (by assumption) (by assumption)


structure RangeVector (l: ℕ): Type where
  ranges : Fin l → Option (Range × Fin l)
  rangesDisjoint : ∀ (i j : Fin l), i ≠ j → match ranges i with
    | none => True
    | some (ri, _) => match ranges j with
      | none => True
      | some (rj, _) => ri < rj ∨ rj < ri
  nextIndexCorrect : ∀(i : Fin l), match ranges i with
    | none => True
    | some (r, nextIndex) => r.hi = 0 ∨ match ranges nextIndex with
      | none => False
      | some (rn, _) => rn > r ∧  ∀ j, j ≠ i ∧ j ≠ nextIndex → match ranges j with
        | none => True
        | some (rj, _) => rj < r ∨ rj > rn

instance {l : ℕ} : Membership Address (RangeVector l) where
  mem rv x := ∃(j: Fin l) (r:Range) (i:Fin l), x ∈ r ∧ some (r, i) = rv.ranges j

def Range.remove (r : Range) (v : Address) (hmem : v ∈ r) : (Range × Range) := (rlo, rhi) where
  rlo := {r with hi := v, valid := .inl hmem.1}
  rhi := {r with lo := v, valid := .inl hmem.2}

theorem Range.remove_lt {r v hmem} : (Range.remove r v hmem).1 < (r.remove v hmem).2 := by
  cases hmem
  rename_i l _
  simp [remove, remove.rlo, remove.rhi, LT.lt, Fin.ne_zero_of_lt l]

theorem Range.remove_1_subset : (Range.remove r v hmem).1 ⊆ r := by
  apply And.intro
  · cases hmem
    have := Fin.ne_zero_of_lt (b:=v) (by assumption)
    simp [remove, remove.rlo, *]
  · cases hmem
    simp [remove, remove.rlo, le_of_lt, *]

theorem Range.remove_2_subset : (Range.remove r v hmem).2 ⊆ r := by
  apply And.intro
  · cases hmem
    have := Fin.ne_zero_of_lt (b:=v) (by assumption)
    simp [remove, remove.rhi, *]
  · cases hmem
    simp [remove, remove.rhi, le_of_lt, *]

theorem Range.nextIndex_of_mem {l v} {r : RangeVector l} {i} (h : v ∈ r.ranges i):
    ∃c j, r.ranges i = some (c, j) ∧ ∃c' k, r.ranges j = some (c', k) ∧ c' > c ∧ ∀l, l ≠ i ∧ l ≠ j → match r.ranges l with
      | none => True
      | some (rj, _) => rj < c ∨ rj > c' := by
  have := r.nextIndexCorrect i
  have := Option.isSome_iff_exists.mp (isSome_of_mem h)
  cases this; rename_i w hp; cases w
  simp only [hp] at *
  have := hi_ne_zero_of_mem (mem_of_mem h)
  simp only [this] at *
  simp only [false_or] at this
  split at this
  · trivial
  rename_i hsnd
  apply Exists.intro
  apply Exists.intro
  apply And.intro rfl
  simp only [hsnd]
  apply Exists.intro
  apply Exists.intro
  apply And.intro rfl
  assumption

theorem Range.nextIndex_not_self_of_mem {l v} {r : RangeVector l} {i} (h : v ∈ r.ranges i) : ((r.ranges i).get (isSome_of_mem h) |>.2) ≠ i := by
  have := r.nextIndexCorrect i
  have := Option.isSome_iff_exists.mp (isSome_of_mem h)
  cases this
  rename_i w hp
  cases w
  simp only [hp] at this
  rw [hp] at h
  split at this
  · by_contra; simp_all
  have hinez := hi_ne_zero_of_mem (mem_of_mem h)
  simp only [hinez, false_or] at this
  intro heq
  simp only [hp, Option.get_some] at heq
  cases heq
  rename_i heq
  rw [hp] at heq
  cases heq
  apply lt_irrefl _ (this.1)


def RangeVector.remove
    (r : RangeVector l)
    (v : Fin (2^248))
    (currentIndex : Fin l)
    (emptyIndex : Fin l)
    (currentIndex_valid : v ∈ r.ranges currentIndex)
    (emptyIndex_empty : r.ranges emptyIndex = none): RangeVector l :=
  let curRange := r.ranges currentIndex |>.get (isSome_of_mem currentIndex_valid)
  let lohi := curRange.1.remove v (by simp only [Membership.mem] at currentIndex_valid; split at currentIndex_valid; contradiction; simp [Membership.mem, curRange, *])
  let lowHalfRange := (lohi.1, emptyIndex)
  let highHalfRange := (lohi.2, curRange.2)
  RangeVector.mk
    (fun i => if i = currentIndex then some lowHalfRange else if i = emptyIndex then some highHalfRange else r.ranges i)
    (by
      have curRange_def : r.ranges currentIndex = curRange := by simp [curRange]
      have : currentIndex ≠ emptyIndex := by
        rintro rfl
        rw [emptyIndex_empty] at currentIndex_valid
        simp [Membership.mem] at currentIndex_valid
      intro i j hneij
      by_cases hicurr: i = currentIndex
      · cases hicurr
        rw [ne_comm] at hneij
        by_cases hjemp : j = emptyIndex
        · cases hjemp
          simp [hneij, lowHalfRange, highHalfRange, lohi, Or.inl, Range.remove_lt]
        · have := r.rangesDisjoint j currentIndex hneij
          rw [curRange_def] at this
          simp only [curRange] at this
          simp only [*, ite_true, ite_false]
          split at this
          · trivial
          · simp only [curRange_def, Option.get_some] at this
            simp only [lowHalfRange, lohi]
            cases this
            · rename_i h
              apply Or.inr
              apply Range.subset_mono_gt
              apply Range.remove_1_subset
              assumption
            · apply Or.inl
              apply Range.subset_mono_lt
              apply Range.remove_1_subset
              assumption
      · simp only [hicurr, ite_false]
        by_cases hiemp : i = emptyIndex
        · simp only [hiemp, ite_true]
          have : ¬j = emptyIndex := by
            rintro rfl
            simp_all
          simp only [this, ite_false]
          by_cases hjcurr : j = currentIndex
          · simp only [hjcurr, ite_true]
            simp [lowHalfRange, highHalfRange, lohi, Or.inr, Range.remove_lt]
          · simp only [hjcurr, ite_false]
            split
            · trivial
            · rename_i heq
              have := r.rangesDisjoint j currentIndex hjcurr
              simp only [heq, curRange_def] at this
              simp only [highHalfRange, lohi]
              cases this
              · apply Or.inr
                apply Range.subset_mono_gt
                apply Range.remove_2_subset
                assumption
              · apply Or.inl
                apply Range.subset_mono_lt
                apply Range.remove_2_subset
                assumption
        · simp only [hiemp, ite_false]
          split
          · trivial
          · by_cases hjcurr : j = currentIndex
            · simp only [hjcurr, ite_true]
              have := r.rangesDisjoint i currentIndex hicurr
              simp only [curRange_def, *] at this
              simp only [lowHalfRange, lohi]
              cases this
              · apply Or.inl
                apply Range.subset_mono_gt
                apply Range.remove_1_subset
                assumption
              · apply Or.inr
                apply Range.subset_mono_lt
                apply Range.remove_1_subset
                assumption
            · simp only [hjcurr, ite_false]
              by_cases hjemp : j = emptyIndex
              · simp only [hjemp, ite_true]
                have := r.rangesDisjoint i currentIndex hicurr
                simp only [curRange_def, *] at this
                simp only [highHalfRange, lohi]
                cases this
                · apply Or.inl
                  apply Range.subset_mono_gt
                  apply Range.remove_2_subset
                  assumption
                · apply Or.inr
                  apply Range.subset_mono_lt
                  apply Range.remove_2_subset
                  assumption
              · simp only [hjemp, ite_false]
                split
                · trivial
                · rename_i heq
                  have := r.rangesDisjoint i j hneij
                  simp only [*] at this
                  assumption
    )
    (by
      intro i
      have curRange_def : r.ranges currentIndex = curRange := by simp [curRange]
      have : currentIndex ≠ emptyIndex := by
        rintro rfl
        rw [emptyIndex_empty] at currentIndex_valid
        simp [Membership.mem] at currentIndex_valid
      simp only
      by_cases hicurr: i = currentIndex
      · simp only [hicurr, ite_true, lowHalfRange, highHalfRange, lohi, ne_comm.mp this, ite_false]
        apply Or.inr
        simp only [Range.remove_lt, true_and]
        intro j
        rintro ⟨jnecurr, jneemp⟩
        simp only [jnecurr, jneemp, ite_false]
        split
        · trivial
        · have := r.rangesDisjoint j currentIndex jnecurr
          simp only [*] at this
          cases this
          · apply Or.inl
            apply Range.subset_mono_gt
            apply Range.remove_1_subset
            assumption
          · apply Or.inr
            apply Range.subset_mono_lt
            apply Range.remove_2_subset
            assumption
      · simp only [hicurr, ite_false]
        by_cases hiemp : i = emptyIndex
        · simp only [hiemp, ite_true]
          apply Or.inr
          simp only [lowHalfRange, highHalfRange, lohi, ite_false, curRange]
          have := Range.nextIndex_not_self_of_mem currentIndex_valid
          simp only [this, ite_false]
          -- have := Option.isSome_iff_exists.mp (isSome_of_mem currentIndex_valid)
          -- cases this; rename_i w h; cases w
          -- simp only [h] at *

          by_cases h: ((r.ranges currentIndex).get (isSome_of_mem (by assumption))).2 = emptyIndex
          · have := Option.isSome_iff_exists.mp (isSome_of_mem currentIndex_valid)
            cases this; rename_i w hsome
            cases w
            -- simp only [hsome]

            have := r.nextIndexCorrect currentIndex
            simp only [hsome] at this
            simp only [hsome] at currentIndex_valid
            have nez := hi_ne_zero_of_mem (mem_of_mem currentIndex_valid)
            simp only [nez, false_or] at this
            simp only [hsome, Option.get_some] at h
            cases h
            simp only [emptyIndex_empty] at this
          · have := Range.nextIndex_of_mem currentIndex_valid
            rcases this with ⟨c, j, hget, c', k, hgetk, hgt, hother⟩
            simp only [h, ite_false]
            simp only [hget, Option.get_some, hgetk]
            simp only [hget, Option.get_some] at this
            apply And.intro
            · apply Range.subset_mono_lt
              apply Range.remove_2_subset
              assumption
            · intro x
              intro ⟨hx₁, hx₂⟩
              simp only [hx₁, ite_false]
              by_cases h: x = currentIndex
              · simp only [h, ite_true]
                apply Or.inl
                apply Range.remove_lt
              · simp only [h, ite_false]
                have := hother x (And.intro h hx₂)
                split
                · trivial
                rename_i h
                simp only [h] at this
                cases this
                · apply Or.inl
                  apply Range.subset_mono_gt
                  apply Range.remove_2_subset
                  assumption
                · apply Or.inr
                  assumption
        · simp only [hiemp, ite_false]
          split
          · trivial
          rename_i rr jj hgeti
          have := r.nextIndexCorrect i
          simp only [hgeti] at this
          cases this
          · apply Or.inl (by assumption)
          rename_i this
          apply Or.inr
          by_cases hp: jj = currentIndex
          · simp only [hp, ite_true, lowHalfRange, lohi]
            simp only [hp, curRange_def] at this
            apply And.intro
            · apply Range.subset_mono_gt
              apply Range.remove_1_subset
              exact this.1
            · intro j ⟨hj₁, hj₂⟩
              simp only [hj₂, ite_false]
              by_cases hj: j = emptyIndex
              · simp only [hj, ite_true, highHalfRange, lohi, Range.remove_lt, Or.inr]
              · simp only [hj, ite_false]
                have := this.2 j ⟨hj₁, hj₂⟩
                split
                · trivial
                · rename_i hhh
                  simp only [hhh] at this
                  cases this
                  · simp_all
                  · apply Or.inr
                    apply Range.subset_mono_lt
                    apply Range.remove_1_subset
                    assumption
          · simp only [hp, ite_false]
            by_cases hjje : jj = emptyIndex
            · simp only [hjje, emptyIndex_empty] at this
            · simp only [hjje, ite_false]
              split
              · simp_all
              · rename_i h; simp only [h] at this
                apply And.intro (this.1)
                intro jjj ⟨hjj₁, hjj₂⟩
                by_cases hjj: jjj = currentIndex
                · simp only [hjj, ite_true, lowHalfRange, lohi]
                  have := this.2 currentIndex (by simp_all)
                  simp only [curRange_def] at this
                  cases this
                  · apply Or.inl
                    apply Range.subset_mono_lt
                    apply Range.remove_1_subset
                    assumption
                  · apply Or.inr
                    apply Range.subset_mono_gt
                    apply Range.remove_1_subset
                    assumption
                · simp only [hjj, ite_false]
                  by_cases hjj : jjj = emptyIndex
                  · have := this.2 currentIndex (And.intro (ne_comm.mp hicurr) (ne_comm.mp hp))
                    simp only [curRange_def] at this
                    simp only [hjj, ite_true, highHalfRange, lohi]
                    cases this
                    · apply Or.inl
                      apply Range.subset_mono_lt
                      apply Range.remove_2_subset
                      assumption
                    · apply Or.inr
                      apply Range.subset_mono_gt
                      apply Range.remove_2_subset
                      assumption
                  · simp only [hjj, ite_false]
                    apply this.2
                    simp_all
    )

theorem RangeVector.not_member_remove {v} {r : RangeVector l} {ci ei civ eiv} : v ∉ r.remove v ci ei civ eiv := by
  intro hp
  rcases hp with ⟨j, rj, ni, vin, rjdef⟩
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
    cases h
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
    cases h
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
    rcases hmem with ⟨j, r, nj, v'mem, hlook⟩
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
        apply Exists.intro
        apply And.intro ?_ rfl
        apply And.intro
        · simp [←hlook, v'mem.1]
        · simp [hle]
      · exists ei
        simp only [remove, ite_false, Range.remove, Range.remove.rhi, ite_true, heici]
        apply Exists.intro
        apply Exists.intro
        apply And.intro ?_ rfl
        apply And.intro
        · have := eq_or_lt_of_not_lt hle
          simp only [eq_comm, hvv, false_or] at this
          exact this
        · simp [←hlook, v'mem.2]
    · exists j, r, nj, v'mem
      simp only [remove, Range.remove, Range.remove.rlo, Range.remove.rhi, h, jneemp, ite_false, hlook]
  · intro hmem
    rcases hmem with ⟨j, r, nj, v'mem, hlook⟩
    by_cases hjci : j = ci
    · cases hjci
      simp only [remove, ite_true, Range.remove, Range.remove.rlo] at hlook
      cases hlook
      exists ci
      apply Exists.intro
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
        apply Exists.intro
        apply And.intro ?_ (by rw [←hlook])
        assumption


def Range.hash : Range → F := fun r => poseidon₃ vec![r.lo, r.nextIndex, r.hi]

def RangeTree (d : ℕ) : Type := { t: MerkleTree F poseidon₂ d // ∀ (i : Fin (2^d)), ∃ range, t.itemAtFin i = Range.hash range }

def rangeTreeMem {d} : Range → RangeTree d → Prop := fun r t => r.hash ∈ t.val


instance {d} : Membership F (RangeTree d) where
  mem t x := ∃(r:Range), rangeTreeMem r t ∧ x ∈ r
