--!strict
-- GrabRvng (BN5 chip 0x076): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "grabrvng",
    codes = { "F", "N", "P" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 3,
    mb = 48,
    damage = 40,
    hit_param = 138,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    slot_in_limit = 1,
}
