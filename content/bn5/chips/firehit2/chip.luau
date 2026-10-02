--!strict
-- FireHit2 (BN5 chip 0x098): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "firehit2",
    codes = { "H", "I", "J" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 28,
    damage = 100,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
