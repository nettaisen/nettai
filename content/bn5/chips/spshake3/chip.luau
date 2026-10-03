--!strict
-- SpShake3 (BN5 chip 0x01a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:spshake3",
    codes = { "S", "T", "U" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 50,
    damage = 100,
    hit_param = 20,
    lockout = 20,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
