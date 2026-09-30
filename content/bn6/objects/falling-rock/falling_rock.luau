--!strict
-- A falling rock (attack object #0x80, `sub_80D535C`): its panel blinks
-- for Param1 ticks, then a rock drops from 160 pixels up in 25 ticks and
-- hits the panel for a tick (collision self 0x15, target 5, modifier 3,
-- hit spark 0xA): where it hits something, and where it lands on a solid
-- panel, it breaks into rubble (objects/rubble), cracking the panel with
-- Param3 set; sound 0xD9 unless Param4. Param2 1: it acts while dimmed.
-- GroundMan drops them (objects/ground-man); other callers of
-- `sub_80D54F0` too. See docs/engine/chips.md §3.6.7.

local rubble = require("../rubble/rubble")

local falling_rock = {}

local SPRITE = "10-05"

-- Its height when it starts falling, its speed down (16.16) and ticks to
-- fall (`byte_80D5574`).
local HEIGHT, SPEED, FALL_TICKS = 0xA0_0000, 0x6_5000, 0x19

-- The rubble's velocities (x, z), four pairs a rock picks two consecutive
-- ones of (`byte_80D5550`).
local RUBBLE = { [0] = { 0x8000, 0x2_8000 }, { -0x8000, 0x3_0000 }, { 0x1_0000, 0x2_8000 }, { -0x8000, 0x1_8000 } }

-- `sub_80D54F0`: a rock for `owner` on (x, y) with `element`, `params`
-- {warning ticks, while dimmed, cracks, silent} and the damage word. `z`
-- is the spawner's register that becomes its Z (its init sets Z).
function falling_rock.spawn(
    owner: Object,
    x: number,
    y: number,
    element: number,
    params: { number },
    damage: number,
    z: number
): Object?
    local o = battle.spawn_kind("falling-rock", Vec3.new(y, element, z), params)
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

-- `sub_80D538C`: over its panel, up high; collision on, its region off.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, HEIGHT)
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
    me:setup_collision(0x15, 5, 3)
    local c = me.collision
    c.hit_effect = 0xA
    c.region = 0
    c:present()
    me:set_lifecycle("update")
end

-- `sub_80D5516`: two pieces of rubble from where it is, one RNG draw
-- picking the first of two consecutive velocities.
local function break_up(me: Object)
    local k = bit32.band(battle.rng_positive(), 3)
    for n = 0, 1 do
        local v = RUBBLE[bit32.band(k + n, 3)]
        local p = me.pos
        rubble.spawn(Vec3.new(p.x, p.y, 0), v[1], v[2], { n }, 2)
    end
end

-- `sub_80D5448`: the panel blinks (highlighted 4 ticks of 8) for Param1
-- ticks.
local function warn(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.timer = me:param(1)
        me.timer2 = 0
        return
    end
    local t = me.timer2 + 1
    me.timer2 = t
    if not bit32.btest(t, 4) then
        field.highlight(me.panel_x, me.panel_y)
    end
    local left = me.timer - 1
    me.timer = left
    if left <= 0 then
        me:set_action(4)
    end
end

-- `sub_80D5482`: fall; on landing, the region on.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.sprite.shadow = "ground"
        me.vel = Vec3.new(me.vel.x, me.vel.y, SPEED)
        me.timer = FALL_TICKS
        return
    end
    local p = me.pos
    me.pos = Vec3.new(p.x, p.y, p.z - me.vel.z)
    local left = me.timer - 1
    me.timer = left
    if left > 0 then
        return
    end
    p = me.pos
    me.pos = Vec3.new(p.x, p.y, 0)
    me.collision.region = 1
    me:set_action(8)
end

-- `sub_80D54C0`: on a solid panel it breaks up (cracking the panel with
-- Param3); the sound unless Param4; gone.
local function land(me: Object)
    if field.solid(me.panel_x, me.panel_y) then
        break_up(me)
        if me:param(3) ~= 0 then
            field.crack(me.panel_x, me.panel_y)
        end
    end
    if me:param(4) == 0 then
        battle.play_sound(0xD9)
    end
    me.collision.region = 0
    me:set_lifecycle("destroy")
end

local ACTIONS = { [0] = warn, [4] = fall, [8] = land }

-- `sub_80D53F4`: resolve the hits (a hit breaks it up); act unless dimmed
-- (Param2 1 acts anyway); register again.
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
        break_up(me)
    end
    if me:param(2) == 1 or not battle.dimmed() then
        local f = ACTIONS[me.action]
        if not f then
            error(string.format("falling rock action %#x reads past its table (off_80D543C)", me.action))
        end
        f(me)
    end
    c:present()
end

-- `sub_80D535C`.
function falling_rock.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
    if me:param(2) == 0 then
        me:update_sprite_while_paused()
    else
        me:update_sprite_while_dimmed()
    end
end

return falling_rock
