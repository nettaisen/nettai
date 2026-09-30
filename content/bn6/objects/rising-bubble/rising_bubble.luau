--!strict
-- The rising bubble (effect object #0x14, `sub_80E1CF8`): a bubble that
-- floats up two pixels a tick while its animation plays once, then goes.
-- Only drawn. Anubis and PoisPhar's statue (objects/anubis) sends them up
-- over the poisoned area; Guardian's and Geddon's objects use it too.

local rising_bubble = {}

local SPRITE = "14-02"
local ANIM = 1
-- Up two pixels a tick.
local RISE = 0x2_0000

-- `sub_80E1D7A`: a bubble at `pos` (the spawner's r1 to r3) in sprite
-- palette `palette` (r4, whose low byte is the object's Param1).
function rising_bubble.spawn(pos: Vec3, palette: number): Object?
    return battle.spawn_kind("rising-bubble", pos, { palette })
end

-- `sub_80E1D54`: step the animation and rise; gone once its last frame
-- ends.
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
    sprite.shadow = "hidden"
    me.anim, me.anim_loaded = ANIM, ANIM
    sprite:set_animation(ANIM)
    sprite.palette = me:param(1)
    me.vel = Vec3.new(me.vel.x, me.vel.y, RISE)
    me.lifecycle = "update"
    rise(me)
end

-- `sub_80E1CF8`.
function rising_bubble.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        rise(me)
    else
        -- `sub_80E1D72`: object_freeMemory.
        me:free()
    end
end

return rising_bubble
