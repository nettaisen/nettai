--!strict
-- The buster's recovery by Rapid, then open panels ahead (0x08018530: EXE5's but its last two rows); the empty
-- hand's chip as EXE5's (EXE4's reading of a chip 0xFFFF waits: exe4-map.md §18).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    recovery = {
        { 4, 8, 12, 16, 20, 24 },
        { 3, 7, 10, 14, 17, 20 },
        { 3, 6, 9, 12, 15, 17 },
        { 2, 5, 7, 10, 13, 15 },
        { 2, 4, 6, 8, 10, 12 },
    },
    empty_hand = { null_family = false, fire = false, flags = {} },
}
