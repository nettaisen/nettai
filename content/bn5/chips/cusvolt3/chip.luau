--!strict
-- CusVolt3 (BN5 chip 0x0a8): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "cusvolt3",
    codes = { "G", "M", "P" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 3,
    mb = 70,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "damage_shown_variable", "library", "variable_damage" },
    slot_in_limit = 3,
}
