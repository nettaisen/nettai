--!strict
-- CrsShld's cross (attack object #0xA8, 0x080DB258), the counterattack of the CrsShld chips' guard (../lib/guard): it
-- flies along the row 20 pixels a tick, probing for navis; on one it stops on its panel, and 8 ticks later strikes it
-- once (an attack, hit modifier 3), then goes. Each chip's in its own palette (0, 2 or 4: the original's parameter,
-- times 2).

local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")

local cross = {}

type CrossState = { palette: number }

local CROSS_SPRITE = asset.sprite("cross-shield")
local CROSS_SPEED = 20
local CROSS_SOUND = asset.sound("bite")
-- Its actions.
local FLY, STRIKE, DONE = 0, 4, 8
-- Its probe's spark (row 6, the heavy hit's) and bug (0x080DB2B6: 0xF9, argument 0).
local CROSS_SPARK = sparks.heavy
local CROSS_BUGS = 0x00F9

local function cross_gone(me: Object)
    me.collision:set_region(nil)
    me.visible = false
    me:set_lifecycle("destroy")
end

-- 0x080DB314: flying; on a navi, it snaps onto its panel and waits 8 ticks.
local function cross_fly(me: Object)
    local c = me.collision
    if c.hit_flags ~= 0 then
        me:set_coordinates_from_panel()
        me.anim = 2
        me.timer = 8
        battle.play_sound(CROSS_SOUND)
        me.action, me.phase, me.phase_init = STRIKE, 0, 0
        return
    end
    local p = me.pos
    me.pos = Vec3.new(int.i32(p.x + me.vel.x), p.y, p.z)
    me:set_panel_from_coordinates()
    c.panel_x, c.panel_y = me.panel_x, me.panel_y
end

-- 0x080DB356: after its wait, it becomes an attack (hit modifier 3) for a tick.
local function cross_strike(me: Object)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t > 0 then
        return
    end
    me:setup_collision(collision.attack, collision.hits_navis, 3)
    me.timer = 4
    me.action, me.phase, me.phase_init = DONE, 0, 0
end

-- 0x080DB378: its hits spark, it leaves its panel, and 4 ticks later it is gone.
local function cross_done(me: Object)
    me:hit_spark()
    me.collision:set_region(nil)
    local t = me.timer - 1
    me.timer = int.u16(t)
    if t <= 0 then
        me.visible = false
        me:set_lifecycle("destroy")
    end
end

-- 0x080DB2CE.
local function cross_tick(me: Object)
    local c = me.collision
    c:remove()
    if battle.over() then
        cross_gone(me)
        return
    end
    local a = me.action
    if a == FLY then
        cross_fly(me)
    elseif a == STRIKE then
        cross_strike(me)
    elseif a == DONE then
        cross_done(me)
    else
        error(string.format("CrsShld's cross's action %#x reads past 0x080DB308", a))
    end
    if not field.valid(me.panel_x, me.panel_y) then
        cross_gone(me)
        return
    end
    c:present()
end

-- 0x080DB27C: on its panel, on the ground, its look (sheet 04-05, animation 1, with a shadow; its chip's palette, twice
-- the original's parameter) and its probe (row 0, hitting navis; no damage: the collision's damage word cleared), flying
-- forward.
local function cross_init(me: Object)
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    local sprite = me.sprite
    sprite:load(CROSS_SPRITE)
    sprite.shadow = "ground"
    me.anim, me.anim_loaded = 1, 0xFF
    if not me:create_collision() then
        error("CrsShld's cross has no collision: 0x080DB27C goes on through a null pointer")
    end
    me:setup_collision(collision.nothing, collision.hits_navis, 0)
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me.visible = true
    sprite.palette = (me.state :: CrossState).palette
    me.vel = Vec3.new(me.facing * CROSS_SPEED * 0x10000, me.vel.y, me.vel.z)
    local c = me.collision
    c:set_hit_effect(CROSS_SPARK)
    c.self_damage = 0
    c.bugs = CROSS_BUGS
    c:present()
    me.lifecycle = "update"
    cross_tick(me)
end

local CROSS = new.kind {
    id = "crsshld/cross",
    pool = "attack",
    state = { palette = "u8" },
    -- 0x080DB258.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            cross_init(me)
        elseif l == "update" then
            cross_tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite()
    end,
}

-- 0x080DB398 and `sub_801155A`: a cross of `owner`'s on (x, y), with the element and the damage word, in palette
-- `palette`. (Its X and Y are the spawner's registers until its init places it.)
function cross.spawn(owner: Object, x: number, y: number, element: number, damage: number, palette: number): Object?
    local o = battle.spawn(CROSS, Vec3.new(y, element, 0))
    if o then
        (o.state :: CrossState).palette = palette
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.related1 = owner
    end
    return o
end

return cross
