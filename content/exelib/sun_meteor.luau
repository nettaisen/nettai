--!strict
-- A sun's meteor, as EXE6 and EXE4 share it (EXE6's attack object #0xB5, `sub_80DBEE6`, SunMoon's; EXE4's #0x85,
-- 0x080DDDEE, RedSun's, the same code): it falls from its spawner's height, 20 pixels ahead and 20 lower, onto the
-- panel three columns ahead in 16 ticks, its collision on its way; landing it bursts (an explosion) and hits there for
-- a tick, and breaks the panel if it held nothing. Its look and collision are its variant's (`sun_meteor.variant`: the
-- original's six-byte records by Param1, EXE6's `byte_80DBEE0`, EXE4's 0x080DDDE8): each sun throws its own. It runs
-- while dimmed. See EXE6's docs/engine/chips.md §3.6.32.
--
-- Each game makes its own kind (`sun_meteor.make`) with its sounds, its burst and its sparks by element. Its spawn
-- takes its owner's side alone (its flip left 0: it falls toward the side's enemy, whichever way it faces: EXE6's
-- object_getFrontDirection of the unflipped meteor, EXE4's object_getEnemyDirection).

local regions = require("@exelib/regions")

local sun_meteor = {}

export type State = { variant: SunMeteor }

export type Look = {
    -- The kind's id.
    id: string,
    -- As it starts falling (EXE6's 0xC4, EXE4's 0x146), and as it bursts (0x6F, 0x70).
    fall_sound: Sound,
    burst_sound: Sound,
    -- Its burst where it lands on something (EXE6's effect 5, EXE4's row 6).
    burst: Effect,
    -- Its hits' spark by its element (`sub_801A146`, EXE4's 0x08012F7A: the element's number as the spark's).
    spark_of: (element: number) -> Spark,
}

export type SunMeteors = {
    kind: Kind,
    -- `sub_80DC0C0`, EXE4's 0x080DDFCC: a meteor on (x, y) with `element`, the damage word and its variant, on
    -- `owner`'s side (its flip left 0). `z` is the spawner's register that becomes its starting height.
    spawn: (
        owner: Object,
        x: number,
        y: number,
        element: number,
        variant: SunMeteor,
        damage: number,
        z: number
    ) -> Object?,
}

-- A meteor's variant (a record of the original's table).
function sun_meteor.variant(v: SunMeteor): SunMeteor
    return new.record("sun-meteor", v)
end

local HIT_MOD = 3

-- How far ahead and lower it starts (16.16), and how long it falls.
local START = 0x14_0000
local FALL_TICKS = 0x10

-- What a landing panel must have one of to burst (a solid or occupied panel: as one word).
local HOLDS = 0x0F80_0010

local function record(me: Object): SunMeteor
    return (me.state :: State).variant
end

function sun_meteor.make(look: Look): SunMeteors
    -- `sub_80DBF04`, EXE4's 0x080DDE0C: its look; ahead of and below where it was spawned, aimed at the panel three
    -- columns ahead; its collision on with its region off.
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
        me:setup_collision(r.collision.is, r.collision.hits, HIT_MOD)
        local c = me.collision
        c:set_hit_effect(look.spark_of(me.element))
        c:set_region(nil)
        c:present()
        battle.play_sound(look.fall_sound)
        me.lifecycle = "update"
    end

    -- `sub_80DC000`, EXE4's 0x080DDF0C: a tick, then the fall.
    local function start(me: Object)
        local left = me.timer - 1
        me.timer = left
        if left ~= 0 then
            return
        end
        me.timer = FALL_TICKS
        me:set_action(4)
    end

    -- `sub_80DC018`, EXE4's 0x080DDF24: fall; landing, burst and hit where the panel holds something (and crack or
    -- break it, by its record), else break it.
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
            battle.effect(me.pos, look.burst)
            battle.play_sound(look.burst_sound)
            local r = record(me)
            if r.shake then
                battle.shake_camera(1, 0xF)
            end
            if r.panel == "crack" then
                field.crack(x, y)
            elseif r.panel == "break" then
                field.break_panel(x, y)
            end
        end
        me.collision:set_region(regions.single)
        me:set_action(8)
        me.visible = false
    end

    -- `sub_80DC0AC`, EXE4's 0x080DDFB8: the region off after its tick; gone.
    local function done(me: Object)
        me.visible = false
        me.collision:set_region(nil)
        me:set_lifecycle("destroy")
    end

    -- `sub_80DBFC0`, EXE4's 0x080DDECC: resolve the hits, act, step the sprite, register again.
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
            error(string.format("a sun's meteor: action %#x reads past its table (EXE6's off_80DBFF4)", a))
        end
        me:update_sprite()
        c:present()
    end

    local kind = new.kind {
        id = look.id,
        pool = "attack",
        state = { variant = "record:sun-meteor" },
        -- `sub_80DBEE6`, EXE4's 0x080DDDEE.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                init(me)
            elseif l == "update" then
                tick(me)
            else
                me:destroy()
            end
        end,
    }

    local function spawn(
        owner: Object,
        x: number,
        y: number,
        element: number,
        variant: SunMeteor,
        damage: number,
        z: number
    ): Object?
        local o = battle.spawn(kind, Vec3.new(y, element, z))
        if o then
            local s = o.state :: State
            s.variant = variant
            o.panel_x, o.panel_y = x, y
            o.element = element
            o:set_damage_word(damage)
            o.alliance = owner.alliance
            o.run_while_dimmed = true
        end
        return o
    end

    return { kind = kind, spawn = spawn }
end

return sun_meteor
