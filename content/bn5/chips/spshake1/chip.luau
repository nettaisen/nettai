--!strict
-- SpShake1 (BN5 chip 0x018): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "spshake1",
    codes = { "G", "H", "I" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 0,
    mb = 30,
    damage = 60,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
