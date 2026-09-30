--!strict
-- A puff of smoke (effect object #0x14, `sub_80E1CF8`): what a thrown
-- BugBomb or BlkBomb leaves when it lands in a hole. It rises 2 pixels a
-- tick while its animation plays, and ends with it. Its palette is its
-- first parameter.

local smoke_puff = {}

local SPRITE, ANIM = "14-02", 1
local RISE = 2 * 0x10000

-- `sub_80E1D7A`: a puff at `pos` with palette `palette`.
function smoke_puff.spawn(pos: Vec3, palette: number): Object?
    return battle.spawn_kind("smoke-puff", pos, { palette, 0, 0, 0 })
end

-- `sub_80E1D54`: rise; done when the animation is.
local function rise(me: Object)
    me.sprite:step()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E1D18`.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.anim, me.anim_loaded = ANIM, ANIM
    sprite:set_animation(ANIM)
    sprite.palette = me:param(1)
    local v = me.vel
    me.vel = Vec3.new(v.x, v.y, RISE)
    me.lifecycle = "update"
    rise(me)
end

-- `sub_80E1CF8`.
function smoke_puff.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        rise(me)
    else
        me:free()
    end
end

return smoke_puff
