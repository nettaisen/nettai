--!strict
-- SunMoon's meteor (the original's attack object #0xB5, `sub_80DBEE6`): it
-- falls from its spawner's height, 20 pixels ahead and 20 lower, onto the
-- panel three columns ahead in 16 ticks, its collision on its way; landing
-- it bursts (an explosion) and hits there for a tick, and breaks the panel
-- if it held nothing. It runs while dimmed. See docs/engine/chips.md
-- §3.6.32.

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local meteor = {}

-- Its record (`byte_80DBEE0`, the table's one row): its look, what it is
-- and hits (a thrown attack that reaches navis), whether its landing
-- shakes the camera, and what it does to a panel holding something
-- (nothing, crack it, or break it).
local SPRITE, ANIM = asset.sprite("meteor"), 0
local SHAKES = true
local PANEL: "none" | "crack" | "break" = "none"

local FALL_SOUND = asset.sound("justice-one")
local LAND_SOUND = asset.sound("hit-bomb-1")

-- How far ahead and lower it starts (16.16), and how long it falls.
local START = 0x14_0000
local FALL_TICKS = 0x10

-- What a landing panel must have one of to burst (a solid or occupied
-- panel: as one word).
local HOLDS = 0x0F80_0010

-- `sub_80DBF04`: its look; ahead of and below where it was spawned,
-- aimed at the panel three columns ahead; its collision on with its
-- region off.
local function init(me: Object)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim = ANIM
    me.anim_loaded = ANIM
    sprite:set_animation(ANIM)
    sprite:step()
    sprite.shadow = "with_sprite"
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_coordinates_from_panel()
    local front = me.facing
    local p = me.pos
    local x, z = p.x + front * START, p.z - START
    me.pos = Vec3.new(x, p.y, z)
    local target = field.center(me.panel_x + front * 3, me.panel_y)
    me.vel = Vec3.new(int.tdiv(int.i32(target - x), FALL_TICKS), 0, int.tdiv(int.i32(-z), FALL_TICKS))
    me.timer = 1
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(collision.thrown, collision.hits_navis, 3)
    local c = me.collision
    c:set_hit_effect(sparks.of_element(me.element))
    c:set_region(nil)
    c:present()
    battle.play_sound(FALL_SOUND)
    me.lifecycle = "update"
end

-- `sub_80DC000`: a tick, then the fall.
local function start(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        return
    end
    me.timer = FALL_TICKS
    me:set_action(4)
end

-- `sub_80DC018`: fall; landing, burst and hit where the panel holds
-- something (and crack or break it, by its record), else break it.
local function fall(me: Object)
    local left = me.timer - 1
    me.timer = left
    if left ~= 0 then
        local p, v = me.pos, me.vel
        me.pos = Vec3.new(p.x + v.x, p.y, p.z + v.z)
        me:set_panel_from_coordinates()
        me:update_collision_panels()
        return
    end
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    local x, y = me.panel_x, me.panel_y
    if bit32.band(field.flags(x, y), HOLDS) == 0 then
        field.break_panel(x, y)
    else
        battle.effect(me.pos, effects.burst)
        battle.play_sound(LAND_SOUND)
        if SHAKES then
            battle.shake_camera(1, 0xF)
        end
        if PANEL == "crack" then
            field.crack(x, y)
        elseif PANEL == "break" then
            field.break_panel(x, y)
        end
    end
    me.collision:set_region(regions.single)
    me:set_action(8)
    me.visible = false
end

-- `sub_80DC0AC`: the region off after its tick; gone.
local function done(me: Object)
    me.visible = false
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80DBFC0`: resolve the hits, act, step the sprite, register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local a = me.action
    if a == 0 then
        start(me)
    elseif a == 4 then
        fall(me)
    elseif a == 8 then
        done(me)
    else
        error(string.format("SunMoon's meteor action %#x reads past its table (off_80DBFF4)", a))
    end
    me:update_sprite()
    c:present()
end

-- `sub_80DBEE6`.
local function update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

meteor.kind = define.kind {
    id = "sunmoon/meteor",
    pool = "attack",
    update = update,
}

-- `sub_80DC0C0`: a meteor on (x, y) with `element` and the damage word,
-- on `owner`'s side (its flip left 0). `z` is the spawner's register that
-- becomes its starting height.
function meteor.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number): Object?
    local o = battle.spawn(meteor.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

return meteor
