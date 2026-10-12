--!strict
-- Magnum's dimming controller (effect object #0x30: EXE5's 0x080E3D84, the dimming handler's subtype 15, 0x080E3E04;
-- EXE4's 0x080E57F8, its variant 0x16, 0x080E5878: the same code), made for a game's gunner (`controller.make`): the
-- usual dimming phases (@exelib/dimming), with an effect (EXE5's 0x080E3DC8, EXE4's 0x080E583C) that sends out the
-- gunner (@exelib/magnum/gunner) with the damage plus the bonus (and Magnum's mode 0, Param1), and lasts until the
-- gunner has fired and gone (its stage, Param2, at 2).

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The Atk+ bonus the telop shows (+0x32), which the shot's damage adds.
    bonus: number,
    -- The gunner's stage (Param2: 1 fired, 2 gone).
    stage: number,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The game's gunner's spawn.
    gunner: (controller: Object, user: Object, damage: number) -> Object?,
}

export type Controller = {
    kind: Kind,
    -- Magnum's `dimming` hook.
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

function controller.make(look: Look): Controller
    local function effect(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            look.gunner(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
            me.timer = 60
            me.phase_init = 4
        end
        if s.stage >= 2 then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { bonus = "u16", stage = "u8" },
        update = dimming_chips.phases { name = "Magnum", effect = effect },
    }

    local function hook(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            local s = o.state :: State
            s.bonus = spec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
