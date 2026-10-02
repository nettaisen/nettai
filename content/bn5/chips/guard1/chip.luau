--!strict
-- Guard1 (BN5 chip 0x058): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "guard1",
    codes = { "A", "D", "K", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 0,
    mb = 4,
    damage = 50,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
}
