--!strict
-- HolyDrem's dimming controller, as EXE5 and EXE4 share it (EXE5's effect object #0x59, 0x080E6DFC; EXE4's #0x59,
-- 0x080E8418, the same code; EXE6's BurnSqr controller's, `sub_80E702C`, but for what its effect sends out and where it
-- keeps its flag): the usual dimming phases (@exelib/dimming), with an effect that sends out HolyDrem's stand-in with
-- the damage plus the bonus and lasts until it is done. Each game makes its own (`controller.make`) with its stand-in
-- and what of its user the controller's spawn takes; a HolyDrem chip's `dimming` hook is the made controller's `hook`.

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the shots' damage adds.
    bonus: number,
    -- The stand-in is out (+0x0C).
    busy: boolean,
}

export type Spec = {
    -- The kind's id.
    id: string,
    -- The game's stand-in for `user`, sent out by `controller` with the damage word (EXE5's 0x080BD93E, EXE4's
    -- 0x080DD97E): nil when it couldn't come. It tells the controller when it is done (the state's `busy`).
    stand_in: (controller: Object, user: Object, damage: number) -> Object?,
    -- The hook's spawn copies the user's side alone, not its facing (`dimming_chips.spawn`'s `side_only`: EXE4's
    -- 0x080E8510); none: the side and the facing (EXE5's 0x080E6E78).
    side_only: boolean?,
}

export type Controller = { kind: Kind, hook: (user: Object, chip: DimmingChipSpec) -> Object? }

function controller.make(spec: Spec): Controller
    -- EXE5's 0x080E6E40, EXE4's 0x080E84D8: the stand-in on the first tick; the effect ends once it is done (at once
    -- if it couldn't be spawned: its spawner raises the flag only for a stand-in it made).
    local function effect(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            if spec.stand_in(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus)) then
                s.busy = true
            end
            me.phase_init = 4
        end
        if not s.busy then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = spec.id,
        pool = "effect",
        state = { bonus = "u16", busy = "bool" },
        -- EXE5's 0x080E6DFC, EXE4's 0x080E8418.
        update = dimming_chips.phases { name = "HolyDrem", effect = effect },
    }

    -- The chip's `dimming` hook (EXE5's 0x080E6E78, the dimming spawner's entry 0x23; EXE4's 0x080E8510, its variant
    -- 0x2C): the controller.
    local function hook(user: Object, chip: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, chip, spec.side_only)
        if o then
            (o.state :: State).bonus = chip.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
