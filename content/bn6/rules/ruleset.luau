--!strict
-- BN6's stock rules (docs/design/rules-in-luau.md §2.2): the systems a BN6
-- player plays by, in the order the framework calls them. The framework
-- (Rust) still runs every BN6 rule; the systems take them over slice by
-- slice (§8).

return define.ruleset {
    id = "bn6",
    stock = true,
    systems = {},
}
