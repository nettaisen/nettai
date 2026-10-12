--!strict
-- RedSun's and BlueMoon's dimming controller (EXE4's own: effect objects #0x5A, 0x080E8534, and #0x5B, 0x080E865C, the
-- same code but for what they send out): the usual dimming phases (@exelib/dimming: dim, name, effect, undim), with an
-- effect whose course is a navi chip's long one: the user warps out (./navi_chips' warp, 0x080E2D26) and 31 ticks
-- pass; the sun or the moon comes, with the damage plus the bonus, and the effect waits until it is done; 31 ticks; the
-- user warps back in, 31 ticks, and the undim. The chips' hooks (0x080E863A, 0x080E8760) take the user's side alone.
-- (Their damage is the rules', ../rules/sun_and_moon.)

local dimming_chips = require("@exelib/dimming")
local navi_chips = require("./navi_chips")

local sun_and_moon = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the attack's damage adds.
    bonus: number,
    -- The sun or the moon is out (+0x0C: its spawner raises the flag, its end lowers it).
    busy: boolean,
}

export type Spec = {
    -- The kind's id, and its name for the errors past its tables.
    id: string,
    name: string,
    -- The sun or the moon for the controller, with the damage word: nil when it couldn't come. It lowers the
    -- controller's `busy` as it goes.
    attack: (controller: Object, damage: number) -> Object?,
}

-- Each wait falls below 0 a tick after it says.
local WAIT = 0x1E
-- The effect's phases.
local WARP_OUT, ATTACK, AFTER, WARP_IN = 0x0, 0x4, 0x8, 0xC

-- Count the timer down: true once it fell below 0.
local function ran_out(me: Object): boolean
    local t = me.timer - 1
    me.timer = int.u16(t)
    return t < 0
end

local function next_phase(me: Object, phase: number)
    me.phase, me.phase_init = phase, 0
end

function sun_and_moon.make(spec: Spec): { kind: Kind, hook: (user: Object, chip: DimmingChipSpec) -> Object? }
    -- 0x080E8578 (0x080E86A0): the effect, by phase.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == WARP_OUT then
            -- 0x080E859C
            if me.phase_init == 0 then
                navi_chips.warp(me.alliance, true)
                me.timer = WAIT
                me.phase_init = 4
            end
            if ran_out(me) then
                next_phase(me, ATTACK)
            end
        elseif phase == ATTACK then
            -- 0x080E85C2
            if me.phase_init == 0 then
                if spec.attack(me, dimming_chips.damage_plus(me, s.bonus)) then
                    s.busy = true
                end
                me.phase_init = 4
            end
            if not s.busy then
                next_phase(me, AFTER)
            end
        elseif phase == AFTER then
            -- 0x080E85F2
            if me.phase_init == 0 then
                me.timer = WAIT
                me.phase_init = 4
            end
            if ran_out(me) then
                next_phase(me, WARP_IN)
            end
        elseif phase == WARP_IN then
            -- 0x080E8610
            if me.phase_init == 0 then
                navi_chips.warp(me.alliance, false)
                me.timer = WAIT
                me.phase_init = 4
            end
            if ran_out(me) then
                dimming_chips.done(me)
            end
        else
            error(string.format("%s's controller: phase %#x reads past its table", spec.name, phase))
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16", busy = "bool" },
        update = dimming_chips.phases { name = spec.name, effect = effect },
    }

    local function hook(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, chip, true)
        if o then
            (o.state :: State).bonus = chip.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

-- The attack is done: its controller goes on (the flag through its pointer lowered).
function sun_and_moon.done(controller: Object?)
    if controller and controller.active then
        (controller.state :: State).busy = false
    end
end

return sun_and_moon
