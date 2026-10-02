--!strict
-- BoyBomb2 (BN5 chip 0x056): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "boybomb2",
    codes = { "H", "P", "W" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 1,
    mb = 40,
    damage = 180,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
