--!strict
-- Element weakness: the extra damage multiplier by the receiver's element, then the hitter's (null, fire,
-- aqua, elec, wood, drain; the 28 bytes at 0x08012CA4, EXE5's values). The chip families by the chip record's
-- byte +0x07 (which picks the custom screen's element icon, 0x0801F674; Double Soul's offer reads it as a soul's
-- kind, 0x08020008): 13 has no chip; null is the non-elemental one. No family adds a secondary element (to read:
-- exe4-map.md §18).
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
    families = {
        fire = 0,
        aqua = 1,
        elec = 2,
        wood = 3,
        recovery = 4,
        plus = 5,
        sword = 6,
        invisible = 7,
        ["break"] = 8,
        summon = 9,
        wind = 10,
        metal = 11,
        null = 12,
        program_advance = 14,
        -- The souls' own chips (0x160 on), as EXE6's special family.
        special = 15,
    },
    non_elemental = "null",
}
