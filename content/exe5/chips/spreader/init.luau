--!strict
-- Spreader (EXE5 chip 0x012): a bullet from the panel ahead that bursts over the 3x3 around what it finds: the shared
-- Spreaders' action (@exelib/spreadr/action, EXE6's action 0x25, EXE5's 0x18 at 0x080EC4CC; the same code
-- but for the tick it fires, a row of 0x080EC57C, the first row 1 as EXE6's) with EXE5's gun (the attachment table's
-- row 9, 0c-19: EXE6's row 0x15) 16 pixels up (0x080EC580), and EXE5's bullet (objects/bullet, its record's Param1: row
-- 3).
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local attachment = require("../objects/attachment")
local bullet = require("../objects/bullet")
local navi_arms = require("../lib/navi_arms")
local spreader = require("@exelib/spreadr/action")

local LOOK: SpreaderLook = {
    sound = asset.sound("h-burst"),
    attachment = attachment.kind,
    bullet_kind = bullet.kind,
    gun_anims = navi_arms.spreader,
}

local chip: Chip = {
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
    action = spreader.action({
        gun = attachment.look { sprite = asset.sprite("spreader"), palette = 0, attach_point = 11 },
        z = 0x10_0000,
        bullet = bullet.variants.spreader,
    }, LOOK),
}

return { spreader = chip }
