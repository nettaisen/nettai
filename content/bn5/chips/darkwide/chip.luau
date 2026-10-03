--!strict
-- DarkWide (BN5 chip 0x0c0): a dark chip: WideSht's shot (@bn6/chips/widesht) with BN5's shooter and a wave of kind 9
-- (objects/wave: it goes through and trails), one wave (its subtype's count, 0x080ED008). Its use costs buster
-- blanks (rules/light-dark).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("@bn6/objects/attachment/attachment")
local wave = require("../../objects/wave/variants")
local widesht = require("@bn6/chips/widesht/action")

-- BN5's wide shooter (its attachment row 0x11).
local SHOOTER = attachment.look { sprite = asset.sprite("bn5:wide-shooter"), palette = 0, attach_point = 12 }

return define.chip {
    id = "bn5:darkwide",
    codes = { "T" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 4,
    mb = 99,
    damage = 300,
    hit_param = 158,
    lockout = 0,
    flags = { "has_damage", "dark" },
    slot_in_limit = 1,
    action = widesht.action { id = "bn5:darkwide/action", wave = wave.dark, waves = 1, shooter = SHOOTER, z = 0 },
}
