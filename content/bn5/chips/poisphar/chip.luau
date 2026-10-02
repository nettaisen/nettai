--!strict
-- PoisPhar (BN5 chip 0x156): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "poisphar",
    codes = {},
    element = "null",
    family = "program_advance",
    class = "program_advance",
    rarity = 0,
    mb = 5,
    damage = 1,
    hit_param = 128,
    lockout = 0,
    flags = { "dimming", "library" },
    slot_in_limit = 1,
}
