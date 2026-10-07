--!strict
-- AirShot (EXE5 chip 0x004): a shot from the panel ahead that pushes what it hits: the shared AirShot
-- (@exelib/airshot, EXE6's action 0x21, EXE5's 0x1D; the same code but for the shooter's attachment row, EXE5's 8,
-- EXE6's 0x13, the same look), with EXE5's air shooter, attachment, projectile and shot (no wind element: EXE5's variant
-- row 4); its sound EXE6's number.
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local airshot = require("@exelib/airshot")
local attachment = require("../objects/attachment")
local navi_arms = require("../lib/navi_arms")
local projectile = require("../lib/projectile")
local variants = require("../objects/projectile/variants")

-- EXE5's air shooter (its attachment row 8: EXE6's row 0x13's look, EXE5's sprite).
local SHOOTER = attachment.look { sprite = asset.sprite("air-shooter"), palette = 0, attach_point = 10 }

local chip: Chip = {
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
    action = airshot.action({ shooter = SHOOTER, shot = variants.air, recovery = 10 }, {
        sound = asset.sound("h-burst"),
        -- EXE6's `sub_80EC934`: the shooter by the holder's AI index.
        hold = attachment.holder(navi_arms.air_shooter),
        projectiles = projectile,
        anim = 9,
        slot = "overlay",
    }),
    auto_battle = { class = "into_reach", range = "ahead_in_row" },
}

return { airshot = chip }
