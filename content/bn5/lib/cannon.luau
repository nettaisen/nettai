--!strict
-- BN5's cannons' action (the original's action 0x14, BN6's code: @exelib/cannon) with BN5's attachment and
-- projectile, its arms (the attachment table's rows 0 to 2: sprite 0c-01 in palettes 0 to 2), its sound (0xAE) and
-- GigaCan's third afterimage in Hub Style.

local attachment = require("../objects/attachment/attachment")
local common = require("@exelib/cannon")
local projectile = require("./projectile")

local cannon = {}

local CANNON = asset.sprite("cannon")
cannon.arms = {
    cannon = attachment.look { sprite = CANNON, palette = 0, attach_point = 7 },
    hicannon = attachment.look { sprite = CANNON, palette = 1, attach_point = 7 },
    m_cannon = attachment.look { sprite = CANNON, palette = 2, attach_point = 7 },
}

-- GigaCan's third afterimage (0x080EC224): in Hub Style (NaviStats +0x4C of the user's side), its shade (0c-58) in the
-- navi's animation 8, where the navi stands, without a shadow.
local HUB_SHADE = asset.sprite("hub-shade")
local function hub_afterimage(me: Object, flip: number)
    if battle.navi(me.alliance).hub_style == 0 then
        return
    end
    battle.afterimage(me, me.pos, { sprite = HUB_SHADE, anim = 8, flip = flip, lifetime = 0x1E, color_shader = 0x8318 })
end

local LOOK: ShooterLook = {
    sound = asset.sound("cannon"),
    attachment = attachment.kind,
    projectiles = projectile,
    more_afterimages = hub_afterimage,
}

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
