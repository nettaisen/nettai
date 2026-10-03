--!strict
-- Pulsar1 (BN5 chip 0x015): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:pulsar1",
    codes = { "F", "P", "T" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 14,
    damage = 70,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
}
