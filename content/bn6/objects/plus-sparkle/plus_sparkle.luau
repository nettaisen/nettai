--!strict
-- The plus chips' sparkle (effect object #0x14, `sub_80E1CF8`): a sparkle
-- rising 2 pixels a tick where a navi used a plus chip or the BeastOut
-- chip, gone after its animation's last frame.

local plus_sparkle = {}

local SPRITE, ANIM = "14-02", 1

-- How fast it rises, 16.16.
local RISE = 0x2_0000

-- `sub_80E1D7A`: a sparkle at `pos`, with palette `palette` (its first
-- parameter).
function plus_sparkle.spawn(pos: Vec3, palette: number): Object?
    return battle.spawn_kind("plus-sparkle", pos, { palette })
end

-- `sub_80E1D54`: rise, until the animation's last frame.
local function rise(me: Object)
    me.sprite:step()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z + me.vel.z)
    if bit32.btest(me.sprite.frame_flags, 0x80) then
        me:set_lifecycle("destroy")
    end
end

-- `sub_80E1D18`: visible, animated, rising; and its first rise at once.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite.palette = me:param(1)
    local v = me.vel
    me.vel = Vec3.new(v.x, v.y, RISE)
    -- (A byte store: only the lifecycle state.)
    me.lifecycle = "update"
    rise(me)
end

-- `sub_80E1CF8`.
function plus_sparkle.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        rise(me)
    else
        me:free()
    end
end

return plus_sparkle
