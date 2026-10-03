--!strict
-- BN5's guard (action 0x1F, 0x080ECAE6): the Guard chips (subtypes 0 to 2), the NaviCust's Shield (3) and Reflect (4)
-- programs and the CrsShld chips (5 to 7). BN6's Reflectors' guard (`sub_80ED13E`, action 0x2B) is its nearest code
-- (@bn6/chips/rflectr/guard); BN5's differs in its counters. The navi holds up a shield (@common/rflectr/shield, BN5's
-- looks below) for the chip's ticks (its first parameter) and guards. The first hit the guard blocks from the front
-- sends the counterattack:
--
-- - the Guard chips': a shock wave back along the row (lib/guard/wave, its row 0xC: 0x080ECC10's first byte for
--   all three) with the chip's damage;
-- - the Reflect program's: the charged shot's projectile, 20 pixels up;
-- - the Shield program's (in battle flag 0x40 mode only: it heeds nothing otherwise): 0x580 into the side's gauge;
-- - the CrsShld chips': a cross (CROSS below) leaving from the panel ahead, after which the guard ends at once.
--
-- A hit that breaks guards takes the shield down, the guard with it (the time still runs). The NaviCust programs'
-- uses come with BN5's NaviCust.

local collision = require("../../rules/collision")
local projectile = require("../../objects/projectile/projectile")
local shield = require("@common/rflectr/shield")
local sparks = require("../sparks")
local variants = require("../../objects/projectile/variants")
local wave = require("./wave")

local guard = {}

-- ---- The shields --------------------------------------------------------------------------------------------------

-- The shield (attack object #0x2B, 0x080CA22C: BN6's `sub_80C96A0`, the same code).
local SHIELD = shield.make { id = "bn5:rflectr/shield", sound = asset.sound("bn5:follow-effect") }

-- Its looks (0x080CA1F0's rows, by the guard's second parameter: BN6's `byte_80C9664`'s but for rows 3 to 6, BN5's
-- ProtoMan's and CrsShld's, and row 9's sheet).
local REFLECTOR, CROSS_SHIELD = asset.sprite("bn5:reflector-shield"), asset.sprite("bn5:sprite-04-05")
local function look(l: ReflectorShieldLook): ReflectorShieldLook
    return shield.look(l)
end
guard.looks = {
    guard1 = look { sprite = REFLECTOR, anim = 0, fade_anim = 1, fade_ticks = 0xE, palette = 0 },
    navicust_reflect = look { sprite = REFLECTOR, anim = 2, fade_anim = 3, fade_ticks = 0xE, palette = 0 },
    navicust_shield = look { sprite = REFLECTOR, anim = 4, fade_anim = 5, fade_ticks = 0xE, palette = 0 },
    protoman = look { sprite = asset.sprite("bn5:protoman"), anim = 3, fade_anim = 4, fade_ticks = 4, palette = 7 },
    crsshld1 = look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 0 },
    crsshld2 = look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 2 },
    crsshld3 = look { sprite = CROSS_SHIELD, anim = 3, fade_anim = 4, fade_ticks = 1, palette = 4 },
    guard2 = look { sprite = REFLECTOR, anim = 6, fade_anim = 7, fade_ticks = 0xE, palette = 0 },
    guard3 = look { sprite = REFLECTOR, anim = 8, fade_anim = 9, fade_ticks = 0xE, palette = 0 },
    -- Row 9: ProtoSoul's B+Back with its Chaos Unison charge armed (navis/megaman/forms/protosoul/back).
    chaos_reflect = look { sprite = REFLECTOR, anim = 10, fade_anim = 11, fade_ticks = 0xE, palette = 0 },
}

-- ---- CrsShld's cross ----------------------------------------------------------------------------------------------

-- The cross (attack object #0xA8, 0x080DB258): it flies along the row 20 pixels a tick, probing for navis; on one it
-- stops on its panel, and 8 ticks later strikes it once (an attack, hit modifier 3), then goes.
type CrossState = { palette: number }

local CROSS_SPRITE = asset.sprite("bn5:sprite-04-05")
local CROSS_SPEED = 20
local CROSS_SOUND = asset.sound("bn5:bite")
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

-- 0x080DB27C: on its panel, on the ground, its look (sheet 04-05, animation 1, with a shadow; the palette its
-- parameter, times 2) and its probe (row 0, hitting navis; no damage: the collision's damage word cleared), flying
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
    sprite.palette = (me.state :: CrossState).palette * 2
    me.vel = Vec3.new(me.facing * CROSS_SPEED * 0x10000, me.vel.y, me.vel.z)
    local c = me.collision
    c:set_hit_effect(CROSS_SPARK)
    c.self_damage = 0
    c.bugs = CROSS_BUGS
    c:present()
    me.lifecycle = "update"
    cross_tick(me)
end

local CROSS = define.kind {
    id = "bn5:crsshld/cross",
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
local function cross(owner: Object, x: number, y: number, element: number, damage: number, palette: number): Object?
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

-- ---- The guard ----------------------------------------------------------------------------------------------------

-- What a guard counters with: the Guard chips' shock wave, the Reflect program's shot, the Shield program's gauge
-- (only with battle flag 0x40), or a CrsShld's cross (in its palette).
export type Counter = { kind: "wave" } | { kind: "shot" } | { kind: "gauge" } | { kind: "cross", palette: number }

export type Spec = {
    -- The action's id, for a weapon's (a chip's is derived).
    id: string?,
    -- The guard's ticks (the chip's first parameter), its shield's look, its counter.
    ticks: number,
    look: ReflectorShieldLook,
    counter: Counter,
}

export type State = {
    -- Ticks guarded (AIAttackVars+0x10).
    timer: number,
}

-- The shield sits at the navi's sprite attach point 6.
local SHIELD_POINT = 6
-- A hit that breaks guards (the collision's hit flag 2).
local BREAKS_GUARD = 0x2
-- The Guard chips' wave row, and the Reflect program's shot's height.
local WAVE_ROW, SHOT_Z = 0xC, 0x14_0000
-- What the Shield program puts into the gauge.
local SHIELD_GAUGE = 0x580

-- The shield goes, and the guard with it.
local function drop_shield(me: Object)
    me.related1 = nil
    me.overlay = nil
    me:set_status("guard", false)
end

function guard.action(spec: Spec): Action
    local counter = spec.counter

    local function guarding(me: Object, s: State)
        if me.step_init == 0 then
            local x, z = me:attach_point(SHIELD_POINT)
            SHIELD.spawn(me, Vec3.px(x, 0, z), spec.look)
            me:set_status("guard", true)
            me:set_status("using_action", true)
            me:set_animation(0)
            me.marker = 0
            me.step_init = 4
            s.timer = 0
            return
        end
        local heedless = counter.kind == "gauge" and not battle.per_player_gauges()
        if not heedless and bit32.btest(me.collision.guard_dirs, bit32.lshift(1, me.flip)) then
            if bit32.btest(me.collision.hit_flags, BREAKS_GUARD) then
                drop_shield(me)
            elseif me.marker == 0 then
                me.marker = 1
                local x, y = me.panel_x + me.facing, me.panel_y
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                if counter.kind == "gauge" then
                    battle.add_side_gauge(me.alliance, SHIELD_GAUGE)
                elseif counter.kind == "wave" then
                    wave.spawn(me, x, y, WAVE_ROW, damage)
                elseif counter.kind == "shot" then
                    projectile.spawn(me, x, y, { variant = variants.charged, damage = damage, z = SHOT_Z })
                elseif counter.kind == "cross" then
                    cross(me, x, y, me.attack_element, damage, counter.palette)
                    drop_shield(me)
                    me:exit_attack()
                    return
                end
            end
        end
        local t = s.timer + 1
        s.timer = t
        if t > spec.ticks then
            drop_shield(me)
            me:exit_attack()
        end
    end

    return define.action {
        id = spec.id,
        state = { timer = "u16" },
        args = spec,
        -- 0x080ECAE6: one phase.
        update = function(me: Object, s: State)
            local step = me.step
            if step ~= 0 then
                error(string.format("the guard's phase %#x reads past 0x080ECAF8", step))
            end
            guarding(me, s)
        end,
    }
end

return guard
