--!strict
-- Astroid2 (BN5 chip 0x069): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:astroid2",
    codes = { "E", "M", "U" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 42,
    damage = 70,
    hit_param = 138,
    lockout = 60,
    flags = { "has_damage", "library" },
    slot_in_limit = 3,
}
