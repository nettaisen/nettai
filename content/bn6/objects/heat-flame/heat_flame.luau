--!strict
-- A flame (attack object #0x26, `sub_80C8C74`): it flares up on its panel
-- (5 ticks), burns hitting it (Param2 - 5 ticks, its region gone once it
-- has hit something), and dies down (2 ticks). With Param3 set it counts as
-- a different attacker (self collision type 0xA, else 4). HeatMan's (Param1
-- 4) act and step their sprite while dimmed; others wait a dimming out,
-- collision on. See docs/engine/chips.md §3.6.15.

local heat_flame = {}

local SPRITE = "10-02"

-- `sub_80C8DE0`: a flame on (x, y) from `owner` with `element`, the damage
-- word and `params` {Param1, ticks, Param3}. `z` is the spawner's register
-- that becomes its Z, whose fraction it keeps.
function heat_flame.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("heat-flame", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80C8CA4`: on its panel on the ground, collision on.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, bit32.band(p.z, 0xFFFF))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(if me:param(3) ~= 0 then 0xA else 4, 5, 3)
    me.collision.hit_effect = 1
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80C8D64`: flaring up, 5 ticks (sound 0xF7).
local function flare(me: Object)
    if me.phase == 0 then
        me.phase = 4
        battle.play_sound(0xF7)
        me.timer = 5
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80C8D8E`: burning, Param2 - 5 ticks; then its region goes.
local function burn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 1
        me.timer = me:param(2) - 5
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    me.collision.region = 0
    me:set_action(8)
end

-- `sub_80C8DBC`: dying down, 2 ticks.
local function die_down(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.anim = 2
        me.timer = 2
        return
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_lifecycle("destroy")
    end
end

local ACTIONS = { [0] = flare, [4] = burn, [8] = die_down }

-- `sub_80C8D10`: resolve the hits (its region goes once it hit), act
-- (outside a dimming unless it is HeatMan's), register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c.region = 0
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c.region = 0
    end
    if me:param(1) == 4 or not battle.dimmed() then
        local action = ACTIONS[me.action]
        if not action then
            error(string.format("flame action %#x reads past its table (off_80C8D58)", me.action))
        end
        action(me)
    end
    c:present()
end

-- `sub_80C8C74`.
function heat_flame.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(1) == 4 then
        me:update_sprite_while_dimmed()
    else
        me:update_sprite_while_paused()
    end
end

return heat_flame
