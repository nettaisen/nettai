--!strict
-- MetaGel's dimming controller (effect object #0x21: EXE5's 0x080E2D98, the dimming handler's subtype 6, 0x080E2E9C;
-- EXE4's 0x080E4618, its variant 0x0C, 0x080E4728), made for a game's look (`controller.make`): the usual dimming
-- phases (@exelib/dimming), with an effect (EXE5's 0x080E2DDC, EXE4's 0x080E465C) that drops a gel (@exelib/metagel/gel)
-- on each row's front panel of the other side's area, one every 13 ticks in a shuffled order of the rows, then waits
-- until they are all gone. Which panel is the row's front is the game's (`look.front`): EXE5's the column past the
-- user's area (0x0800BA34, `panels.area_front`), EXE4's the other side's panel nearest the user's (0x0800A1AE,
-- `controller.nearest_other`).

local dimming_chips = require("@exelib/dimming")
local panels = require("@exelib/panels")

local controller = {}

export type State = {
    -- The Atk+ bonus (object +0x32): it adds to the gels' damage word.
    bonus: number,
    -- The rows left (ExtraVars), their order (ExtraVars+4), and the gels still out (ExtraVars+8, a byte each).
    left: number,
    order: { number },
    out: { boolean },
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The game's gel's spawn (@exelib/metagel/gel).
    gel: (holder: Object, x: number, y: number, damage: number, slot: number) -> Object?,
    -- The column of row `y` a gel of `me`'s drops on; 0 for none (the row passed).
    front: (me: Object, y: number) -> number,
}

export type Controller = {
    kind: Kind,
    -- MetaGel's `dimming` hook.
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

local GEL_TICKS = 12

-- EXE5's front: past the user's area, as it faces (0x0800BA34).
function controller.past_own_area(me: Object, y: number): number
    return panels.area_front(me.alliance, me.flip, y)
end

-- EXE4's front: from the far edge of the other side's field back toward the user's, the first panel of the other
-- side's (0x0800A1AE with 0x080E4708's flags: the owner bit 0x20 for side 1, none for side 0).
function controller.nearest_other(me: Object, y: number): number
    local other = bit32.bxor(me.alliance, 1)
    local require, forbid = if other == 1 then 0x20 else 0, if other == 1 then 0 else 0x20
    return panels.closest_in_row(other, y, require, forbid)
end

function controller.make(look: Look): Controller
    -- EXE5's 0x080E2DDC, by phase.
    local function effect(me: Object)
        local s = me.state :: State
        local phase = me.phase
        if phase == 0 then
            -- EXE5's 0x080E2DFC: three rows, shuffled (four rounds).
            s.left = 3
            for k = 1, 4 do
                s.out[k] = false
            end
            local order = { 1, 2, 3 }
            panels.shuffle(order, 3, 4)
            for k = 1, 3 do
                s.order[k] = order[k]
            end
            me.timer = GEL_TICKS
            me.phase, me.phase_init = 4, 0
        elseif phase == 4 then
            -- EXE5's 0x080E2E24: a gel every 13 ticks on the next row's front panel (none: a row passed).
            local t = me.timer - 1
            me.timer = bit32.band(t, 0xFFFF)
            if t >= 0 then
                return
            end
            s.left -= 1
            if s.left < 0 then
                me.phase, me.phase_init = 8, 0
                return
            end
            local y = s.order[s.left + 1]
            local x = look.front(me, y)
            if x ~= 0 then
                -- (It runs while the battle is dimmed.)
                local g = look.gel(me, x, y, dimming_chips.damage_plus(me, s.bonus), s.left + 1)
                if g then
                    g.run_while_dimmed = true
                end
            end
            me.timer = GEL_TICKS
        elseif phase == 8 then
            -- EXE5's 0x080E2E8C: once every gel is gone.
            for k = 1, 4 do
                if s.out[k] then
                    return
                end
            end
            dimming_chips.done(me)
        else
            error(string.format("MetaGel's controller: phase %#x reads past its table", phase))
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { bonus = "u16", left = "i32", order = "u8[3]", out = "bool[4]" },
        update = dimming_chips.phases { name = "MetaGel", effect = effect },
    }

    -- EXE5's 0x080E2E9C, EXE4's 0x080E4728.
    local function hook(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
