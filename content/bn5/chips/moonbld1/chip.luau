--!strict
-- MoonBld1 (BN5 chip 0x039): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:moonbld1",
    codes = { "E", "N", "Z" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 0,
    mb = 23,
    damage = 90,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
