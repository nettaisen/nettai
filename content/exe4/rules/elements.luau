--!strict
-- Element weakness: the extra damage multiplier by the receiver's element, then the hitter's (null, fire,
-- aqua, elec, wood, drain; the 28 bytes at 0x08012CA4, EXE5's values). No family adds a secondary element (to
-- read: exe4-map.md §18).
-- Written by tools/exe4/gen_rules.py (bn6battle-verify) from the EXE4 ROM; people own it now.

return {
    weakness = {
        null = { 0, 0, 0, 0, 0, 0 },
        fire = { 0, 0, 1, 0, 0, 0 },
        aqua = { 0, 0, 0, 1, 0, 0 },
        elec = { 0, 0, 0, 0, 1, 0 },
        wood = { 0, 1, 0, 0, 0, 0 },
        drain = { 0, 0, 0, 0, 0, 0 },
    },
}
