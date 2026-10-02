--!strict
-- LifeAur (BN5 chip 0x0c9): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "lifeaur",
    codes = { "D" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 70,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
}
