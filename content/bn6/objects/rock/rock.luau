--!strict
-- Rocks (attack object #0x59, `sub_80CF954`): field obstacles with HP that
-- block their panel, break into debris when destroyed, and leave after
-- 6000 ticks. Param1 is the variant (`data.objects.rocks`: 0 to 2 rocks, 3
-- the aqua ice block an obstacle encased in ice becomes), Param2 the class
-- of field-object slot it takes (0: two a side, 1: one), Param3 how it
-- comes onto the field (`ENTRANCES`). RockCube and IceCube place one
-- (objects/rock-cube); some stages place them when the round starts (the
-- actor lists' entry type 8). See docs/engine/field-objects.md.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local rock_debris = require("../rock-debris/rock_debris")

local rock = {}

local SPRITE = "10-00"

-- How it comes onto the field (Param3).
local RISE, INSTANT, FALL, PLACED = 0, 1, 2, 3

-- Rocks live this many ticks.
local LIFETIME = 6000
-- The damage a stage's rocks do when thrown.
local STAGE_ROCK_DAMAGE = 200
-- Its kind among absorbed obstacles (`data.objects.absorbed_sprites`).
local ABSORBED_KIND = 4
-- Where it can stand (`object_checkPanelParameters`): a solid panel with
-- nothing on it (bodies, neutral objects, blockers, reservations).
local SOLID, OCCUPIED = 0x10, 0x0F880080
-- Its collision: an obstacle (types 0xE and 0xF, hit modifier 3), hit
-- spark 0xA.
local SELF_TYPE, TARGET_TYPE, HIT_MOD, HIT_SPARK = 0xE, 0xF, 3, 0xA
-- Falling from 160 pixels up, 0.25 pixels a tick faster each tick.
local FALL_HEIGHT, GRAVITY = 0xA0_0000, 0x4000
local LAND_SOUND = 0xC0
-- What's left when a chip removes it without blinking (effect #0 look
-- 0x14, 12 pixels up), and the dust when it breaks (look 2, 16 up).
local REMOVED_EFFECT, BREAK_EFFECT = 0x14, 2

local ELEMENTS: { [string]: number } = { null = 0, fire = 1, aqua = 2, elec = 3, wood = 4 }

local function kind(me: Object): RockKind
    local k = data.objects.rocks[me:param(1)]
    if not k then
        error(string.format("rock variant %d is not in the pack", me:param(1)))
    end
    return k
end

-- `sub_80CFBC4`: a rock for `side` on (x, y) with the spawn parameters
-- `params` (variant, class, entrance) and the damage word it hits with
-- when thrown, registered in the side's field-object slots (evicting the
-- oldest if they are full). Placed at the start, it also runs while the
-- battle is paused. (Until its init places it, its position is the
-- spawner's registers: Y, the side, and what nothing reads.)
function rock.spawn(x: number, y: number, side: number, params: { number }, damage: number): Object?
    local o = battle.spawn_kind("rock", Vec3.new(y, side, 0), params)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = x, y
    o.alliance = side
    o.damage = damage
    o.stamina = bit32.rshift(damage, 16)
    o.run_while_dimmed = true
    if o:param(3) == PLACED then
        o.run_while_paused = true
    end
    obstacle.register(o, side, o:param(2))
    return o
end

-- `sub_80074FA`: an actor list's rock, on its panel's side.
function rock.actor_list_entry(spec: ActorListEntrySpec): Object?
    local x, y = spec.panel_x, spec.panel_y
    local panel = field.panel(x, y)
    if not panel then
        error(string.format("an actor list places a rock off the field at (%d, %d)", x, y))
    end
    return rock.spawn(x, y, panel.alliance, { spec.variant, 0, PLACED, 0 }, STAGE_ROCK_DAMAGE)
end

-- Stand on the panel: the standing animation, collision on the panel, and
-- the normal pause behavior.
local function stand(me: Object)
    me.anim = kind(me).anim
    me.collision.region = 1
    me.run_while_paused = false
    me:set_action(8)
end

-- `sub_80CFAC0`: fall from 160 pixels up; break if the panel below is gone.
local function fall(me: Object)
    if me.phase_init == 0 then
        me.anim = kind(me).anim
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
            me:set_action(2)
        else
            -- (The landing shakes the camera, which draws the local RNG.)
            me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
            me.collision.region = 1
            battle.play_sound(LAND_SOUND)
            me:set_action(8)
        end
    end
    field.highlight(me.panel_x, me.panel_y)
end

-- `sub_80CFA6C`: take the panel once the entrance is over; if it isn't
-- free solid ground, break at once.
local function appear(me: Object)
    local entrance = me:param(3)
    if entrance == FALL then
        fall(me)
        return
    end
    if entrance == RISE and not bit32.btest(me.sprite.frame_flags, 0x80) then
        return
    end
    if not field.check(me.panel_x, me.panel_y, SOLID, OCCUPIED) then
        me:set_action(2)
        return
    end
    stand(me)
end

-- `sub_80CFB2C`: leave the field, breaking into debris unless a chip
-- removed it.
local function destroyed(me: Object)
    me.collision.region = 0
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    obstacle.release_tracking(me)
    obstacle.unregister(me)
    local how = obstacle.removal(me)
    if how == "absorbed" then
        obstacle.fly_to_absorber(me, ABSORBED_KIND)
    elseif how ~= "broken" then
        local blink = obstacle.blink_out(me)
        if blink == "blinking" then
            return
        elseif blink == "no" then
            local p = me.pos
            battle.effect(Vec3.new(p.x, p.y, p.z + 0xC_0000), REMOVED_EFFECT)
        end
    else
        local k = kind(me)
        local p = me.pos
        for _ = 1, 2 do
            rock_debris.spawn(battle.jitter(0xF, p), k.debris_palette)
        end
        battle.effect(Vec3.new(p.x, p.y, p.z + 0x10_0000), BREAK_EFFECT)
        battle.play_sound(k.break_sound)
    end
    me.visible = false
    me:set_lifecycle("destroy")
end

-- `sub_80CFB28`.
local function idle(_me: Object) end

-- The rock's action table (`off_80CFA48`): its own actions and the
-- framework's shared ones.
local ACTIONS: { [number]: (Object) -> () } = {
    [0] = appear,
    [1] = obstacle.return_to_idle,
    [2] = destroyed,
    [3] = obstacle.flinch,
    [4] = obstacle.paralyzed,
    [5] = obstacle.slide,
    [6] = obstacle.frozen,
    [7] = obstacle.bubbled,
    [8] = idle,
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
    local k = kind(me)
    me.visible = true
    local sprite = me.sprite
    sprite:load(SPRITE)
    me.anim_loaded = 0xFF
    local anim = if me:param(3) == PLACED then k.anim else 0
    me.anim = anim
    sprite:set_animation(anim)
    sprite:step()
    sprite.shadow = "ground"
    me:set_coordinates_from_panel()
    me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
    me.future_panel_x, me.future_panel_y = me.panel_x, me.panel_y
    me.element = ELEMENTS[k.element]
    me.hp, me.max_hp = k.hp, k.hp
    me.name_id = k.name_id
    me.timer = LIFETIME
    if not me:create_collision() then
        me:free()
        return
    end
    me:setup_collision(SELF_TYPE, TARGET_TYPE, HIT_MOD)
    me.collision.hit_effect = HIT_SPARK
    me.collision.region = 0
    me.collision:present()
    me.lifecycle = "update"
    tick(me)
end

-- `sub_80CF954`.
function rock.update(me: Object)
    local l = me.lifecycle
    if l == "init" then
        init(me)
    elseif l == "update" then
        tick(me)
    else
        me:destroy()
    end
end

return rock
