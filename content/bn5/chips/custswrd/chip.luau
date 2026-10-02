--!strict
-- CustSwrd (BN5 chip 0x036): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "custswrd",
    codes = { "E", "L", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 20,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 3,
}
