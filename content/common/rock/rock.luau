--!strict
-- Rocks, as BN5 and BN6 share them (BN6's attack object #0x59, `sub_80CF954`; BN5's #0x59, 0x080D0FA0, the same
-- code): field obstacles with HP that block their panel, break into debris when destroyed, and leave after 6000
-- ticks. A rock is spawned with its variant (the original's first parameter, a row of BN6's `byte_80CF934` or BN5's
-- 0x080D0F80: the rock cubes and the ice block), the class of field-object slot it takes (the second: 0, two a side;
-- 1, one) and how it comes onto the field (the third, its entrance). RockCube and IceCube place one, and others use
-- it too: an obstacle encased in ice becomes an ice block (BN6's), and some stages place them when the round starts
-- (an actor list's entry type 8, the kind's `place`, with the variant the stage names). See BN6's
-- docs/engine/field-objects.md.
--
-- Each game makes its own kind (`rock.make`) with its look: the sprite, the debris and effects it leaves, the
-- landing sound, how it looks absorbed (BN6's; BN5 neither absorbs nor blinks obstacles out: a removal leaves a
-- puff, 0x080D1170) and the damage a stage's rocks are thrown for. Its variants are the game's (`rock.variant`).
--
-- An obstacle: its update runs the obstacle framework's steps (the `obstacle` service) around its own actions.

local debris = require("./debris")
local element = require("@common/element")
local regions = require("@common/regions")

local rock = {}

export type State = {
    variant: RockVariant,
    entrance: RockEntrance,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The cubes' sheet (animation 0 a cube rising; the rows' are the cubes standing).
    sprite: SpriteAsset,
    -- The game's debris kind (@common/rock/debris), and the effects it leaves: a puff when removed, dust when
    -- broken.
    debris: Kind,
    effects: { puff: Effect, dust: Effect },
    -- The spark its hits show (the breaking one).
    spark: Spark,
    -- What it is and reaches (a neutral body that everything reaches, while dimmed too).
    is: CollisionType,
    hits: CollisionType,
    land_sound: Sound,
    -- How it looks absorbed (BN6's obstacle kind 4: its own sprite, in the animation it stood in); none (BN5's): a
    -- removal leaves a puff.
    absorbed: AbsorbedLook?,
    -- The damage a stage's rocks do when thrown (BN6's 200; BN5's the same, 0x08006910).
    stage_damage: number,
}

export type Rock = {
    kind: Kind,
    -- `sub_80CFBC4`: a rock for `side` on (x, y): of `spec.variant`, in field-object class `spec.class` of the
    -- side's slots (0 by default; evicting the oldest if they are full), coming as `spec.entrance` ("rise" by
    -- default), with the damage word it hits with when thrown. Placed at the start, it also runs while the battle
    -- is paused. (Until its init places it, its position is the spawner's registers: Y, the side, and what nothing
    -- reads.)
    spawn: (x: number, y: number, side: number, spec: RockSpec, damage: number) -> Object?,
}

-- Rocks live this many ticks.
local LIFETIME = 6000
-- Where it can stand (`object_checkPanelParameters`): a solid panel with nothing on it (bodies, neutral objects,
-- blockers, reservations).
local SOLID, OCCUPIED = 0x10, 0x0F880080
local HIT_MOD = 3
-- Falling from 160 pixels up, 0.25 pixels a tick faster each tick.
local FALL_HEIGHT, GRAVITY = 0xA0_0000, 0x4000
-- The animation it rises in.
local RISING = 0
-- A frame's flag: its animation's last.
local LAST_FRAME = 0x80

-- Its actions (BN6's `off_80CFA48`; BN5's table has no frozen or bubbled rows and stands in its 6, the same
-- actions).
local APPEAR, DESTROYED, STANDING = 0, 2, 8

-- A rock's variant (a row of the game's table).
function rock.variant(v: RockVariant): RockVariant
    return define.record("rock-variant", v)
end

function rock.make(look: Look): Rock
    local absorbed = look.absorbed

    -- Stand on the panel: the standing animation, collision on the panel, and the normal pause behavior.
    local function stand(me: Object)
        me.anim = (me.state :: State).variant.anim
        me.collision:set_region(regions.single)
        me.run_while_paused = false
        me:set_action(STANDING)
    end

    -- `sub_80CFAC0`: fall from 160 pixels up; break if the panel below is gone.
    local function fall(me: Object)
        if me.phase_init == 0 then
            me.anim = (me.state :: State).variant.anim
            me.vel = Vec3.new(me.vel.x, me.vel.y, 0)
            me.pos = Vec3.new(me.pos.x, me.pos.y, FALL_HEIGHT)
            me.phase_init = 4
        end
        local vz = int.i32(me.vel.z - GRAVITY)
        me.vel = Vec3.new(me.vel.x, me.vel.y, vz)
        local z = int.i32(me.pos.z + vz)
        me.pos = Vec3.new(me.pos.x, me.pos.y, z)
        if z <= 0 then
            if not field.solid(me.panel_x, me.panel_y) then
                me:set_action(DESTROYED)
            else
                battle.shake_camera(1, 0x14)
                me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
                me.collision:set_region(regions.single)
                battle.play_sound(look.land_sound)
                me:set_action(STANDING)
            end
        end
        field.highlight(me.panel_x, me.panel_y)
    end

    -- `sub_80CFA6C`: take the panel once the entrance is over; if it isn't free solid ground, break at once.
    local function appear(me: Object)
        local entrance = (me.state :: State).entrance
        if entrance == "fall" then
            fall(me)
            return
        end
        if entrance == "rise" and not bit32.btest(me.sprite.frame_flags, LAST_FRAME) then
            return
        end
        if not field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
            me:set_action(DESTROYED)
            return
        end
        stand(me)
    end

    -- `sub_80CFB2C`: leave the field, breaking into debris unless a chip removed it.
    local function destroyed(me: Object)
        me.collision:set_region(nil)
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        obstacle.release_tracking(me)
        obstacle.unregister(me)
        local how = obstacle.removal(me)
        if absorbed and how == "absorbed" then
            obstacle.fly_to_absorber(me, absorbed)
        elseif how ~= "broken" then
            local blink = if absorbed then obstacle.blink_out(me) else "no"
            if blink == "blinking" then
                return
            elseif blink == "no" then
                local p = me.pos
                battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), look.effects.puff)
            end
        else
            local v = (me.state :: State).variant
            local p = me.pos
            for _ = 1, 2 do
                debris.spawn(look.debris, battle.jitter(0xF, p), v.debris_palette)
            end
            battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), look.effects.dust)
            battle.play_sound(v.break_sound)
        end
        me.visible = false
        me:set_lifecycle("destroy")
    end

    -- `sub_80CFB28`.
    local function standing(_me: Object) end

    -- The rock's action table (`off_80CFA48`): its own actions and the framework's shared ones.
    local ACTIONS: { [number]: (Object) -> () } = {
        [APPEAR] = appear,
        [1] = obstacle.return_to_idle,
        [DESTROYED] = destroyed,
        [3] = obstacle.flinch,
        [4] = obstacle.paralyzed,
        [5] = obstacle.slide,
        [6] = obstacle.frozen,
        [7] = obstacle.bubbled,
        [STANDING] = standing,
    }

    -- `sub_80CFA18`.
    local function tick(me: Object)
        me.visible = true
        obstacle.take_hits(me)
        me:hit_spark()
        obstacle.tick_lifetime(me)
        local a = obstacle.react(me)
        if a then
            local action = ACTIONS[a]
            if not action then
                error(string.format("rock action %d reads past its table", a))
            end
            action(me)
        end
        me:update_sprite()
        me.collision:present()
    end

    -- `sub_80CF974`: on its panel with the variant's HP, element and name, rising (or standing at once when placed
    -- at the start).
    local function init(me: Object)
        local s = me.state :: State
        local v = s.variant
        me.visible = true
        local sprite = me.sprite
        sprite:load(look.sprite)
        me.anim_loaded = 0xFF
        local anim = if s.entrance == "placed" then v.anim else RISING
        me.anim = anim
        sprite:set_animation(anim)
        sprite:step()
        sprite.shadow = "ground"
        me:set_coordinates_from_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
        me.element = element.byte(v.element)
        me.hp, me.max_hp = v.hp, v.hp
        me.identity = v.identity
        me.timer = LIFETIME
        if not me:create_collision() then
            me:free()
            return
        end
        me:setup_collision(look.is, look.hits, HIT_MOD)
        me.collision:set_hit_effect(look.spark)
        me.collision:set_region(nil)
        me.collision:present()
        me.lifecycle = "update"
        tick(me)
    end

    -- The kind (below; its `place` spawns one).
    local kind: Kind

    local function spawn(x: number, y: number, side: number, spec: RockSpec, damage: number): Object?
        local o = battle.spawn(kind, Vec3.new(y, side, 0))
        if not o then
            return nil
        end
        local entrance: RockEntrance = spec.entrance or "rise"
        local s = o.state :: State
        s.variant, s.entrance = spec.variant, entrance
        o.panel_x, o.panel_y = x, y
        o.alliance = side
        o:set_damage_word(damage)
        o.run_while_dimmed = true
        if entrance == "placed" then
            o.run_while_paused = true
        end
        obstacle.register(o, side, spec.class or 0)
        return o
    end

    kind = define.kind {
        id = look.id,
        pool = "attack",
        state = { variant = "record:rock-variant", entrance = { "rise", "instant", "fall", "placed" } },
        -- `sub_80CF954`.
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
        -- `sub_80074FA`: a stage's rock (an actor list's entry type 8), of the variant the stage names, on its
        -- panel's side, standing from the start.
        place = function(spec: PlaceSpec): Object?
            local x, y = spec.panel_x, spec.panel_y
            local panel = field.panel(x, y)
            if not panel then
                error(string.format("a stage places a rock off the field at (%d, %d)", x, y))
            end
            local v = spec.variant :: RockVariant?
            if not v then
                error("a stage's rock needs its `variant` (one of its game's rock variants)")
            end
            local placed: RockSpec = { variant = v, entrance = "placed" }
            return spawn(x, y, panel.alliance, placed, look.stage_damage)
        end,
    }

    return { kind = kind, spawn = spawn }
end

return rock
