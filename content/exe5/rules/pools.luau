--!strict
-- The object pools' sizes: EXE5 holds 16 actors, 32 attacks and 32 effects (docs/design/exe5-map.md §0). A
-- battle takes each pool's larger size of its two players' games (docs/design/rules-in-luau.md §2.3).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

return { actor = 16, attack = 32, effect = 32 }
