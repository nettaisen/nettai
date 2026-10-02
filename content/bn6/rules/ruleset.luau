--!strict
-- BN6's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN6
-- player plays by, in the order the framework calls them. The framework
-- (Rust) still runs the BN6 rules no system has taken over yet; the systems
-- take them slice by slice (§8).

local beast = require("./beast/system")
local forms = require("./forms/system")
local navicust = require("./navicust/system")
local patch_cards = require("./patch-cards/system")

return define.ruleset {
    id = "bn6",
    stock = true,
    systems = {
        -- The NaviCust: the programs a player placed, compiled into the
        -- navi's stats (docs/design/navicust.md); before the patch cards,
        -- which apply to what it made.
        navicust,
        -- The Japanese games' patch cards (docs/engine/patch-cards.md).
        patch_cards,
        -- The form changes: the Crosses', Beast Out's and Beast Over's.
        forms.system,
        -- Beast Out's turns.
        beast,
    },
}
