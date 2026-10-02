--!strict
-- SerchMnSP (BN5 chip 0x0e7): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "serchmn-sp",
    codes = { "S" },
    element = "null",
    family = "cursor",
    class = "mega",
    rarity = 3,
    mb = 72,
    hit_param = 133,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
