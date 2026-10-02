--!strict
-- Astroid3 (BN5 chip 0x06a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "astroid3",
    codes = { "C", "P", "Y" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 2,
    mb = 56,
    damage = 80,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
