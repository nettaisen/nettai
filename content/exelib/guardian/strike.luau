--!strict
-- A Guardian statue's strike-back dimming (effect object #0x53: EXE6's `sub_80E680C`, EXE5's 0x080E6890, the same code;
-- docs/engine/dimming-chip-effects.md §5): the controller the statue starts when an attack breaks it. The usual
-- dimming phases (its telop names Punisher, chip 0x175 in both games), with an effect that tells the statue to strike
-- and waits 60 ticks. A maker taking a game's kind id and telop.

local dimming_chips = require("@exelib/dimming")

local strike = {}

-- A game's strike: its kind's id and the chip its telop names (Punisher).
export type StrikeLook = { id: string, punisher: Chip }

-- The strike made for a game (what @exelib/guardian/statue's look takes as its `strike`).
export type Strike = {
    kind: Kind,
    -- `sub_80E6878`: the strike's dimming controller for `statue`.
    spawn: (statue: Object) -> Object?,
    -- What its telop names.
    telop: DimmingTelop,
}

-- What it tells the statue (the statue's state).
type StatueState = { strike: boolean }

local WAIT = 0x3C

function strike.make(look: StrikeLook): Strike
    -- EXE6's `sub_80E6850`: the statue strikes; 60 ticks.
    local function effect(me: Object)
        if me.phase_init == 0 then
            local statue = me.related1
            if not statue then
                error("the Guardian strike's controller has no statue (sub_80E6850 writes through a null pointer)")
            end
            (statue.state :: StatueState).strike = true
            me.timer = WAIT
            me.phase_init = 4
        end
        local t = me.timer - 1
        me.timer = t
        if t == 0 then
            dimming_chips.done(me)
        end
    end

    local kind = define.kind {
        id = look.id,
        pool = "effect",
        -- EXE6's `sub_80E680C`.
        update = dimming_chips.phases { name = "the Guardian strike", effect = effect },
    }

    -- EXE6's `sub_80E6878`: the controller for `statue`, on its panel and side, with its element and damage word. (It
    -- sits at the spawner's registers: the panel's Y, the element, 0.)
    local function spawn(statue: Object): Object?
        local o = battle.spawn(kind, Vec3.new(statue.panel_y, statue.element, 0))
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = statue.panel_x, statue.panel_y
        o.element = statue.element
        o.related1 = statue
        o.alliance = statue.alliance
        o.damage, o.stamina = statue.damage, statue.stamina
        return o
    end

    -- What its telop names (EXE6's `sub_80D4DF0` spawns it with Punisher, bonus 0).
    return { kind = kind, spawn = spawn, telop = { chip = look.punisher } :: DimmingTelop }
end

return strike
