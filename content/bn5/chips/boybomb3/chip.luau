--!strict
-- BoyBomb3 (BN5 chip 0x057): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "boybomb3",
    codes = { "B", "Q", "Y" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 230,
    hit_param = 158,
    lockout = 0,
    flags = { "dimming", "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
