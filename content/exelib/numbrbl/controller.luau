--!strict
-- NumbrBl's dimming controller (effect object #0x69 in both games: EXE6's
-- `sub_80E7F38`, docs/engine/dimming-chip-effects.md §15; EXE5's
-- 0x080E82DC), made with a game's effect (`controller.make`). It runs a
-- navi chip's phases, not the dimming chips' (an AntiNavi check, which a
-- chip that isn't a navi chip passes straight on, and the navi telop, which
-- skips the effect only when the user was deleted), with an effect that
-- sends NumberMan out and waits until he is done: EXE6's draws him as the
-- user, EXE5's warps the user out and the real NumberMan in.
--
-- NumbrBl's `dimming` hook is `controller.hook(kind, balls)`.

local dimming_chips = require("@exelib/dimming")

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
                    dimming.check_anti_navi(me, s.chip)
                elseif a == TELOP then
                    dimming.show_navi_telop(me, s.chip)
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
