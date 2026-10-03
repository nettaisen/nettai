--!strict
-- Spreader (BN5 chip 0x012): a bullet from the panel ahead that bursts over the 3x3 around what it finds: BN6's
-- Spreaders' action (@bn6/chips/spreadr/action, the original's action 0x25, BN5's 0x18 at 0x080EC4CC; the same code
-- but for the tick it fires, a row of 0x080EC57C, the first row 1 as BN6's) with BN5's gun (the attachment table's
-- row 9, 0c-19: BN6's row 0x15) 16 pixels up (0x080EC580), and BN5's bullet (objects/bullet, its record's Param1: row
-- 3).
-- Its record is written by tools/bn5/gen_content.py (bn6battle-verify) from the BN5 ROMs.

local attachment = require("@bn6/objects/attachment/attachment")
local bullet = require("../../objects/bullet/bullet")
local spreader = require("@bn6/chips/spreadr/action")

return define.chip {
    id = "bn5:spreader",
    codes = { "C", "D", "E", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 1,
    mb = 10,
    damage = 30,
    hit_param = 10,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
    slot_in_limit = 3,
    action = spreader.action {
        id = "bn5:spreader/action",
        gun = attachment.look { sprite = asset.sprite("bn5:spreader"), palette = 0, attach_point = 11 },
        z = 0x10_0000,
        bullet = bullet.variants.spreader,
        bullet_kind = bullet.kind,
        sound = asset.sound("bn5:h-burst"),
    },
}
