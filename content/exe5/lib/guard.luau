--!strict
-- EXE5's guard (action 0x1F, 0x080ECAE6): the Guard chips (subtypes 0 to 2), the NaviCust's Shield (3) and Reflect (4)
-- programs and the CrsShld chips (5 to 7). EXE6's Reflectors' guard (`sub_80ED13E`, action 0x2B) is its nearest code
-- (its chips/rflectr/guard); EXE5's differs in its counters. The navi holds up a shield (@exelib/rflectr/shield) for the
-- chip's ticks (its first parameter) and guards. The first hit the guard blocks from the front sends the counterattack.
-- Each user states its own (`guard.action { ticks = ..., look = ..., counter = ... }`): the shield's look (a row of
-- 0x080CA1F0, by the guard's second parameter: EXE6's `byte_80C9664`'s but for rows 3 to 6, EXE5's ProtoMan's and
-- CrsShld's, and row 9's sheet) and its counter:
--
-- - the Guard chips': a shock wave back along the row (chips/guard/wave) with the chip's damage;
-- - the Reflect program's (and ProtoMan's and ProtoSoul's B+Back): the charged shot's projectile, 20 pixels up
--   (`guard.reflect`);
-- - the Shield program's: 0x580 into the side's gauge; it heeds what it blocks only in the operation battle (`heeds`);
-- - the CrsShld chips': a cross (chips/crsshld/cross) leaving from the panel ahead, after which the guard ends at once.
--
-- A hit that breaks guards takes the shield down, the guard with it (the time still runs). The NaviCust programs'
-- uses come with EXE5's NaviCust.

local projectile = require("../objects/projectile")
local shield = require("@exelib/rflectr/shield")
local variants = require("../objects/projectile/variants")

local guard = {}

-- ---- The shields --------------------------------------------------------------------------------------------------

-- The shield (attack object #0x2B, 0x080CA22C: EXE6's `sub_80C96A0`, the same code).
local SHIELD = shield.make { id = "rflectr/shield", sound = asset.sound("follow-effect") }

-- A shield's look: its sheet, its animations, the ticks it fades and its palette.
guard.look = shield.look

-- ---- The guard ----------------------------------------------------------------------------------------------------

-- What a guard counters with, one panel ahead of the navi (x, y), with the attack's damage word: true when the guard
-- ends with it.
export type Counter = (me: Object, x: number, y: number, damage: number) -> boolean

export type Spec = {
    -- The action's id, for a weapon's (a chip's is derived).
    id: string?,
    -- The guard's ticks (the chip's first parameter), its shield's look, its counter.
    ticks: number,
    look: ReflectorShieldLook,
    counter: Counter,
    -- Whether it heeds what it blocks now (none: always).
    heeds: (() -> boolean)?,
}

export type State = {
    -- Ticks guarded (AIAttackVars+0x10).
    timer: number,
}

-- The shield sits at the navi's sprite attach point 6.
local SHIELD_POINT = 6
-- A hit that breaks guards (the collision's hit flag 2).
local BREAKS_GUARD = 0x2
-- The Reflect program's shot's height.
local SHOT_Z = 0x14_0000

-- The Reflect program's counter (and ProtoMan's and ProtoSoul's B+Back): the charged shot's projectile, 20 pixels up.
function guard.reflect(me: Object, x: number, y: number, damage: number): boolean
    projectile.spawn(me, x, y, { variant = variants.charged, damage = damage, z = SHOT_Z })
    return false
end

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
        local heedless = spec.heeds ~= nil and not spec.heeds()
        if not heedless and bit32.btest(me.collision.guard_dirs, bit32.lshift(1, me.flip)) then
            if bit32.btest(me.collision.hit_flags, BREAKS_GUARD) then
                drop_shield(me)
            elseif me.marker == 0 then
                me.marker = 1
                local x, y = me.panel_x + me.facing, me.panel_y
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                if counter(me, x, y, damage) then
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

    return new.action {
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
