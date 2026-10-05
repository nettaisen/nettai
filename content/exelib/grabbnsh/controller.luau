--!strict
-- GrabBnsh's and GrabRvng's dimming controller, as EXE5 and EXE6 share it
-- (EXE6's effect object #0x22, `sub_80E2CA4`; EXE5's #0x22;
-- docs/engine/dimming-chip-effects.md §3): each game makes its own
-- (`controller.make`: its kind, its hand's, the flash and the sound of the
-- panels taken back). The usual dimming phases (@exelib/dimming), with an
-- effect that takes back the
-- panels the other side stole from the user's (they return after the
-- dimming) and, every 31 ticks, drops a hand (./hand) on every panel an
-- enemy navi stands on, once for each panel taken back.

local dimming_chips = require("../dimming")
local hand = require("./hand")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the hands'
    -- damage adds.
    bonus: number,
    -- The strikes left (ExtraVars+0): one per panel taken back.
    strikes: number,
}

-- A game's controller: its kind's key, its hand's kind, the flash on each
-- panel taken back (EXE6's effect 0x3B) and the sound of the taking back.
export type Spec = { id: string, hand: Kind, returned: Effect, return_sound: Sound }

local INTERVAL, AFTER = 0x1E, 0x3C
-- Where the hands go (`byte_80E2E24`, by side): the other side's player
-- navi's panels.
local TARGETS: { [number]: { number } } = { [0] = { 0x0020_0000, 0 }, [1] = { 0x0040_0000, 0 } }

-- `sub_80E2D98`: each of the side's own panels the other side holds
-- (`sub_800D618`) comes back when the dimming is over (its column's return
-- timer 1), with a flash; how many.
local function take_back(me: Object, RETURNED: Effect): number
    local n = 0
    for x = 1, 6 do
        for y = 1, 3 do
            local p = field.panel(x, y)
            if p and p.home ~= p.alliance and p.home == me.alliance then
                local cx, cy = field.center(x, y)
                battle.effect(Vec3.new(cx, cy, 0), RETURNED)
                field.set_column_timer(x, 1)
                n += 1
            end
        end
    end
    return n
end

-- `sub_80E2DE0`: a hand on every enemy navi's panel (from the far column's
-- bottom back); how many.
local function strike(me: Object, hand_kind: Kind): number
    local rule = TARGETS[me.alliance]
    if not rule then
        error(string.format("GrabBnsh's side %d reads past byte_80E2E24", me.alliance))
    end
    local damage = dimming_chips.damage_plus(me, (me.state :: State).bonus)
    local n = 0
    for x = 6, 1, -1 do
        for y = 3, 1, -1 do
            if field.check(x, y, rule[1], rule[2]) then
                hand.spawn(hand_kind, me, x, y, me.element, damage, n == 0)
                n += 1
            end
        end
    end
    return n
end

-- `sub_80E2CE8`, by phase: take the panels back (none: the effect ends);
-- strike every 31 ticks while there are strikes left and targets; 61
-- ticks.
local function effect(me: Object, spec: Spec)
    local s = me.state :: State
    local phase = me.phase
    if phase == 0 then
        s.strikes = take_back(me, spec.returned)
        me.timer = INTERVAL
        if s.strikes == 0 then
            dimming_chips.done(me)
            return
        end
        battle.play_sound(spec.return_sound)
        me.phase, me.phase_init = 4, 0
    elseif phase == 4 then
        local t = me.timer - 1
        me.timer = t
        if t >= 0 then
            return
        end
        me.timer = INTERVAL
        if strike(me, spec.hand) ~= 0 then
            s.strikes -= 1
            if s.strikes ~= 0 then
                return
            end
        end
        me.phase, me.phase_init = 8, 0
    elseif phase == 8 then
        if me.phase_init == 0 then
            me.timer = AFTER
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            dimming_chips.done(me)
        end
    else
        error(string.format("GrabBnsh's effect: phase %#x reads past off_80E2CFC", phase))
    end
end

-- A game's controller: the chips' `dimming` hook (`sub_80E2D76`): the
-- controller, of the user's side (its facing not copied).
function controller.make(spec: Spec): { hook: (user: Object, chip: DimmingChipSpec) -> Object? }
    local kind = define.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16", strikes = "u32" },
        -- `sub_80E2CA4`.
        update = dimming_chips.phases {
            name = "GrabBnsh",
            effect = function(me: Object)
                effect(me, spec)
            end,
        },
    }

    local function hook(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, chip, true)
        if o then
            (o.state :: State).bonus = chip.bonus
        end
        return o
    end

    return { hook = hook }
end

return controller
