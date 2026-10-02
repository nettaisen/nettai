--!strict
-- NrthWind (BN5 chip 0x08a): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "nrthwind",
    codes = { "D", "Q", "V", "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 2,
    mb = 33,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
}
