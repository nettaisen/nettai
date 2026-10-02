--!strict
-- TimeBom1 (BN5 chip 0x04b): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:timebom1",
    codes = { "H", "N", "T" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 0,
    mb = 32,
    damage = 100,
    hit_param = 148,
    lockout = 30,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
