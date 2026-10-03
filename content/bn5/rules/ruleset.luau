--!strict
-- BN5's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN5 player plays by: light and dark
-- MegaMan so far; Soul Unison, Chaos Unison, BN5's emotions and its Team Battle come as systems
-- (docs/design/bn5-map.md §15).
-- Written by tools/bn5/gen_rules.py (bn6battle-verify) from the BN5 ROM; people own it now.

local light_dark = require("./light-dark/system")

return define.ruleset {
    id = "bn5:stock",
    stock = true,
    systems = {
        -- Light and dark MegaMan: the holy panels a dark one clears, the chips each may use.
        light_dark,
    },
}
