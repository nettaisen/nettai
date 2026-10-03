--!strict
-- NeoVari (BN5 chip 0x0d9): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:neovari",
    codes = { "V" },
    element = "null",
    family = "sword",
    class = "mega",
    rarity = 4,
    mb = 74,
    damage = 240,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "library" },
    slot_in_limit = 1,
}
