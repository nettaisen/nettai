--!strict
-- Pulsar3 (BN5 chip 0x017): a pulse along the row that bursts around the first body it hits, paralyzing (BN5's
-- Pulsar, objects/pulsar: action 0x52).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local pulsar = require("../../objects/pulsar/pulsar")

return define.chip {
    id = "bn5:pulsar3",
    codes = { "C", "R", "S" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 38,
    damage = 110,
    hit_param = 20,
    lockout = 30,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = pulsar.action,
}
