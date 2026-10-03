--!strict
-- Astroid1 (BN5 chip 0x068): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:astroid1",
    codes = { "A", "Q", "T" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 0,
    mb = 26,
    damage = 60,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
