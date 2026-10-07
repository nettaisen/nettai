--!strict
-- Element weakness: the extra damage multiplier by the receiver's element, then the hitter's (null, fire,
-- aqua, elec, wood, drain; the 28 bytes at 0x08016900 EXE5's weakness routine reads, hitter 5 reading the
-- next receiver's first): EXE6's values. The chip families by the numbers its pack has their custom-screen icons
-- by: EXE6's, with EXE5's recovery and invisible after them (its records' family byte, +6, numbers them its own
-- way: fire 0, aqua 1, elec 2, wood 3, recovery 4, plus 5, sword 6, invisible 7, cursor 8, summon 9, wind 10,
-- break 11, null 12, program advance 13, special 14; nettai-extract lays the icons out by these); null the
-- non-elemental one. No family adds a secondary element: EXE5 stores a chip's element as its record has it
-- (0x0800FE72, EXE6's `sub_80126E4` without `byte_80129E4`).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

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
        plus = 4,
        sword = 5,
        cursor = 6,
        summon = 7,
        wind = 8,
        ["break"] = 9,
        null = 10,
        program_advance = 11,
        special = 12,
        recovery = 13,
        invisible = 14,
    },
    non_elemental = "null",
}
