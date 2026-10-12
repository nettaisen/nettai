--!strict
-- EXE4's cannons' action (the chips' action 0x0B, 0x080EB854): EXE6's and EXE5's (@exelib/cannon) with EXE4's look:
-- its sound (0x8C); the navi holds the cannon in its related slot as EXE4's own attachment, its animation the navi's
-- number (NaviStats +0x23, MegaMan's 0); its projectile; the navi raises it in animation 9 and lowers it in 8; and a
-- GigaCan's afterimages its own (0x080EB8BE). Each chip composes its own (`cannon.action { arm = ..., shot = ... }`),
-- stating its cannon (its variant's attachment row, 0x080EB9B4: the cannon sprite 0c-01 in a palette) and its shot
-- (a projectile row, 0x080EB9BB).

local attachment = require("../objects/attachment")
local common = require("@exelib/cannon")
local projectile = require("./projectile")

local cannon = {}

-- One state for every cannon.
cannon.STATE = common.STATE

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0
-- The afterimages' color shader and lifetime.
local AFTERIMAGE_SHADER, AFTERIMAGE_TICKS = 0x8318, 0x1E

-- 0x080EB8BE: a GigaCan's afterimages (action 0x0B's variants 4 to 6: chips/gigacan), on the action's eighth tick:
-- the navi in its animation 9, its shadow on the ground (0x080E5020's r7 bytes 2 and 3 both 1: the afterimage's
-- init, 0x080E4FAA, keeps the shadow it gets; byte 2 alone, KendoMan's afterimages', takes it away again), then its
-- cannon where the navi stands (both 0: its shadow at its height), then, in a soul that wears something, that in the
-- navi's animation 9 (0x080EB92A: 0x08018068 by the soul, FireSoul's flames and NumberSoul's overlay; the form's
-- `wears`), its shadow at its height too.
local function afterimages(me: Object, spec: CannonSpec, flip: number)
    local form = battle.navi(me.alliance).form
    -- (Of the navi's sprite in its form, 0x0800B90A.)
    battle.afterimage(me, me.pos, {
        sprite = form.sprite,
        anim = 9,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
        shadow = "ground",
    })
    battle.afterimage(me, me.pos, {
        sprite = spec.arm.sprite,
        anim = NAVI_NUMBER,
        flip = flip,
        lifetime = AFTERIMAGE_TICKS,
        color_shader = AFTERIMAGE_SHADER,
    })
    local worn: SpriteAsset? = form.wears
    if worn then
        battle.afterimage(me, me.pos, {
            sprite = worn,
            anim = 9,
            flip = flip,
            lifetime = AFTERIMAGE_TICKS,
            color_shader = AFTERIMAGE_SHADER,
        })
    end
end

local LOOK: CannonLook = {
    sound = asset.sound("cannon"),
    -- 0x080EB89E: EXE4's own attachment in the slot the action names, its animation the navi's number.
    hold = function(me: Object, look: AttachmentLook, which: Slot)
        attachment.attach(me, look, which, NAVI_NUMBER)
    end,
    projectiles = projectile,
    -- The navi raises the cannon in its animation 9 and lowers it in 8 (0x080EB87C, 0x080EB9D0).
    raise_anim = 9,
    lower_anim = 8,
    afterimages = afterimages,
}

-- A chip's cannon.
function cannon.action(spec: CannonSpec): Action
    return common.action(spec, LOOK)
end

return cannon
