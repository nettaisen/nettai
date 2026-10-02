--!strict
-- SuprSpr3 (BN5 chip 0x14b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "suprspr3",
    codes = { "A", "B", "C", "*" },
    element = "aqua",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 140,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
}
