--!strict
-- A rock falling on a panel (the original's attack object #0x80,
-- `sub_80D535C`): GroundMan's rockfall, and his link navi's RC Brakr
-- (navis/groundman). Its panel blinks for a while, then the rock drops
-- from 160 pixels in 25 ticks and hits the panel it lands on; on a solid
-- panel it breaks into two chips (objects/falling-rock/chip), cracking the
-- panel if it cracks. A hit on the way throws chips off too, but it falls
-- on. See docs/engine/chips.md §3.6.25.

local collision = require("../../rules/collision")
local regions = require("../../lib/regions")
local rock_chip = require("../../objects/falling-rock/chip")
local sparks = require("../../lib/sparks")

local rock = {}

export type State = {
    -- Ticks its panel blinks before it drops (Param1).
    warning: number,
    -- It acts, and animates, while the battle is dimmed (Param2 1).
    while_dimmed: boolean,
    -- It cracks the panel it lands on (Param3), silently (Param4).
    cracks: boolean,
    silent: boolean,
}

export type Drop = { warning: number, while_dimmed: boolean, cracks: boolean, silent: boolean }

local SPRITE = asset.sprite("falling-rock")
local SOUND_BREAK = asset.sound("falling-rock")

-- Its height as it starts to fall, how fast it falls (`byte_80D5574`),
-- and for how long.
local HEIGHT, FALL_SPEED, FALL_TICKS = 0xA0_0000, 0x6_5000, 0x19

-- The chips' velocities (x, z): a rock throws two consecutive pairs, the
-- first picked at random (`byte_80D5550`).
local CHIPS = { [0] = { 0x8000, 0x2_8000 }, [1] = { -0x8000, 0x3_0000 }, [2] = { 0x1_0000, 0x2_8000 }, [3] = { -0x8000, 0x1_8000 } }

-- Its phases (actions).
local WARN, FALL, LAND = 0, 4, 8

-- `sub_80D5516`: two chips from where it is, drawn in front (priority
-- 2); one draw picks the first pair. (Their Param1, their number, nothing
-- reads.)
local function break_up(me: Object)
    local k = bit32.band(battle.rng_positive(), 3)
    for n = 0, 1 do
        local v = CHIPS[bit32.band(k + n, 3)]
        local p = me.pos
        rock_chip.spawn(Vec3.new(p.x, p.y, 0), v[1], v[2], 2)
    end
end

-- `sub_80D5448`: its panel blinks (4 ticks out of 8) for its warning.
local function warn(me: Object, s: State)
    if me.phase == 0 then
        me.phase = 4
        me.timer = s.warning
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
        me:set_action(FALL)
    end
end

-- `sub_80D5482`: fall with a ground shadow; landed, its region opens.
local function fall(me: Object)
    if me.phase == 0 then
        me.phase = 4
        me.sprite.shadow = "ground"
        me.vel = Vec3.new(me.vel.x, me.vel.y, FALL_SPEED)
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
    me.collision:set_region(regions.single)
    me:set_action(LAND)
end

-- `sub_80D54C0`: on a solid panel it breaks up (and cracks the panel);
-- the sound unless silent; gone.
local function land(me: Object, s: State)
    if field.solid(me.panel_x, me.panel_y) then
        break_up(me)
        if s.cracks then
            field.crack(me.panel_x, me.panel_y)
        end
    end
    if not s.silent then
        battle.play_sound(SOUND_BREAK)
    end
    me.collision:set_region(nil)
    me:set_lifecycle("destroy")
end

-- `sub_80D53F4`: resolve its hits (a hit throws chips off); act unless
-- dimmed (or while dimmed too); register again.
local function tick(me: Object)
    local s = me.state :: State
    local c = me.collision
    c:remove()
    me:hit_spark()
    if battle.over() then
        c:set_region(nil)
        me:set_lifecycle("destroy")
        return
    end
    if c.hit_flags ~= 0 then
        break_up(me)
    end
    if s.while_dimmed or not battle.dimmed() then
        local a = me.action
        if a == WARN then
            warn(me, s)
        elseif a == FALL then
            fall(me)
        elseif a == LAND then
            land(me, s)
        else
            error(string.format("falling rock action %#x reads past its table (off_80D543C)", a))
        end
    end
    c:present()
end

-- `sub_80D538C`: over its panel, up high; collision on, its region
-- closed. Without a collision slot it is freed.
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
    me:setup_collision(collision.thrown_break, collision.hits_navis, 3)
    me.collision:set_hit_effect(sparks.breaking)
    me.collision:set_region(nil)
    me.collision:present()
    me:set_lifecycle("update")
end

rock.kind = define.kind {
    id = "grndman/rock",
    pool = "attack",
    state = { warning = "u8", while_dimmed = "bool", cracks = "bool", silent = "bool" },
    -- `sub_80D535C`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
        end
        if (me.state :: State).while_dimmed then
            me:update_sprite_while_dimmed()
        else
            me:update_sprite_while_paused()
        end
    end,
}

-- `sub_80D54F0`: a rock of `owner`'s on (x, y) with `element` and the
-- damage word, running while dimmed (flag 0x10; the rock's own gate
-- still holds it unless it acts while dimmed). `z` is the spawner's
-- register its position takes until the init places it.
function rock.drop(owner: Object, x: number, y: number, element: number, damage: number, how: Drop, z: number): Object?
    local o = battle.spawn(rock.kind, Vec3.new(y, element, z))
    if o then
        o.panel_x, o.panel_y = x, y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
        local s = o.state :: State
        s.warning, s.while_dimmed, s.cracks, s.silent = how.warning, how.while_dimmed, how.cracks, how.silent
    end
    return o
end

return rock
