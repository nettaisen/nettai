--!strict
-- MetrKnuk's dimming controller, as EXE6 and EXE5 share it (EXE6's effect
-- object #0x76, `sub_80E8B00`; EXE5's #0x76, 0x080E9188;
-- docs/engine/dimming-chip-effects.md §18): the usual dimming phases
-- (../dimming), with an effect that throws 16 fists (./fist) at the enemy's
-- area, 8 ticks apart, at random panels around the enemies (never the last
-- one twice in a row). EXE6's first goes at a random panel an enemy stands
-- on, and with no enemy standing the others go at the enemy's area; EXE5's
-- first goes around the enemies too, and with none standing no fist goes.
-- EXE5's fists are null. Each game makes its kind (`controller.make`).

local dimming_chips = require("../dimming")
local panels = require("../panels")
local targets = require("./targets")

local controller = {}

-- A game's MetrKnuk.
export type Spec = {
    id: string,
    -- A fist over (x, y) at height z with the damage word: the game's.
    fist: (spawner: Object, x: number, y: number, element: number, z: number, damage: number) -> Object?,
    -- EXE6's: the first fist at an enemy's own panel (`sub_80E8C44`), and the
    -- enemy's whole area when no enemy stands (`sub_80E8CFA`).
    first_on_body: boolean,
    area_fallback: boolean,
    -- EXE6's fists take the chip's element; EXE5's are null (0x080E9268).
    fists_take_element: boolean,
}

export type State = {
    -- The Atk+ / cross bonus (object +0x32): it adds to the fists' damage
    -- word.
    bonus: number,
    -- The last target (ExtraVars+0) and the panels around the enemies (the
    -- object's +0x64, 0x28 bytes past its variables): ./targets'.
    last: number,
    grid: { number },
}

local PUNCHES = 16
-- The ticks to the next punch, less one.
local BETWEEN = 6
-- The height the fists fall from.
local HEIGHT = 0x78_0000

-- A game's MetrKnuk controller: its kind, and the chip's `dimming` hook.
function controller.make(spec: Spec): { kind: Kind, hook: (user: Object, dspec: DimmingChipSpec) -> Object? }
    -- Where its fists go (./targets): an enemy's panels are those of the
    -- enemy's area it stands on (`byte_80E8D6C`).
    local aim = { body = targets.BODY_IN_AREA, area_fallback = spec.area_fallback }

    -- `sub_80E8B70`: a fist at the target; after the 16th the effect is
    -- over, else 7 ticks to the next.
    local function punch(me: Object)
        local s = me.state :: State
        local target = if spec.first_on_body and me.timer2 == PUNCHES
            then targets.first(me.alliance, aim)
            else targets.next(me.alliance, s, aim)
        s.last = target
        local x = panels.x(target)
        if x ~= 0 then
            -- sub_80E8BE2 (the punch's parity, Timer2 & 1, the fist's first
            -- parameter, is read by nothing).
            local element = if spec.fists_take_element then me.element else 0
            spec.fist(me, x, panels.y(target), element, HEIGHT, dimming_chips.damage_plus(me, s.bonus))
        end
        local n = me.timer2 - 1
        me.timer2 = n
        if n <= 0 then
            dimming_chips.done(me)
            return
        end
        me.timer = BETWEEN
        me.phase, me.phase_init = 8, 0
    end

    -- `sub_80E8BB0`: wait (the first time, what the dim left on the timer).
    local function wait(me: Object)
        local t = me.timer - 1
        me.timer = t
        if t < 0 then
            me.phase, me.phase_init = 4, 0
        end
    end

    -- `sub_80E8B44`: by phase: 0 the punches' count, 4 a punch, 8 the wait.
    local function effect(me: Object)
        local phase = me.phase
        if phase == 0 then
            -- sub_80E8B64.
            me.timer2 = PUNCHES
            me.phase, me.phase_init = 8, 0
        elseif phase == 4 then
            punch(me)
        elseif phase == 8 then
            wait(me)
        else
            error(string.format("MetrKnuk's controller: phase %#x reads past off_80E8B58", phase))
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16", last = targets.LAST, grid = targets.GRID },
        -- `sub_80E8B00`.
        update = dimming_chips.phases { name = "MetrKnuk", effect = effect },
    }

    -- `sub_80E8BC0`: MetrKnuk's `dimming` hook.
    local function hook(user: Object, dspec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, dspec)
        if o then
            (o.state :: State).bonus = dspec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
