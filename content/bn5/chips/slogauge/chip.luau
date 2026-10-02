--!strict
-- SloGauge (BN5 chip 0x078): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "slogauge",
    codes = { "H", "Q", "W", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 69,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 1,
}
