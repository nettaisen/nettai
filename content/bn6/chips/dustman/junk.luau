--!strict
-- DustMan's junk (the original's attack object #0xB3, `sub_80DBB40`): a
-- field object he took, thrown back: it looks as the object's junk look
-- says (`sub_800F26C`; a look that is the table's none isn't thrown), and
-- flies from in front of his panel, 6 pixels a tick, to the far edge,
-- hitting until it hits something. It acts and animates while dimmed. See
-- docs/engine/chips.md §3.6.26.

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local junk = {}

export type State = {
    -- Its look (`obstacle.junk_look` of the object it was, ExtraVars[0]).
    look: number,
}

-- How far in front of its panel's centre it starts, how fast it flies
-- (16.16); its height, in whole pixels.
local AHEAD, SPEED, HEIGHT = 0x12_0000, 0x6_0000, 0x10

-- `sub_80DBB64`: the object's look, in front of its panel at 16 pixels
-- (Z's whole part: the fraction is the spawner's register), collision on;
-- gone at once if it has no look (or collision can't be had).
local function init(me: Object)
    local s = me.state :: State
    if not me:wear_junk_look(s.look) then
        me:free()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    local z = bit32.bor(bit32.lshift(HEIGHT, 16), bit32.band(p.z, 0xFFFF))
    me.pos = Vec3.new(p.x + me.facing * AHEAD, p.y, z)
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.breaking, collision.hits_navis, 3)
    me.collision:set_hit_effect(sparks.impact)
    me.collision:present()
    me:set_lifecycle("update")
end

-- `sub_80DBC48`: its flight: on the first tick, its velocity and how long
-- it flies to the far edge (`sub_80DBCBA`: a BIOS division); then forward,
-- its panel following; it hits on until its time is out (off the field its
-- region goes).
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.vel = Vec3.new(me.facing * SPEED, me.vel.y, me.vel.z)
        local edge = if me.alliance == 0 then 7 else 0
        local ex = field.center(edge, 1)
        me.timer = int.tdiv(math.abs(ex - me.pos.x), SPEED)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x + me.vel.x, p.y, p.z)
    me:set_panel_from_coordinates()
    me:update_collision_panels()
    if me.collision.hit_flags ~= 0 then
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if not field.valid(me.panel_x, me.panel_y) then
        me.collision:set_region(nil)
    end
end

-- `sub_80DBC0A`: resolve the hits; the battle's end or a hit ends it.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() or c.hit_flags ~= 0 then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if me.action ~= 0 then
        error(string.format("DustMan's junk action %#x reads past its table (off_80DBC44)", me.action))
    end
    fly(me)
    c:present()
end

junk.kind = define.kind {
    id = "dustman/junk",
    pool = "attack",
    state = { look = "u16" },
    -- `sub_80DBB40`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80DBC90`: junk of look `look`, thrown by `owner` from (x, y) with
-- `element` and the damage word; it runs while dimmed (flag 0x10). `z` is
-- the spawner's r3 (DustMan's: the object loop's, `battle.loop_register`),
-- which becomes its Z, of which the init keeps the fraction.
function junk.throw(owner: Object, x: number, y: number, element: number, damage: number, look: number, z: number): Object?
    local o = battle.spawn(junk.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.related1 = owner
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        (o.state :: State).look = look
    end
    return o
end

return junk
