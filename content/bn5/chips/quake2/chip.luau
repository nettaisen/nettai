--!strict
-- Quake2 (BN5 chip 0x024): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "quake2",
    codes = { "B", "L", "Y" },
    element = "null",
    family = "break",
    class = "standard",
    rarity = 1,
    mb = 25,
    damage = 150,
    hit_param = 50,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
