--!strict
-- CloudMnSP (BN5 chip 0x10b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:cloudmn-sp",
    codes = { "C" },
    element = "elec",
    family = "elec",
    class = "mega",
    rarity = 3,
    mb = 74,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "has_damage", "navi", "library" },
    slot_in_limit = 1,
    megaman = "light",
}
