--!strict
-- A surge of water (attack object #0x76, `sub_80D385C`), SpoutCross
-- Beast's charged chip (action 0x3A): it stands 20 pixels ahead of its
-- panel, on the ground, and hits its panel and the one ahead (region 2)
-- for Param1 ticks; a hit takes its region away until Param2 ticks later.
-- It ebbs (animation 1) with Param1 - 6 ticks left and falls (animation 2)
-- with 8.

local aqua_surge = {}

local SPRITE = "10-2e"

-- How far ahead of its panel's center it stands, 16.16.
local AHEAD = 0x14_0000

-- Collision: what it is and hits, its hit modifier, and its region (its
-- panel and the one ahead).
local SELF_TYPE, TARGET_TYPE, HIT_MOD = 4, 5, 1
local REGION = 2

-- `sub_80D398E`: a surge from `owner` on (x, y): Param1 ticks, its region
-- back every Param2. It takes the owner's side (not its facing). Its
-- position is the registers (y, the element, 0) until its init places it.
function aqua_surge.spawn(owner: Object, x: number, y: number, element: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("aqua-surge", Vec3.new(y, element, 0), params)
    if o then
        o.element = element
        o.panel_x, o.panel_y = x, y
        o.timer = o:param(1)
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance = owner.alliance
    end
    return o
end

-- The end: no region, hidden, destroyed.
local function finish(me: Object)
    me.collision.region = 0
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80D3946`: the animation by the ticks left; the region back every
-- Param2 ticks; the end after Param1.
local function count(me: Object)
    if me.timer == me:param(1) - 6 then
        me.anim = 1
    end
    if me.timer == 8 then
        me.anim = 2
    end
    local t = me.timer - 1
    me.timer = t
    if t == 0 then
        return finish(me)
    end
    local t2 = me.timer2 - 1
    me.timer2 = t2
    if t2 > 0 then
        return
    end
    me.timer2 = me:param(2)
    me.collision.region = REGION
end

-- `sub_80D390A`: resolve the hits (a hit takes the region away), count,
-- and present again (even on its last tick); gone once the battle is over.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        me.visible = false
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    count(me)
    me:update_sprite()
    c:present()
end

-- `sub_80D387C`: ahead of its panel (not off the field), with its
-- collision, and a splash.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "hidden"
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, 0)
    sprite.palette = 0
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    me.collision.hit_effect = 0xFF
    me.timer2 = me:param(2)
    me.collision.region = REGION
    me.collision:present()
    battle.play_sound(0xB8)
    me:set_lifecycle("update")
    tick(me)
end

-- `sub_80D385C`.
function aqua_surge.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return aqua_surge
