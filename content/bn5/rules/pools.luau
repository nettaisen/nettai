--!strict
-- The object pools' sizes: BN5 holds 16 actors, 32 attacks and 32 effects (docs/design/bn5-map.md §0). A
-- battle takes each pool's larger size of its two players' games (docs/design/rules-in-luau.md §2.3).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

return define.rules("bn5:pools", { actor = 16, attack = 32, effect = 32 })
