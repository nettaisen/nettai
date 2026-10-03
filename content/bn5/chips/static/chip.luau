--!strict
-- Static (BN5 chip 0x01b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:static",
    codes = { "N", "S", "V" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 55,
    damage = 20,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    megaman = "dark",
}
