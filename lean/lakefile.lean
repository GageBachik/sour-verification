import Lake
open Lake DSL

package «SourVerification» where

lean_lib SourVerification where
  roots := #[`SourVerification.Sour.NoBadDebt]

