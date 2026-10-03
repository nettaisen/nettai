--!strict
-- DrkSonic (BN5 chip 0x0c6): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "bn5:drksonic",
    codes = { "E", "*" },
    element = "null",
    family = "summon",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "dimming", "dark" },
    slot_in_limit = 1,
}
