--!strict
-- VDoll (BN5 chip 0x067): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:vdoll",
    codes = { "O", "T", "Z" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 3,
    mb = 58,
    damage = 10,
    hit_param = 10,
    lockout = 0,
    flags = { "standard_library", "library" },
    extra_flags = { 128 },
    slot_in_limit = 3,
}
