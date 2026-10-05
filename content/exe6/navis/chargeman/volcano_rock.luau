--!strict
-- A volcano rock (the original's attack object #0x86, `sub_80D5D54`):
-- ChargeMan's VolcChrg throws it up from his panel (48 pixels up, 10 ahead)
-- onto its target panel in a 45-tick arc, the target highlighted on the
-- way; landing on a panel with a navi, an object or solid ground of the
-- other side (or neutral), it bursts and hits there for 5 ticks. It waits
-- a dimming out, and goes when the battle is over. See
-- docs/engine/standard-chips.md, "Action 0x0A".

local collision = require("../../rules/collision")
local effects = require("../../lib/effects")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

local volcano_rock = {}

local SPRITE = asset.sprite("volcano-rock")
local SOUND_LAND = asset.sound("hit-bomb-1")

-- It starts 10 pixels ahead of its thrower, 48 up.
local AHEAD, HEIGHT = 0xA, 0x30

-- The arc: to its target's center on the ground in 45 ticks, falling by a
-- quarter pixel a tick more each tick.
local FLIGHT_TICKS, GRAVITY = 0x2D, -0x4000

-- What it bursts on, by side (`byte_80D5EA0`: the other side's bodies and
-- objects, neutral objects, and solid ground).
local BURSTS_ON = { [0] = 0x1580_0010, 0x2A80_0010 }

-- Its burst's hit: a thrown attack at navis, fire spark, for 5 ticks.
local HIT_TICKS, HIT_MOD = 5, 3

-- `sub_80D5D78`: on its thrower's panel, ahead and up; with a ground
-- shadow.
local function init(me: Object)
    me:set_coordinates_from_panel()
    local p = me.pos
    local facing = if bit32.bxor(me.alliance, me.flip) == 0 then 1 else -1
    -- The whole parts of X and Z (their halfwords above the fractions).
    local x = bit32.bor(bit32.band(p.x, 0xFFFF), bit32.lshift(bit32.band(bit32.rshift(p.x, 16) + facing * AHEAD, 0xFFFF), 16))
    local z = bit32.bor(bit32.band(p.z, 0xFFFF), bit32.lshift(HEIGHT, 16))
    me.pos = Vec3.new(int.i32(x), p.y, int.i32(z))
    local sprite = me.sprite
    sprite:load(SPRITE)
    sprite.shadow = "ground"
    me.visible = true
    me.anim = 0
    me.anim_loaded = 0
    sprite:set_animation(0)
    sprite.palette = 0
    sprite:set_flip(bit32.bxor(me.alliance, me.flip))
    me:set_lifecycle("update")
end

-- `sub_80D5EDE`: the burst's hit on its panel.
local function hit(me: Object)
    local h = battle.hitbox(me, {
        panel_x = me.panel_x,
        panel_y = me.panel_y,
        element = me.element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.fire,
        target = collision.hits_navis,
        self_type = collision.thrown,
        damage = me.damage,
        stamina = me.stamina,
        hit_mod = HIT_MOD,
    })
    if h then
        h.timer = HIT_TICKS
    end
end

-- `sub_80D5DF4`: the flight. Its entry aims the arc; each tick after it
-- flies while the timer, counted down, stays at 0 or more; then it bursts
-- if it can, and goes.
local function fly(me: Object)
    if me.phase == 0 then
        me.phase = 4
        local x, y = field.center(me.future_panel_x, me.future_panel_y)
        me.vel = trajectory.in_ticks(me.pos, x, y, 0, FLIGHT_TICKS, GRAVITY)
        me.timer = FLIGHT_TICKS
        me.timer2 = 0
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t < 0 then
        if bit32.btest(field.flags(me.panel_x, me.panel_y), BURSTS_ON[me.alliance]) then
            battle.play_sound(SOUND_LAND)
            battle.effect(me.pos, effects.explosion)
            hit(me)
        end
        me:set_lifecycle("destroy")
        me.timer2 = 0
        return
    end
    local n = me.timer2 + 1
    me.timer2 = n
    if not bit32.btest(n, 4) then
        field.highlight(me.future_panel_x, me.future_panel_y)
    end
    local v = me.vel
    me.pos = me.pos + v
    me.vel = Vec3.new(v.x, v.y, v.z + GRAVITY)
    me:set_panel_from_coordinates()
end

-- `sub_80D5DC8`: gone when the battle is over; still while dimmed.
local function tick(me: Object)
    if battle.over() then
        me:set_lifecycle("destroy")
        return
    end
    if battle.dimmed() then
        return
    end
    if me.action ~= 0 then
        error(string.format("volcano rock action %#x reads past off_80D5DF0", me.action))
    end
    fly(me)
end

volcano_rock.kind = define.kind {
    id = "chargeman/volcano-rock",
    pool = "attack",
    -- `sub_80D5D54`.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:destroy()
            return
        end
        me:update_sprite_while_paused()
    end,
}

-- `sub_80D5EB0`: a rock from `owner`'s panel onto (x, y), with `element`
-- and the damage word `damage`. It runs while dimmed (and waits). `z` is
-- the register its spawner leaves for its position's Z, whose fraction its
-- init keeps.
function volcano_rock.spawn(owner: Object, x: number, y: number, element: number, damage: number, z: number): Object?
    local o = battle.spawn(volcano_rock.kind, Vec3.new(y, element, z))
    if o then
        o.future_panel_x, o.future_panel_y = x, y
        o.panel_x, o.panel_y = owner.panel_x, owner.panel_y
        o.element = element
        o:set_damage_word(damage)
        o.alliance, o.flip = owner.alliance, owner.flip
        o.run_while_dimmed = true
    end
    return o
end

return volcano_rock
