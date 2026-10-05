--!strict
-- Banners that hold until removed (types 2 and 4 of the 49 banner records at 0x0801B810, EXE6's
-- `pt_801EF84`).
-- Written by tools/exe5/gen_rules.py (bn6battle-verify) from the EXE5 ROM; people own it now.

return {
    holding = {
        asset.banner("program-advance"),
        asset.banner("hit-damage-judge"),
        asset.banner("program-advance-empty"),
    },
}
