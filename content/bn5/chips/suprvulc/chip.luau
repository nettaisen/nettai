--!strict
-- SuprVulc (BN5 chip 0x0dc): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "suprvulc",
    codes = { "S" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 3,
    mb = 75,
    damage = 10,
    hit_param = 4,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
}
