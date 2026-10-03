--!strict
-- DblPoint (BN5 chip 0x0cf): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:dblpoint",
    codes = { "*" },
    element = "null",
    family = "plus",
    class = "mega",
    rarity = 3,
    mb = 50,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
}
