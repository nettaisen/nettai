--!strict
-- The Vulcans' action (the original's action 0x17, `sub_80EBF10`;
-- docs/engine/shot-chips.md §4): the shared action
-- (@exelib/vulcan/action) with EXE6's sound, vulcan, attachment and bullet.

local attachment = require("../../objects/attachment")
local bullet = require("../../objects/bullet")
local common = require("@exelib/vulcan/action")
local regions = require("@exelib/regions")

local vulcan = {}

-- One state for the four vulcans.
vulcan.STATE = common.STATE

-- The bullets: the Vulcans' (the bullet's row 0xC) and SuprVulc's (0x10, its
-- hit brighter): the panel found and the one behind it, staggering.
vulcan.bullets = {
    plain = bullet.variant { region = regions.long, hit_mod = 1, effect = bullet.hit, single = true },
    super = bullet.variant {
        region = regions.long,
        hit_mod = 1,
        effect = new.effect { sprite = asset.sprite("vulcan-hit"), anim = 0, palette = 1 },
        single = true,
    },
}

local LOOK: VulcanLook = {
    sound = asset.sound("col-army-2"),
    -- The vulcan (the original's attachment row 0xD).
    arm = attachment.look { sprite = asset.sprite("vulcan"), palette = 0, attach_point = 14 },
    attachment = attachment.kind,
    bullet_kind = bullet.kind,
}

-- A chip's vulcan.
function vulcan.action(spec: VulcanSpec): Action
    return common.action(spec, LOOK)
end

return vulcan
