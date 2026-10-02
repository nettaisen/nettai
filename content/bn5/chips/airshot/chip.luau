--!strict
-- AirShot (BN5 chip 0x004): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "airshot",
    codes = { "*" },
    element = "null",
    family = "wind",
    class = "standard",
    rarity = 1,
    mb = 6,
    damage = 20,
    hit_param = 20,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
}
