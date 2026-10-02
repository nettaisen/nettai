--!strict
-- Slasher (BN5 chip 0x038): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:slasher",
    codes = { "B", "I", "R" },
    element = "null",
    family = "sword",
    class = "standard",
    rarity = 2,
    mb = 18,
    damage = 240,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
