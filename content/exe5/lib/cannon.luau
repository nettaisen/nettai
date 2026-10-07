--!strict
-- EXE5's cannons' action (the original's action 0x14, EXE6's code: @exelib/cannon) with EXE5's attachment and
-- projectile, its sound (0xAE) and GigaCan's third afterimage in Hub Style. Each chip states its own arm (Cannon's,
-- HiCannon's and M-Cannon's the attachment table's rows 0 to 2: sprite 0c-01 in palettes 0 to 2) and shot.

local attachment = require("../objects/attachment")
local common = require("@exelib/cannon")
local navi_arms = require("./navi_arms")
local projectile = require("./projectile")

local cannon = {}

-- GigaCan's third afterimage (0x080EC224): in Hub Style (NaviStats +0x4C of the user's side), its shade (0c-58) in the
-- navi's animation 8, where the navi stands, without a shadow.
local HUB_SHADE = asset.sprite("hub-shade")
local function hub_afterimage(me: Object, flip: number)
    if battle.navi(me.alliance).hub_style == 0 then
        return
    end
    battle.afterimage(me, me.pos, { sprite = HUB_SHADE, anim = 8, flip = flip, lifetime = 0x1E, color_shader = 0x8318 })
end

local LOOK: CannonLook = {
    sound = asset.sound("cannon"),
    -- EXE6's `sub_80EBD68`: a player's arm by its AI index, anyone else's the plain one.
    hold = attachment.holder(navi_arms.cannon, true),
    projectiles = projectile,
    -- The navi raises the cannon in its animation 8 and lowers it in 7 (0x080EC14C, as EXE6's).
    raise_anim = 8,
    lower_anim = 7,
    more_afterimages = hub_afterimage,
}

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
