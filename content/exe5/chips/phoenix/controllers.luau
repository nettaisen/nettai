--!strict
-- Phoenix's and DethPhnx's dimming controllers (EXE5's effect objects #0x0D, 0x080E1590, and #0x15, 0x080E1E9C;
-- EXE5's own): a navi chip's controller (EXE6's `sub_80E17E8`, the engine's) but for its effect. Its actions: the dim,
-- AntiNavi's check, the chip's name as a navi chip's (`sub_800BA8A`), the effect, the undim. The effect (0x080E1F08):
-- 31 ticks; the chip's phoenix, which it waits on; 31 ticks; 31 more. The user stays on the field throughout (the
-- navi chip's controller warps it out and back), and nothing is kept as the last navi chip. Each chip makes its own
-- (`controllers.make`: ./controller, ../dethphnx/controller).

local dimming_chips = require("@exelib/dimming")

local controllers = {}

export type State = {
    -- The chip, which its telop names and AntiNavi checks (+0x30), and its Atk+ bonus (+0x32), which the phoenix's
    -- damage adds.
    chip: Chip?,
    bonus: number,
    -- The phoenix is out (+0x18): set by its spawn, cleared when it is done.
    busy: boolean,
}

export type Spec = {
    -- The kind's id, and its name for the errors past its tables.
    id: string,
    name: string,
    -- The chip's phoenix for `user`, on the controller's panel, of the user's side and facing, with the damage word
    -- (bonus included), reporting to the controller (its `done`): EXE5's 0x080BFCC4 and 0x080C00B6.
    spawn: (controller: Object, user: Object, damage: number) -> Object?,
}

export type Controller = {
    kind: Kind,
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

-- The actions: the dim, AntiNavi, the name, the effect, the undim.
local DIM, ANTI_NAVI, NAME, EFFECT = 0x0, 0x4, 0x8, 0xC
local WAIT = 0x1E

-- The phoenix is done (it writes 0 through the pointer its spawn was given).
function controllers.done(me: Object)
    (me.state :: State).busy = false
end

-- 30 ticks from the phase's first, then `next`.
local function wait(me: Object, next: () -> ())
    if me.phase_init == 0 then
        me.timer = WAIT
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left < 0 then
        next()
    end
end

function controllers.make(spec: Spec): Controller
    -- 0x080E1EE4 (DethPhnx's; Phoenix's 0x080E15D8), by phase.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            wait(me, function()
                me.phase, me.phase_init = 4, 0
            end)
        elseif phase == 4 then
            -- The phoenix, with the damage plus the bonus.
            if me.phase_init == 0 then
                if spec.spawn(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus)) then
                    s.busy = true
                end
                me.phase_init = 4
            end
            if not s.busy then
                me.phase, me.phase_init = 8, 0
            end
        elseif phase == 8 then
            wait(me, function()
                me.phase, me.phase_init = 0xC, 0
            end)
        elseif phase == 0xC then
            wait(me, function()
                me:set_action(0x10)
            end)
        else
            error(string.format("%s's controller: effect phase %#x reads past its table", spec.name, phase))
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { chip = "chip", bonus = "u16", busy = "bool" },
        update = function(me: Object)
            local l = me.lifecycle
            if l == "init" then
                dimming.begin(me)
            elseif l == "update" then
                local a = me.action
                local chip = (me.state :: State).chip
                if a == DIM then
                    dimming.dim_screen(me)
                elseif a == ANTI_NAVI then
                    dimming.check_anti_navi(me, chip)
                elseif a == NAME then
                    dimming.show_navi_telop(me, chip)
                elseif a == EFFECT then
                    effect(me)
                elseif a == 0x10 then
                    dimming.undim_screen(me)
                else
                    error(string.format("%s's controller: action %#x reads past its table", spec.name, a))
                end
            else
                dimming.finish(me)
            end
        end,
    }

    -- The chip's `dimming` hook (the dimming spawner's entries 1 and 2: 0x080E168E, 0x080E1F9A): the controller, on
    -- the user's panel, of its side and facing, with the chip and its bonus.
    local function hook(user: Object, dspec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, dspec)
        if o then
            local s = o.state :: State
            s.chip = dspec.chip
            s.bonus = dspec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controllers
