--!strict
-- The object pools' sizes: BN6 holds 32 actors, 32 attacks and 32 effects
-- (`eT1BattleObject0`, `eT3BattleObject0`, `eT4BattleObject0`; docs/engine/
-- objects-and-player.md §1). A battle takes each pool's larger size of its two
-- players' games (docs/design/rules-in-luau.md §2.3: a capacity-only limit).

return define.rules("bn6:pools", { actor = 32, attack = 32, effect = 32 })
