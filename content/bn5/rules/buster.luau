--!strict
-- The buster's recovery by Rapid, then open panels ahead (0x0801CEA4, BN6's values); and what the empty
-- hand's chip reads (the chip record 0xFFFF past the chip table at 0x0801E214).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

return {
    recovery = {
        { 4, 8, 12, 16, 20, 24 },
        { 3, 7, 10, 14, 17, 20 },
        { 3, 6, 9, 12, 15, 17 },
        { 2, 4, 6, 8, 10, 12 },
        { 2, 3, 4, 5, 6, 7 },
    },
    empty_hand = { null_family = false, fire = false, flags = {} },
    -- Chaos Unison's cycle (0x08010650), a row by the chaos level (at most 2): the period, then the bounds of its
    -- windows: a release succeeds under the first (21, 12, 9 ticks into a full charge), fails past it.
    chaos_cycle = {
        { 42, 21, 12, 42 },
        { 36, 12, 9, 36 },
        { 27, 9, 9, 27 },
    },
}
