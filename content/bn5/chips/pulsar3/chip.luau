--!strict
-- Pulsar3 (BN5 chip 0x017): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:pulsar3",
    codes = { "C", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 110,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
