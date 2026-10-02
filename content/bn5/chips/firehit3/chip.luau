--!strict
-- FireHit3 (BN5 chip 0x099): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:firehit3",
    codes = { "E", "F", "G" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 44,
    damage = 140,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
