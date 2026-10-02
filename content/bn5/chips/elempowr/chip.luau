--!strict
-- ElemPowr (BN5 chip 0x121): its common record, written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.
-- No use yet: the BN5 port writes it. It loads with the BN5 root (rules-in-luau.md R).

return define.chip {
    id = "elempowr",
    codes = { "A", "B", "C", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 4,
    mb = 64,
    damage = 0,
    hit_param = 50,
    lockout = 0,
    flags = { "dimming" },
    extra_flags = { 128 },
    slot_in_limit = 1,
}
