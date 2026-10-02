--!strict
-- DarkPlus (BN5 chip 0x0be): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:darkplus",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 50,
    hit_param = 0,
    lockout = 20,
    flags = { "dark" },
    slot_in_limit = 1,
}
