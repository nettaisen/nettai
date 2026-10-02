--!strict
-- BN6's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN6
-- player plays by, in the order the framework calls them. The framework
-- (Rust) still runs the BN6 rules no system has taken over yet; the systems
-- take them slice by slice (§8).

local beast = require("./beast/system")

return define.ruleset {
    id = "bn6",
    stock = true,
    systems = { beast },
}
