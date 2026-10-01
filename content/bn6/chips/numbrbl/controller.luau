--!strict
-- NumbrBl's dimming controller (the original's effect object #0x69,
-- `sub_80E7F38`; docs/engine/dimming-chip-effects.md §15). It runs a navi
-- chip's phases, not the dimming chips' (an AntiNavi check, which a chip
-- that isn't a navi chip passes straight on, and the navi telop, which
-- skips the effect only when the user was deleted), with an effect that
-- sends NumberMan (./numberman) out in the user's place and waits until
-- he is done.
--
-- NumbrBl's `dimming` hook is `controller.hook(balls)`.

local dimming_chips = require("../../lib/dimming")
local numberman = require("./numberman")

local controller = {}

export type State = {
    -- The chip the telop shows and AntiNavi checks (object +0x30), and
    -- its Atk+ / cross bonus (+0x32), which adds to NumberMan's damage
    -- word.
    chip: number,
    bonus: number,
    -- The balls NumberMan sets down (the chip's first parameter).
    balls: number,
    -- NumberMan is out (the original's second parameter, whose address he
    -- holds).
    busy: boolean,
}

-- The controller's actions (`off_80E7F6C`).
local DIM, ANTI_NAVI, TELOP, EFFECT, UNDIM = 0x0, 0x4, 0x8, 0xC, 0x10

-- `sub_80E7F80`: NumberMan; once he is done, undim.
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        numberman.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus), s.balls)
        me.phase_init = 4
    end
    if not s.busy then
        me:set_action(UNDIM)
    end
end

controller.kind = define.kind {
    id = "numbrbl/controller",
    pool = "effect",
    state = { chip = "u16", bonus = "u16", balls = "u8", busy = "bool" },
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
                error(string.format("NumbrBl's controller: action %#x reads past off_80E7F6C", a))
            end
        else
            dimming.finish(me)
        end
    end,
}

-- `sub_80E7FBA`: NumbrBl's `dimming` hook, for `balls` balls.
function controller.hook(balls: number): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(controller.kind, user, spec)
        if o then
            local s = o.state :: State
            s.chip, s.bonus, s.balls = spec.chip, spec.bonus, balls
        end
        return o
    end
end

return controller
