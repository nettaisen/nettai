--!strict
-- EXE4's guard (action 0x25, 0x080ECCF2; EXE5's 0x1F, 0x080ECAE6, grew from it): the NaviCust's Shield (variant 3)
-- and Reflect (4) programs' B+Left special, and the Guard chips' (0 to 2). The navi holds up a shield
-- (../objects/shield) for the guard's ticks (its parameters' first byte) and guards; a hit the guard turns aside
-- (the collision's guard byte, +0x26) is countered once, from the panel ahead with the attack's damage word. Each user
-- states its own counter (the original picks it by the variant):
--
-- - the Reflect program's: the charged shot's projectile (row 6), 20 pixels up (navis/megaman/weapons/guards);
-- - the Shield program's: none (variant 3 never looks at what it turns aside);
-- - the Guard chips': a shock wave (0x080CFD2A, its row 6: chips/guard).
--
-- A guard byte with bit 1 takes the shield down, the guard with it (EXE4's collision only ever writes 1 there,
-- 0x08012BA6: `reactions.hit_test.guard_marks_direction`).

local shield = require("../objects/shield")

local guard = {}

-- What a guard counters with, from the panel ahead of the navi (x, y), with the attack's damage word.
export type Counter = (me: Object, x: number, y: number, damage: number) -> ()

export type Spec = {
    id: string,
    -- The guard's ticks (its parameters word's first byte), its shield's look (../objects/shield's, the look row its
    -- second byte names), and its counter (none: it never looks at what it turns aside).
    ticks: number,
    shield: ReflectorShieldLook,
    counter: Counter?,
}

export type State = {
    -- Ticks guarded (AIAttackVars+0x10).
    timer: number,
}

-- The shield's offset from its navi by the navi's number (NaviStats +0x23, 0x080ECDF0): MegaMan's 16 pixels ahead,
-- level. (EXE4's content has no other navi.)
local SHIELD_X, SHIELD_Z = 16, 0
-- The guard byte's bit that takes the shield down.
local DROPS = 0x2

-- The shield goes, and the guard with it.
local function drop_shield(me: Object)
    me.related1 = nil
    me:set_status("guard", false)
end

function guard.action(spec: Spec): Action
    local counter = spec.counter

    local function guarding(me: Object, s: State)
        if me.step_init == 0 then
            shield.spawn(me, Vec3.px(SHIELD_X * me.facing, 0, SHIELD_Z), spec.shield)
            me:set_status("guard", true)
            me:set_status("using_action", true)
            me.anim = 0
            me.marker = 0
            me.step_init = 4
            s.timer = 0
            return
        end
        local guarded = me.collision.guard_dirs
        if counter and guarded ~= 0 then
            if bit32.btest(guarded, DROPS) then
                drop_shield(me)
            elseif me.marker == 0 then
                me.marker = 1
                local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
                counter(me, me.panel_x + me.facing, me.panel_y, damage)
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
        -- 0x080ECCF2: one phase.
        update = function(me: Object, s: State)
            local step = me.step
            if step ~= 0 then
                error(string.format("the guard's phase %#x reads past 0x080ECD08", step))
            end
            guarding(me, s)
        end,
    }
end

return guard
