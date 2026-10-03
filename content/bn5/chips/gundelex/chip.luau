--!strict
-- GunDelEX (BN5 chip 0x0db): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:gundelex",
    codes = { "G" },
    element = "null",
    family = "null",
    class = "mega",
    rarity = 4,
    mb = 80,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "library" },
    extra_flags = { 1 },
    slot_in_limit = 1,
    megaman = "light",
}
