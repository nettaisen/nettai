--!strict
-- Rocks (the original's attack object #0x59, `sub_80CF954`): field
-- obstacles with HP that block their panel, break into debris when
-- destroyed, and leave after 6000 ticks. A rock is spawned with its variant
-- (the original's first parameter, a row of `byte_80CF934`: the rock cubes
-- and the ice block), the class of field-object slot it takes (the second:
-- 0, two a side; 1, one) and how it comes onto the field (the third, its
-- entrance). RockCube and IceCube place one (./cube), and others use it
-- too: an obstacle encased in ice becomes an ice block (the encasing's
-- replacement, objects/encased-bubble), and some stages place them when
-- the round starts (stages/netbattle: the actor lists' entry type 8, the
-- kind's `place`, with the variant the stage names). See
-- docs/engine/field-objects.md.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle/absorbed_obstacle")
local collision = require("../../rules/collision")
local debris = require("./debris")
local effects = require("../../lib/effects")
local element = require("../../lib/element")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

local rock = {}

export type State = {
    variant: RockVariant,
    entrance: RockEntrance,
}

-- The cubes' sheet (animation 0 a cube rising; the rows' are the cubes
-- standing).
local SPRITE = asset.sprite("immobilized")
-- How a rock looks absorbed (the original's obstacle kind 4: its own
-- sprite, in the animation it stood in).
local ABSORBED = absorbed_obstacle.look { sprite = SPRITE }

-- Rocks live this many ticks.
local LIFETIME = 6000
-- The damage a stage's rocks do when thrown.
local STAGE_ROCK_DAMAGE = 200
-- Where it can stand (`object_checkPanelParameters`): a solid panel with
-- nothing on it (bodies, neutral objects, blockers, reservations).
local SOLID, OCCUPIED = 0x10, 0x0F880080
-- What it is and reaches: a neutral body that everything reaches, while
-- dimmed too (rows 0x0E and 0x0F); hit modifier 3; the breaking spark.
local IS, HITS, HIT_MOD = collision.neutral_body, collision.everything, 3
-- Falling from 160 pixels up, 0.25 pixels a tick faster each tick.
local FALL_HEIGHT, GRAVITY = 0xA0_0000, 0x4000
local LAND_SOUND = asset.sound("land")
-- The animation it rises in.
local RISING = 0
-- A frame's flag: its animation's last.
local LAST_FRAME = 0x80

-- Its actions (`off_80CFA48`).
local APPEAR, DESTROYED, STANDING = 0, 2, 8

-- A rock's variant.
local function variant(v: RockVariant): RockVariant
    return define.record("rock-variant", v)
end

-- What a rock is taken for: a rock cube, an ice block (field objects, with
-- the looks something that takes their place shows).
local ROCK = define.identity {
    id = "rock",
    class = "field_object",
    version = 3,
    ai_index = 1,
    look = { sprite = asset.sprite("immobilized"), anim = 1, palette = 0, shadow = true },
}
local ICE = define.identity {
    id = "ice-block",
    class = "field_object",
    version = 4,
    ai_index = 1,
    look = { sprite = asset.sprite("immobilized"), anim = 2, palette = 0, shadow = true },
}

-- The rows of `byte_80CF934` (animation, HP in twos, debris palette, break
-- sound, identity; a row past the second is aqua, and a row without HP has
-- 1).
local ROWS: { [number]: RockVariant } = {
    -- A rock cube anything breaks (no chip or stage places it).
    [0] = variant {
        id = "rockcube/brittle",
        anim = 1,
        hp = 1,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- RockCube's, and the stages'.
    [1] = variant {
        id = "rockcube/cube",
        anim = 1,
        hp = 200,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- A hard one (no chip or stage places it).
    [2] = variant {
        id = "rockcube/hard",
        anim = 1,
        hp = 500,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- The ice block: IceCube's, the stages', and what an obstacle encased
    -- in ice becomes.
    [3] = variant {
        id = "rockcube/ice",
        anim = 2,
        hp = 200,
        element = "aqua",
        debris_palette = 1,
        break_sound = asset.sound("falling-rock"),
        identity = ICE,
    },
}
rock.variants = { brittle = ROWS[0], cube = ROWS[1], hard = ROWS[2], ice = ROWS[3] }

-- Stand on the panel: the standing animation, collision on the panel, and
-- the normal pause behavior.
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
            battle.play_sound(LAND_SOUND)
            me:set_action(STANDING)
        end
    end
    field.highlight(me.panel_x, me.panel_y)
end

-- `sub_80CFA6C`: take the panel once the entrance is over; if it isn't
-- free solid ground, break at once.
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

-- `sub_80CFB2C`: leave the field, breaking into debris unless a chip
-- removed it.
local function destroyed(me: Object)
    me.collision:set_region(nil)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED)
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), effects.puff)
        end
    else
        local v = (me.state :: State).variant
        local p = me.pos
        for _ = 1, 2 do
            debris.spawn(battle.jitter(0xF, p), v.debris_palette)
        end
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), effects.dust)
        battle.play_sound(v.break_sound)
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CFB28`.
local function standing(_me: Object) end

-- The rock's action table (`off_80CFA48`): its own actions and the
-- framework's shared ones.
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

-- `sub_80CF974`: on its panel with the variant's HP, element and name,
-- rising (or standing at once when placed at the start).
local function init(me: Object)
    local s = me.state :: State
    local v = s.variant
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
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
    me:setup_collision(IS, HITS, HIT_MOD)
    me.collision:set_hit_effect(sparks.breaking)
    me.collision:set_region(nil)
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- The kind (below; its `place` spawns one).
local KIND: Kind

-- `sub_80CFBC4`: a rock for `side` on (x, y): of `spec.variant`, in
-- field-object class `spec.class` of the side's slots (0 by default;
-- evicting the oldest if they are full), coming as `spec.entrance` ("rise"
-- by default), with the damage word it hits with when thrown. Placed at the
-- start, it also runs while the battle is paused. (Until its init places
-- it, its position is the spawner's registers: Y, the side, and what
-- nothing reads.)
function rock.spawn(x: number, y: number, side: number, spec: RockSpec, damage: number): Object?
    local o = battle.spawn(KIND, Vec3.new(y, side, 0))
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

KIND = define.kind {
    id = "rockcube/rock",
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
    -- `sub_80074FA`: a stage's rock (an actor list's entry type 8), of the
    -- variant the stage names, on its panel's side, standing from the start.
    place = function(spec: PlaceSpec): Object?
        local x, y = spec.panel_x, spec.panel_y
        local panel = field.panel(x, y)
        if not panel then
            error(string.format("a stage places a rock off the field at (%d, %d)", x, y))
        end
        local v = spec.variant :: RockVariant?
        if not v then
            error("a stage's rock needs its `variant` (one of rock.variants)")
        end
        local placed: RockSpec = { variant = v, entrance = "placed" }
        return rock.spawn(x, y, panel.alliance, placed, STAGE_ROCK_DAMAGE)
    end,
}
rock.kind = KIND

return rock
