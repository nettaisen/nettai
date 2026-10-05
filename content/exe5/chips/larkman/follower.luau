--!strict
-- What carries a powered swoop's damage (EXE5's attack object #0xCB, 0x080DFB94; EXE5's own): a hit with no sprite
-- that its swoop (./swoop) moves to where its body is each tick, its hit on the panel under it, gone with its swoop.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local regions = require("@exelib/regions")

local follower = {}

local HIT_MOD = 3

-- 0x080DFBB8: panel (0, 0), its hit nowhere until it moves onto the field.
local function init(me: Object)
    me.panel_x, me.panel_y = 0, 0
    me:set_coordinates_from_panel()
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, HIT_MOD)
    me.collision:set_hit_effect(sparks.plain)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
end

-- 0x080DFBF0: resolve the hits; its hit on the panel it is over; gone with its swoop (or the battle).
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        c:set_region(nil)
    end
    if not field.valid(me.panel_x, me.panel_y) then
        c:set_region(nil)
    end
    local x, y = me.panel_x, me.panel_y
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if field.valid(me.panel_x, me.panel_y) and (me.panel_x ~= x or me.panel_y ~= y) then
        c:set_region(regions.single)
    end
    -- 0x080DFC84.
    local swoop = me.related1
    if swoop and swoop.lifecycle == "destroy" then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    c:present()
end

follower.kind = define.kind {
    id = "larkman/follower",
    pool = "attack",
    -- 0x080DFB94.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite()
    end,
}

-- 0x080DFC60: the follower of `swoop`, with its element and damage word.
function follower.spawn(swoop: Object, element: number, damage: number): Object?
    local o = battle.spawn(follower.kind, Vec3.zero)
    if o then
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = swoop.alliance, swoop.flip
        o.related1 = swoop
        o.run_while_dimmed = true
    end
    return o
end

return follower
