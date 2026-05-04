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

end Sour
