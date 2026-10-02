--!strict
-- Element weakness: the extra damage multiplier by the receiver's element, then the hitter's (null, fire,
-- aqua, elec, wood, drain; the 28 bytes at 0x08016900 BN5's weakness routine reads, hitter 5 reading the
-- next receiver's first): BN6's values. No family adds a secondary element: BN5 stores a chip's element
-- as its record has it (0x0800FE72, BN6's `sub_80126E4` without `byte_80129E4`).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

return define.rules("elements", {
    weakness = {
        null = { 0, 0, 0, 0, 0, 0 },
        fire = { 0, 0, 1, 0, 0, 0 },
        aqua = { 0, 0, 0, 1, 0, 0 },
        elec = { 0, 0, 0, 0, 1, 0 },
        wood = { 0, 1, 0, 0, 0, 0 },
        drain = { 0, 0, 0, 0, 0, 0 },
    },
})
