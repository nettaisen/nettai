--!strict
-- Geyser (BN5 chip 0x02d): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "geyser",
    codes = { "G", "M", "S" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 3,
    mb = 38,
    damage = 200,
    hit_param = 40,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
