--!strict
-- Pulsar2 (BN5 chip 0x016): a pulse along the row that bursts around the first body it hits, paralyzing (BN5's
-- Pulsar, objects/pulsar: action 0x52).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local pulsar = require("../../objects/pulsar/pulsar")

return define.chip {
    id = "bn5:pulsar2",
    codes = { "E", "G", "M" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 90,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = pulsar.action,
}
