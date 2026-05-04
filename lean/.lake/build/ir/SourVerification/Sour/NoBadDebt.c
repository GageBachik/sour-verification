// Lean compiler output
// Module: SourVerification.Sour.NoBadDebt
// Imports: public import Init
#include <lean/lean.h>
#if defined(__clang__)
#pragma clang diagnostic ignored "-Wunused-parameter"
#pragma clang diagnostic ignored "-Wunused-label"
#elif defined(__GNUC__) && !defined(__CLANG__)
#pragma GCC diagnostic ignored "-Wunused-parameter"
#pragma GCC diagnostic ignored "-Wunused-label"
#pragma GCC diagnostic ignored "-Wunused-but-set-variable"
#endif
#ifdef __cplusplus
extern "C" {
#endif
lean_object* lean_nat_sub(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_availableAssets(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_availableAssets___boxed(lean_object*, lean_object*);
lean_object* lean_nat_mul(lean_object*, lean_object*);
lean_object* lean_nat_div(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedCurveMaxLoss(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedCurveMaxLoss___boxed(lean_object*, lean_object*, lean_object*);
uint8_t lean_nat_dec_le(lean_object*, lean_object*);
lean_object* lean_nat_add(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedClosePositivePnl(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedClosePositivePnl___boxed(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_perUserCap(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_perUserCap___boxed(lean_object*, lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_worstSingleLoss(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_worstSingleLoss___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_riskBudget(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_riskBudget___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0___boxed(lean_object*, lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_aggregateLoss(lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_aggregateLoss___boxed(lean_object*);
LEAN_EXPORT lean_object* lp_SourVerification_Sour_availableAssets(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lean_nat_sub(x_1, x_2);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_availableAssets___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_SourVerification_Sour_availableAssets(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedCurveMaxLoss(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; lean_object* x_5; 
x_4 = lean_nat_mul(x_1, x_2);
x_5 = lean_nat_div(x_4, x_3);
lean_dec(x_4);
return x_5;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedCurveMaxLoss___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = lp_SourVerification_Sour_fixedCurveMaxLoss(x_1, x_2, x_3);
lean_dec(x_3);
lean_dec(x_2);
lean_dec(x_1);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedClosePositivePnl(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
uint8_t x_4; 
x_4 = lean_nat_dec_le(x_3, x_1);
if (x_4 == 0)
{
lean_object* x_5; 
x_5 = lean_box(0);
return x_5;
}
else
{
lean_object* x_6; lean_object* x_7; lean_object* x_8; lean_object* x_9; 
x_6 = lean_nat_sub(x_1, x_3);
x_7 = lean_nat_add(x_2, x_3);
x_8 = lean_alloc_ctor(0, 2, 0);
lean_ctor_set(x_8, 0, x_6);
lean_ctor_set(x_8, 1, x_7);
x_9 = lean_alloc_ctor(1, 1, 0);
lean_ctor_set(x_9, 0, x_8);
return x_9;
}
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_fixedClosePositivePnl___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = lp_SourVerification_Sour_fixedClosePositivePnl(x_1, x_2, x_3);
lean_dec(x_3);
lean_dec(x_2);
lean_dec(x_1);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_perUserCap(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; lean_object* x_5; 
x_4 = lean_nat_mul(x_1, x_2);
x_5 = lean_nat_div(x_4, x_3);
lean_dec(x_4);
return x_5;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_perUserCap___boxed(lean_object* x_1, lean_object* x_2, lean_object* x_3) {
_start:
{
lean_object* x_4; 
x_4 = lp_SourVerification_Sour_perUserCap(x_1, x_2, x_3);
lean_dec(x_3);
lean_dec(x_2);
lean_dec(x_1);
return x_4;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_worstSingleLoss(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; lean_object* x_4; lean_object* x_5; 
x_3 = lean_nat_mul(x_1, x_2);
x_4 = lean_unsigned_to_nat(10000u);
x_5 = lean_nat_div(x_3, x_4);
lean_dec(x_3);
return x_5;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_worstSingleLoss___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_SourVerification_Sour_worstSingleLoss(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_riskBudget(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; lean_object* x_4; lean_object* x_5; 
x_3 = lean_nat_mul(x_1, x_2);
x_4 = lean_unsigned_to_nat(10000u);
x_5 = lean_nat_div(x_3, x_4);
lean_dec(x_3);
return x_5;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_riskBudget___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_SourVerification_Sour_riskBudget(x_1, x_2);
lean_dec(x_2);
lean_dec(x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0(lean_object* x_1, lean_object* x_2) {
_start:
{
if (lean_obj_tag(x_2) == 0)
{
return x_1;
}
else
{
lean_object* x_3; lean_object* x_4; lean_object* x_5; lean_object* x_6; lean_object* x_7; lean_object* x_8; 
x_3 = lean_ctor_get(x_2, 0);
x_4 = lean_ctor_get(x_2, 1);
x_5 = lean_ctor_get(x_3, 0);
x_6 = lean_ctor_get(x_3, 1);
x_7 = lp_SourVerification_Sour_worstSingleLoss(x_5, x_6);
x_8 = lean_nat_add(x_1, x_7);
lean_dec(x_7);
lean_dec(x_1);
x_1 = x_8;
x_2 = x_4;
goto _start;
}
}
}
LEAN_EXPORT lean_object* lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0___boxed(lean_object* x_1, lean_object* x_2) {
_start:
{
lean_object* x_3; 
x_3 = lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0(x_1, x_2);
lean_dec(x_2);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_aggregateLoss(lean_object* x_1) {
_start:
{
lean_object* x_2; lean_object* x_3; 
x_2 = lean_unsigned_to_nat(0u);
x_3 = lp_SourVerification_List_foldl___at___00Sour_aggregateLoss_spec__0(x_2, x_1);
return x_3;
}
}
LEAN_EXPORT lean_object* lp_SourVerification_Sour_aggregateLoss___boxed(lean_object* x_1) {
_start:
{
lean_object* x_2; 
x_2 = lp_SourVerification_Sour_aggregateLoss(x_1);
lean_dec(x_1);
return x_2;
}
}
lean_object* initialize_Init(uint8_t builtin);
static bool _G_initialized = false;
LEAN_EXPORT lean_object* initialize_SourVerification_SourVerification_Sour_NoBadDebt(uint8_t builtin) {
lean_object * res;
if (_G_initialized) return lean_io_result_mk_ok(lean_box(0));
_G_initialized = true;
res = initialize_Init(builtin);
if (lean_io_result_is_error(res)) return res;
lean_dec_ref(res);
return lean_io_result_mk_ok(lean_box(0));
}
#ifdef __cplusplus
}
#endif
