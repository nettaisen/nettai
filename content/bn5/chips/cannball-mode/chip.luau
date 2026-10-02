--!strict
-- CannBall (BN5 chip 0x11a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "cannball-mode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 30,
    damage = 130,
    hit_param = 30,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
}
