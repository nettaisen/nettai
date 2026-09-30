--!strict
-- SunMoon's meteor (attack object #0xB5, `sub_80DBEE6`): it falls from
-- its spawner's height, 20 pixels ahead and 20 lower, onto the panel three
-- columns ahead in 16 ticks, its collision on its way; landing it bursts
-- (an explosion) and hits there for a tick, and breaks the panel if it
-- held nothing. Its look and collision are its record's (Param1): SunMoon
-- throws record 0's. It runs while dimmed. See docs/engine/chips.md
-- §3.6.32.

local sun_meteor = {}

-- Its records (`byte_80DBEE0`, six bytes by Param1): the sprite (category
-- 0x0C), the animation, the collision's self and target types, whether
-- the landing shakes the camera (drawn only), and what it does to a
-- panel holding something (0 nothing, 1 crack, else break).
type Record = { sprite: string, anim: number, self_type: number, target: number, shake: boolean, panel: number }
local RECORDS: { [number]: Record } = {
    [0] = { sprite = "0C-31", anim = 0, self_type = 0x0A, target = 5, shake = true, panel = 0 },
}

-- How far ahead and lower it starts (16.16), and how long it falls.
local START = 0x14_0000
local FALL_TICKS = 0x10

-- What a landing panel must have one of to burst (a solid or occupied
-- panel: as one word).
local HOLDS = 0x0F80_0010

local function record(me: Object): Record
    local r = RECORDS[me:param(1)]
    if not r then
        error(string.format("SunMoon's meteor record %d reads past byte_80DBEE0", me:param(1)))
    end
    return r
end

-- `sub_80DC0C0`: a meteor on (x, y) with `element`, the damage word and
-- `params`, on `owner`'s side (its flip left 0). `z` is the spawner's
-- register that becomes its starting height.
function sun_meteor.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("sun-meteor", Vec3.new(y, element, z), params)
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o.damage = damage
        o.stamina = bit32.rshift(damage, 16)
        o.alliance = owner.alliance
        o.run_while_dimmed = true
    end
    return o
end

-- `sub_80DBF04`: its look; ahead of and below where it was spawned,
-- aimed at the panel three columns ahead; its collision on with its
-- region off.
local function init(me: Object)
    me.visible = true
    local r = record(me)
    local sprite = me.sprite
    sprite:load(r.sprite)
    me.anim = r.anim
    me.anim_loaded = r.anim
    sprite:set_animation(r.anim)
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
    me:setup_collision(r.self_type, r.target, 3)
    local c = me.collision
    c.hit_effect = bit32.band(me.element, 0xF)
    c.region = 0
    c:present()
    battle.play_sound(0xC4)
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
        battle.effect(me.pos, 5)
        battle.play_sound(0x70)
        -- (A shake of the camera, if its record says so, is drawn only.)
        local r = record(me)
        if r.panel == 1 then
            field.crack(x, y)
        elseif r.panel ~= 0 then
            field.break_panel(x, y)
        end
    end
    me.collision.region = 1
    me:set_action(8)
    me.visible = false
end

-- `sub_80DC0AC`: the region off after its tick; gone.
local function done(me: Object)
    me.visible = false
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = start, [4] = fall, [8] = done }

-- `sub_80DBFC0`: resolve the hits, act, step the sprite, register again.
local function tick(me: Object)
    local c = me.collision
    c:remove()
    me:hit_spark()
    local action = ACTIONS[me.action]
    if not action then
        error(string.format("SunMoon's meteor action %#x reads past its table (off_80DBFF4)", me.action))
    end
    action(me)
    me:update_sprite()
    c:present()
end

-- `sub_80DBEE6`.
function sun_meteor.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return sun_meteor
