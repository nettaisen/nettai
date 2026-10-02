--!strict
-- FullCust (BN5 chip 0x0d1): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "fullcust",
    codes = { "*" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 45,
    damage = 0,
    hit_param = 0,
    lockout = 20,
    flags = { "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
}
