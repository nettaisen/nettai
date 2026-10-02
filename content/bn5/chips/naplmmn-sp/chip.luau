--!strict
-- NaplmMnSP (BN5 chip 0x0e4): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:naplmmn-sp",
    codes = { "N" },
    element = "fire",
    family = "fire",
    class = "mega",
    rarity = 3,
    mb = 72,
    hit_param = 168,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
