--!strict
-- A volcano panel's eruption (attack object #7, `sub_80C5A34`): a pillar of fire on the panel whose collision turns on
-- for one stretch of its life, cut short once it has hit something. The volcano's `tick` spawns it (`sub_80C5B76`) when
-- the field's cycle comes to the panel's side's count. docs/engine/field-collision-damage.md.

local collision = require("../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../lib/sparks")

local eruption = {}

export type State = {
    -- Its first parameter (Param1), which its lifetime adds to.
    lifetime: number,
}

local SPRITE = asset.sprite("eruption")
-- Added to its first parameter for its lifetime.
local LIFETIME = 0x28
-- What a volcano's spawn starts it with (`sub_80C5B76`), and the damage word it takes (50).
local START, DAMAGE = 0x28, 0x32
-- The field's cycle counts when the volcanos of the left side's columns (1 to 3) erupt, and the right side's.
local LEFT_AT, RIGHT_AT = 0x8C, 0x46
-- Bodies on a panel (the flags word's: 0x03800000).
local BODIES = 0x03800000

local function state(me: Object): State
    return me.state :: State
end

-- `sub_80C5B40`: 40 ticks before the end the collision comes on (and it rises to the panel), with the rising
-- animation; then the pillar; the fall at the end.
local function by_timer(me: Object)
    local t, start = me.timer, state(me).lifetime - LIFETIME
    local anim
    if t == start then
        me.collision:set_region(regions.single)
        local p = me.pos
        me.pos = Vec3.new(p.x, p.y + 0x10000, 0)
        anim = 1
    elseif t == start - 2 then
        anim = 2
    elseif t == 2 then
        anim = 1
    else
        return
    end
    me.anim = anim
end

-- `sub_80C5AD4`: the last tick's hits resolved; once it hit something (or its panel stops being a volcano with the
-- collision on: the volcano's flag 0x1000), the collision goes off and it ends 3 ticks later (hidden where a body
-- stands); its animation and collision follow the timer.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me.lifecycle = "destroy"
        c:present()
        return
    end
    local x, y = me.panel_x, me.panel_y
    local panel = field.panel(x, y)
    local ending
    if not (panel and panel.kind == "volcano") then
        ending = c.region ~= nil
        if ending then
            me.timer = 3
        end
    else
        ending = c.hit_flags ~= 0
    end
    if ending then
        c:set_region(nil)
        if bit32.band(field.flags(x, y), BODIES) ~= 0 then
            me.visible = false
        end
    end
    local left = me.timer - 1
    me.timer = bit32.band(left, 0xFFFF)
    if left <= 0 then
        c:set_region(nil)
        me.lifecycle = "destroy"
    else
        by_timer(me)
    end
    c:present()
end

-- `sub_80C5A58`: on its panel, one pixel back and down, with its collision (off for now); then its first tick.
local function init(me: Object)
    if not field.valid(me.panel_x, me.panel_y) then
        me:free()
        return
    end
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "with_sprite"
    sprite.palette = 0
    sprite:set_animation(0)
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    local s = state(me)
    s.lifetime = bit32.band(s.lifetime + LIFETIME, 0xFF)
    me.timer = s.lifetime
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y - 0x10000, -0x10000)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.hitbox, collision.hits_both_sides, 1)
    local c = me.collision
    c:set_hit_effect(sparks.fire)
    c:set_region(nil)
    c:present()
    me.lifecycle = "update"
    tick(me)
end

eruption.kind = new.kind {
    id = "eruption",
    pool = "attack",
    state = { lifetime = "u8" },
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        if me.active then
            me:update_sprite()
        end
    end,
}

-- The volcano's `tick` (`sub_800C380`'s volcano arm): when the field's cycle comes to its side's count it erupts
-- (`sub_80C5B76`: the eruption on the panel, its position the spawner's registers until its init places it).
function eruption.volcano(x: number, y: number)
    if field.cycle() ~= (if x <= 3 then LEFT_AT else RIGHT_AT) then
        return
    end
    local o = battle.spawn(eruption.kind, Vec3.new(y, 0, 0))
    if not o then
        return
    end
    o.panel_x, o.panel_y = x, y
    o.element = 0
    o.damage = DAMAGE
    o.stamina = 0
    state(o).lifetime = START
end

return eruption
