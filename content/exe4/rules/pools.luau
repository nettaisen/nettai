--!strict
-- The object pools' sizes: EXE4 holds 8 actors, 32 attacks and 32 effects (the pool table, 0x08003444;
-- exe4-map.md §3.1). A battle takes each pool's larger size of its two players' games.
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return { actor = 8, attack = 32, effect = 32 }
