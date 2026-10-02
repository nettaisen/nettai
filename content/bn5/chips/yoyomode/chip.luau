--!strict
-- YoYoMode (BN5 chip 0x11c): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "yoyomode",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 50,
    damage = 50,
    hit_param = 10,
    lockout = 20,
    flags = {},
    extra_flags = { 32 },
    slot_in_limit = 1,
}
