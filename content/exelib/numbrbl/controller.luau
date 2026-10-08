--!strict
-- NumbrBl's dimming controller (effect object #0x69 in the three games: EXE6's
-- `sub_80E7F38`, docs/engine/dimming-chip-effects.md §15; EXE5's
-- 0x080E82DC; EXE4's 0x080E9838), made with a game's effect
-- (`controller.make`). It runs a navi chip's phases, not the dimming chips'
-- (an AntiNavi check, which a chip that isn't a navi chip passes straight
-- on, and the navi telop, which skips the effect only when the user was
-- deleted), with an effect that sends NumberMan out and waits until he is
-- done: EXE6's draws him as the user; EXE5's and EXE4's warp the user out
-- and the real NumberMan in (`controller.warping`).
--
-- NumbrBl's `dimming` hook is `controller.hook(kind, balls)`.

local dimming_chips = require("@exelib/dimming")
local navi_chips = require("@exelib/navi_chips/controller")

local controller = {}

export type State = {
    -- The chip the telop shows and AntiNavi checks (object +0x30), and
    -- its Atk+ / cross bonus (+0x32), which adds to NumberMan's damage
    -- word.
    chip: Chip?,
    bonus: number,
    -- The balls NumberMan sets down (the chip's first parameter; EXE5's
    -- NumberMan sets down three more).
    balls: number,
    -- NumberMan is out (the original's second parameter, whose address he
    -- holds).
    busy: boolean,
}

-- The controller's actions (EXE6's `off_80E7F6C`, EXE5's 0x080E8310).
local DIM, ANTI_NAVI, TELOP, EFFECT, UNDIM = 0x0, 0x4, 0x8, 0xC, 0x10

-- The action after the effect: the undim.
controller.UNDIM = UNDIM

-- A controller kind `id` whose effect (its action 0xC) is `effect`.
function controller.make(id: string, effect: (me: Object) -> ()): Kind
    return new.kind {
        id = id,
        pool = "effect",
        state = { chip = "chip", bonus = "u16", balls = "u8", busy = "bool" },
        -- `sub_80E7F38`: `object_timefreezeBegin` at its init; by its action
        -- `object_dimScreen`, `sub_800BDB2` (AntiNavi), `sub_800BA8A` (the
        -- navi telop), the effect and `object_undimScreen`; and
        -- `object_timefreezeEnd` when it is destroyed.
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                dimming.begin(me)
            elseif l == "update" then
                local s = me.state :: State
                local a = me.action
                if a == DIM then
                    dimming.dim_screen(me)
                elseif a == ANTI_NAVI then
                    navi_chips.anti_navi(me, navi_chips.in_block(s.chip))
                elseif a == TELOP then
                    navi_chips.name(me, s.chip, navi_chips.in_block(s.chip))
                elseif a == EFFECT then
                    effect(me)
                elseif a == UNDIM then
                    dimming.undim_screen(me)
                else
                    error(string.format("NumbrBl's controller: action %#x reads past its table", a))
                end
            else
                dimming.finish(me)
            end
        end,
    }
end

local WARP_TICKS = 0x1E

-- EXE5's effect (0x080E8324; EXE4's 0x080E9754, the same with its own warp), by its phase: the user warps out
-- (`warp`, 30 ticks); NumberMan on the controller's panel (`bring`, with the damage plus the bonus and the chip's
-- balls) until he is done; 30 ticks; the user warps back in (30 ticks); then the undim.
function controller.warping(
    warp: (me: Object, out: boolean) -> (),
    bring: (me: Object, damage: number, balls: number) -> ()
): (me: Object) -> ()
    -- 30 ticks from the phase's first, then phase `next`.
    local function wait(me: Object, next: number)
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = next, 0
        end
    end

    return function(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            -- EXE5's 0x080E8348: the warp out.
            if me.phase_init == 0 then
                warp(me, true)
                me.timer = WARP_TICKS
                me.phase_init = 4
            end
            wait(me, 4)
        elseif phase == 4 then
            -- EXE5's 0x080E836E: NumberMan, with the chip's balls; once he is done, on.
            if me.phase_init == 0 then
                bring(me, dimming_chips.damage_plus(me, s.bonus), s.balls)
                me.phase_init = 4
            end
            if not s.busy then
                me.phase, me.phase_init = 8, 0
            end
        elseif phase == 8 then
            -- EXE5's 0x080E839E: 30 ticks.
            if me.phase_init == 0 then
                me.timer = WARP_TICKS
                me.phase_init = 4
            end
            wait(me, 0xC)
        elseif phase == 0xC then
            -- EXE5's 0x080E83BC: the warp back in; 30 ticks, then the undim.
            if me.phase_init == 0 then
                warp(me, false)
                me.timer = WARP_TICKS
                me.phase_init = 4
            end
            local t = me.timer - 1
            me.timer = t
            if t < 0 then
                me:set_action(UNDIM)
            end
        else
            error(string.format("NumbrBl's controller: phase %#x of its effect reads past its table", phase))
        end
    end
end

-- `sub_80E7FBA` (EXE5's 0x080E83E6): NumbrBl's `dimming` hook, a controller
-- of `kind` for `balls` balls.
function controller.hook(kind: Kind, balls: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            local s = o.state :: State
            s.chip, s.bonus, s.balls = spec.chip, spec.bonus, balls
        end
        return o
    end
end

return controller
