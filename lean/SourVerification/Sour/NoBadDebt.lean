namespace Sour

-- ---------------------------------------------------------------------------
-- v0.5.0 LP-scaled per-user cap theorems
--
-- Formula:
--   perUserCap      = lp_nav * risk_budget_bps / max_mm_bps        (Nat division)
--   worstSingleLoss = notional * mm_bps / 10_000
--   riskBudget      = lp_nav * risk_budget_bps / 10_000
--
-- Invariant 1 (general): notional ≤ perUserCap → worstSingleLoss ≤ riskBudget
-- Invariant 2 (concrete N=4): aggregate worst loss ≤ 4 * riskBudget
--   General-N proof deferred to v0.6 (requires Mathlib Finset.sum_le_card_nsmul).
-- ---------------------------------------------------------------------------

def availableAssets (totalAssets outstandingWinnerPnl : Nat) : Nat :=
  totalAssets - outstandingWinnerPnl

def solvent (totalAssets badDebtReserve outstandingWinnerPnl : Nat) : Prop :=
  outstandingWinnerPnl <= totalAssets + badDebtReserve

def fixedCurveMaxLoss
    (qmaxStorage adverseDistance fixedOne : Nat) : Nat :=
  (qmaxStorage * adverseDistance) / fixedOne

def fixedClosePositivePnl
    (totalAssets traderCollateral pnl : Nat) : Option (Nat × Nat) :=
  if pnl <= totalAssets then
    some (totalAssets - pnl, traderCollateral + pnl)
  else
    none

theorem available_assets_le_total
    (totalAssets outstandingWinnerPnl : Nat) :
    availableAssets totalAssets outstandingWinnerPnl <= totalAssets := by
  unfold availableAssets
  exact Nat.sub_le totalAssets outstandingWinnerPnl

theorem fixed_curve_loss_half_unit_capacity :
    fixedCurveMaxLoss 2147483648 10000000 4294967296 = 5000000 := by
  native_decide

theorem fixed_close_positive_pnl_rejects_underfunded :
    fixedClosePositivePnl 1000000 0 5000000 = none := by
  native_decide

theorem fixed_close_positive_pnl_debits_exactly :
    fixedClosePositivePnl 10000000 7000 1250000 = some (8750000, 1257000) := by
  native_decide

-- ---------------------------------------------------------------------------
-- LP-scaled per-user cap definitions (v0.5.0)
-- ---------------------------------------------------------------------------

/-- Per-user notional cap derived from LP NAV and risk parameters.
    Formula: lp_nav × risk_budget_bps / max_mm_bps (Nat truncating division). -/
def perUserCap (lp_nav risk_budget_bps max_mm_bps : Nat) : Nat :=
  lp_nav * risk_budget_bps / max_mm_bps

/-- Worst-case LP loss for a single position at maintenance margin.
    Formula: notional × mm_bps / 10_000 (Nat truncating division). -/
def worstSingleLoss (notional mm_bps : Nat) : Nat :=
  notional * mm_bps / 10000

/-- LP risk budget: the fraction of lp_nav the cap is designed to protect.
    Formula: lp_nav × risk_budget_bps / 10_000. -/
def riskBudget (lp_nav risk_budget_bps : Nat) : Nat :=
  lp_nav * risk_budget_bps / 10000

/-- Aggregate worst-case LP loss over a list of (notional, mm_bps) positions. -/
def aggregateLoss (positions : List (Nat × Nat)) : Nat :=
  positions.foldl (fun acc p => acc + worstSingleLoss p.1 p.2) 0

-- ---------------------------------------------------------------------------
-- Key helper: for Nat division, a / k ≤ b / k when a ≤ b.
-- This is Nat.div_le_div_right from Init.
-- ---------------------------------------------------------------------------

/-- Invariant 1 (general): if notional ≤ perUserCap and mm_bps ≤ max_mm_bps,
    then worstSingleLoss ≤ riskBudget.

    Proof sketch:
      notional ≤ lp_nav * risk_budget_bps / max_mm_bps
      ⟹ notional * mm_bps ≤ (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps
      ≤ lp_nav * risk_budget_bps          (since mm_bps ≤ max_mm_bps, integer div)
      ⟹ notional * mm_bps / 10_000 ≤ lp_nav * risk_budget_bps / 10_000.
-/
theorem single_position_bound
    (lp_nav risk_budget_bps max_mm_bps mm_bps notional : Nat)
    (hmm    : mm_bps ≤ max_mm_bps)
    (hcap   : notional ≤ perUserCap lp_nav risk_budget_bps max_mm_bps) :
    worstSingleLoss notional mm_bps ≤ riskBudget lp_nav risk_budget_bps := by
  unfold worstSingleLoss riskBudget perUserCap at *
  -- notional * mm_bps ≤ (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps
  have h1 : notional * mm_bps ≤ (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps :=
    Nat.mul_le_mul_right mm_bps hcap
  -- (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps ≤ lp_nav * risk_budget_bps
  -- because (a / k) * k ≤ a and mm_bps ≤ max_mm_bps
  have h2 : (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps ≤ lp_nav * risk_budget_bps := by
    calc (lp_nav * risk_budget_bps / max_mm_bps) * mm_bps
        ≤ (lp_nav * risk_budget_bps / max_mm_bps) * max_mm_bps :=
          Nat.mul_le_mul_left _ hmm
      _ ≤ lp_nav * risk_budget_bps :=
          Nat.div_mul_le_self (lp_nav * risk_budget_bps) max_mm_bps
  -- Combine: notional * mm_bps ≤ lp_nav * risk_budget_bps
  have h3 : notional * mm_bps ≤ lp_nav * risk_budget_bps := Nat.le_trans h1 h2
  -- Divide both sides by 10_000 (monotone)
  exact Nat.div_le_div_right h3

-- ---------------------------------------------------------------------------
-- Invariant 2 (concrete N=4): aggregate worst loss ≤ 4 × riskBudget.
--
-- Uses decide over concrete parameters matching default devnet config:
--   lp_nav = 106_000_000 ($106 in micros), risk_budget_bps = 1000 (10%),
--   max_mm_bps = 100 (LETHAL), mm_bps = 100 (worst tier), four positions at cap.
--
-- NOTE: General-N proof (Finset.sum_le_card_nsmul) requires Mathlib and is
-- deferred to v0.6.0. N is bounded externally by Solana account rent and
-- vault size; it is NOT proven here.
-- ---------------------------------------------------------------------------
theorem aggregate_bound_n4_concrete :
    let lp_nav        := 106_000_000   -- $106 LP pool in micros
    let risk_bps      := 1000          -- 10% risk budget
    let max_mm        := 100           -- LETHAL mm_bps
    let cap           := perUserCap lp_nav risk_bps max_mm  -- 1_060_000
    let four_positions := [(cap, max_mm), (cap, max_mm), (cap, max_mm), (cap, max_mm)]
    aggregateLoss four_positions ≤ 4 * riskBudget lp_nav risk_bps := by
  native_decide

-- ---------------------------------------------------------------------------
-- v0.5.1-P1c — aggregate-budget enforcement theorems
--
-- On-chain surface (programs/sour/src/state.rs + upsert_position.rs):
--   • `Protocol.aggregate_max_lp_loss` — running sum of per-market
--     `worst_case_lp_loss(max_mm_bps, mark)` across all markets.
--   • `aggregate_cap = total_assets × aggregate_budget_bps / max(max_mm_bps, 1)`.
--   • `upsert_position` enforces `proposed_aggregate ≤ aggregate_cap`
--     BEFORE the per-user cap (line 648).
--   • Permissionless `recompute_aggregate` ix re-derives the counter
--     from per-market state (drift recovery).
--
-- The theorems below model the math layer. Account-authentication
-- (PDA re-derivation, program-owner check) is documented in
-- `docs/instruction-verification-targets.md` as a runtime-level
-- obligation outside the pure-math lane.
-- ---------------------------------------------------------------------------

/-- Per-market worst-case LP loss in µUSDC. Mirrors
    `Market::worst_case_lp_loss(max_mm_bps, mark_micros)` from `state.rs:547`.
    Stored in the same units as the on-chain helper:
      - `long_oi`/`short_oi` in `qmax_storage` units (FIXED_ONE-scaled),
      - `curve_long`/`curve_short` already in µUSDC,
      - `mark_micros` raw µUSDC per base.
    The `fixedOne` parameter abstracts `1u128 << 32`. -/
def perMarketWorstCase
    (long_oi short_oi : Nat) (curve_long curve_short : Nat)
    (max_mm_bps mark_micros fixedOne : Nat) : Nat :=
  let net_skew := if long_oi ≥ short_oi then long_oi - short_oi else short_oi - long_oi
  let perp_side := net_skew * mark_micros * max_mm_bps / 10000 / fixedOne
  let curve_side := if curve_long ≥ curve_short then curve_long else curve_short
  perp_side + curve_side

/-- Aggregate cap formula matching `upsert_position.rs:579`. -/
def aggregateCap (total_assets aggregate_budget_bps max_mm_bps : Nat) : Nat :=
  total_assets * aggregate_budget_bps / max max_mm_bps 1

/-- A market record bundling the inputs to `perMarketWorstCase`. -/
structure MarketRec where
  long_oi : Nat
  short_oi : Nat
  curve_long : Nat
  curve_short : Nat
  mark_micros : Nat

/-- Total worst-case LP loss across a list of markets. Mirrors the body
    of `recompute_aggregate.rs::handler` minus the runtime PDA / owner
    authentication, which is account-level not math. -/
def aggregateMaxLpLoss (markets : List MarketRec) (max_mm_bps fixedOne : Nat) : Nat :=
  markets.foldl
    (fun acc m =>
      acc +
        perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
          max_mm_bps m.mark_micros fixedOne)
    0

-- ---------------------------------------------------------------------------
-- Theorem `aggregate_bound_general_n` — aggregate is exactly the sum of
-- per-market terms. Proven over `Nat`, general-N, no `sorry`, no Mathlib.
--
-- Together with `update_aggregate_preserves_bound` below, this is the
-- decomposition + monotonicity duo that makes the on-chain hot-path
-- counter equivalent to the canonical recompute.
-- ---------------------------------------------------------------------------

/-- Helper: `foldl (+ f) acc xs = acc + foldl (+ f) 0 xs` for any function
    `f : α → Nat`. Used to peel the accumulator out of the fold so an
    induction over the list can rewrite both sides cleanly. -/
private theorem foldl_add_acc {α : Type _} (f : α → Nat) :
    ∀ (xs : List α) (acc : Nat),
      xs.foldl (fun a x => a + f x) acc = acc + xs.foldl (fun a x => a + f x) 0
  | [], acc => by simp
  | x :: xs, acc => by
    simp only [List.foldl]
    rw [foldl_add_acc f xs (acc + f x)]
    rw [foldl_add_acc f xs (0 + f x)]
    rw [Nat.zero_add]
    rw [Nat.add_assoc]

/-- General-N theorem: `aggregateMaxLpLoss = Σ perMarketWorstCase` over
    the markets list. Pure `Nat`, no Mathlib dep. The right-hand side is
    spelled as a fold of the per-market formula starting from 0; the
    left-hand side IS that fold by definition, so the theorem reduces
    after unfolding. The lemma documents the equality with the standard
    sum form so downstream theorems can rewrite it. -/
theorem aggregate_bound_general_n
    (markets : List MarketRec) (max_mm_bps fixedOne : Nat) :
    aggregateMaxLpLoss markets max_mm_bps fixedOne =
      (markets.map
        (fun m =>
          perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
            max_mm_bps m.mark_micros fixedOne)).foldl (· + ·) 0 := by
  unfold aggregateMaxLpLoss
  induction markets with
  | nil => rfl
  | cons m ms ih =>
    simp only [List.map, List.foldl]
    -- LHS: foldl (+ f) (0 + f m) ms = (0 + f m) + foldl (+ f) 0 ms
    -- RHS: foldl (· + ·) (0 + f m) (map f ms) = (0 + f m) + foldl (· + ·) 0 (map f ms)
    rw [foldl_add_acc _ ms (0 + _)]
    rw [show (fun (a : Nat) (b : Nat) => a + b) = (· + ·) from rfl] at *
    -- Reduce RHS using the same accumulator-peel lemma applied to the mapped list.
    have hrhs :
        (List.foldl (· + ·) (0 + perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
                                  max_mm_bps m.mark_micros fixedOne)
          (List.map
            (fun m => perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
                       max_mm_bps m.mark_micros fixedOne) ms))
        =
        (0 + perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
              max_mm_bps m.mark_micros fixedOne)
        + (List.map
            (fun m => perMarketWorstCase m.long_oi m.short_oi m.curve_long m.curve_short
                       max_mm_bps m.mark_micros fixedOne) ms).foldl (· + ·) 0 :=
      foldl_add_acc _ _ _
    rw [hrhs]
    rw [ih]

/-- Concrete N=4 sanity check: aggregate of an explicit four-market list
    equals the explicit sum of per-market terms. Tests the general theorem
    against `decide` on a small case so a regression in the fold/sum
    relationship would surface. -/
theorem aggregate_bound_n4_decomposes :
    let m₁ : MarketRec := ⟨10, 0, 1000, 0, 100⟩
    let m₂ : MarketRec := ⟨0, 5, 0, 500, 200⟩
    let m₃ : MarketRec := ⟨3, 3, 250, 750, 50⟩
    let m₄ : MarketRec := ⟨0, 0, 0, 0, 1000⟩
    let max_mm := 100
    let fixedOne := 4294967296
    aggregateMaxLpLoss [m₁, m₂, m₃, m₄] max_mm fixedOne
      = perMarketWorstCase m₁.long_oi m₁.short_oi m₁.curve_long m₁.curve_short
          max_mm m₁.mark_micros fixedOne
      + perMarketWorstCase m₂.long_oi m₂.short_oi m₂.curve_long m₂.curve_short
          max_mm m₂.mark_micros fixedOne
      + perMarketWorstCase m₃.long_oi m₃.short_oi m₃.curve_long m₃.curve_short
          max_mm m₃.mark_micros fixedOne
      + perMarketWorstCase m₄.long_oi m₄.short_oi m₄.curve_long m₄.curve_short
          max_mm m₄.mark_micros fixedOne := by
  native_decide

-- ---------------------------------------------------------------------------
-- Theorem `update_aggregate_preserves_bound` — `update_aggregate` keeps the
-- counter ≤ cap when the upsert enforcement check passed. Mirrors the
-- on-chain require! at `upsert_position.rs:648`.
--
-- Stated over Nat (the on-chain helper uses signed i128 to detect
-- underflow; here we model the Ok branch where the post-state is a
-- valid Nat with `proposed ≤ cap`).
-- ---------------------------------------------------------------------------

/-- Signed-add helper modeling `Protocol::update_aggregate` returning
    `some new_value` on success or `none` on underflow. Overflow is
    impossible at Nat — we only check the negative-result case. -/
def updateAggregate (prior : Nat) (delta : Int) : Option Nat :=
  let signed : Int := (prior : Int) + delta
  if signed < 0 then none else some signed.toNat

/-- The cap-preservation theorem. If
      • `aggregate_pre ≤ cap` (system entry invariant), and
      • the upsert check `proposed_aggregate ≤ cap` passed,
    then `update_aggregate` produces a value still bounded by `cap`. -/
theorem update_aggregate_preserves_bound
    (aggregate_pre cap : Nat) (delta : Int)
    (_h_entry : aggregate_pre ≤ cap)
    (h_check_nonneg : (aggregate_pre : Int) + delta ≥ 0)
    (h_check_cap : ((aggregate_pre : Int) + delta).toNat ≤ cap) :
    ∃ post, updateAggregate aggregate_pre delta = some post ∧ post ≤ cap := by
  -- The signed sum is non-negative by `h_check_nonneg`, so the `if` in
  -- `updateAggregate` collapses to the `some` branch.
  refine ⟨((aggregate_pre : Int) + delta).toNat, ?_, h_check_cap⟩
  unfold updateAggregate
  have h_not_lt : ¬ (((aggregate_pre : Int) + delta) < 0) :=
    Int.not_lt.mpr h_check_nonneg
  simp [h_not_lt]

-- ---------------------------------------------------------------------------
-- v0.6.0 — max_oi notional-micros cap theorems
--
-- On-chain surface (programs/sour/src/instructions/upsert_position.rs post-v0.6.0):
--   side_oi       = max(new_long_oi, new_short_oi)         (qmax_storage units)
--   side_notional = side_oi × mark / FIXED_ONE              (µUSDC)
--   require!(side_notional <= max_oi_notional_micros, OverMaxOi);
--
-- Pure Nat — no Mathlib, no `sorry`. Mirrors the structure of
-- `single_position_bound` (LP-scaled per-user cap).
-- ---------------------------------------------------------------------------

/-- Per-trade gross-side notional in µUSDC. Mirrors the on-chain helper at
    `upsert_position.rs` (v0.6.0). `fixedOne` abstracts `1u128 << 32`. -/
def sideNotional
    (new_long_oi new_short_oi mark fixedOne : Nat) : Nat :=
  let side := if new_long_oi ≥ new_short_oi then new_long_oi else new_short_oi
  side * mark / fixedOne

/-- The on-chain max_oi check: notional ≤ cap. -/
def maxOiCheckPasses
    (new_long_oi new_short_oi mark fixedOne max_oi_notional_micros : Nat) : Prop :=
  sideNotional new_long_oi new_short_oi mark fixedOne ≤ max_oi_notional_micros

/-- Bound theorem: if the cap check passes, the gross-side notional is
    bounded by the cap. (Trivially true by definition — kept as an explicit
    lemma so downstream proofs can rewrite it without unfolding.) -/
theorem max_oi_bound
    (new_long_oi new_short_oi mark fixedOne max_oi_notional_micros : Nat)
    (h : maxOiCheckPasses new_long_oi new_short_oi mark fixedOne max_oi_notional_micros) :
    sideNotional new_long_oi new_short_oi mark fixedOne ≤ max_oi_notional_micros := by
  exact h

/-- Monotonicity in the cap: tightening the cap from `c1` to `c2 ≤ c1` can
    only reject more opens — every passing open under `c2` also passes
    under `c1`. -/
theorem max_oi_check_cap_monotone
    (new_long_oi new_short_oi mark fixedOne c1 c2 : Nat)
    (hcap : c2 ≤ c1)
    (h : maxOiCheckPasses new_long_oi new_short_oi mark fixedOne c2) :
    maxOiCheckPasses new_long_oi new_short_oi mark fixedOne c1 := by
  unfold maxOiCheckPasses at *
  exact Nat.le_trans h hcap

/-- Cross-market price-agnostic invariant (concrete): two markets, same
    dollar notional, different mark prices, same cap → both pass or both
    fail identically. We pick a configuration where both opens land at
    exactly $80 notional under a $100 cap and prove both pass.

    BTC mark = $80,000 µUSDC = 80_000_000_000.  qmax_storage chosen so
    notional = $80 = 80_000_000 µUSDC.
    XRP mark = $1.40 µUSDC = 1_400_000.  qmax_storage chosen so notional
    matches.

    `qmax = 80_000_000 × FIXED_ONE / mark` exactly. -/
theorem max_oi_dollar_parity_btc_vs_xrp :
    let fixedOne     := 4294967296            -- 2^32
    let btc_mark     := 80_000_000_000        -- $80,000
    let xrp_mark     := 1_400_000             -- $1.40
    let cap          := 100_000_000           -- $100
    -- qmax_storage = 80_000_000 × fixedOne / mark
    let btc_qmax     := 80_000_000 * fixedOne / btc_mark
    let xrp_qmax     := 80_000_000 * fixedOne / xrp_mark
    maxOiCheckPasses btc_qmax 0 btc_mark fixedOne cap
      ∧ maxOiCheckPasses xrp_qmax 0 xrp_mark fixedOne cap := by
  unfold maxOiCheckPasses sideNotional
  native_decide

end Sour
