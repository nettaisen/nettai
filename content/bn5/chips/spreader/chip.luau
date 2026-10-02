--!strict
-- Spreader (BN5 chip 0x012): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "spreader",
    codes = { "C", "D", "E", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 30,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
