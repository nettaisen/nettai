--!strict
-- BN6's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN6
-- player plays by, in the order the framework calls them. The framework
-- (Rust) still runs most BN6 rules; the systems take them over slice by
-- slice (§8).

local patch_cards = require("./patch-cards/system")

return define.ruleset {
    id = "bn6",
    stock = true,
    systems = {
        -- The Japanese games' patch cards (docs/engine/patch-cards.md).
        patch_cards,
    },
}
