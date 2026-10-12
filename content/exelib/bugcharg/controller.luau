--!strict
-- BugCharg's dimming controller (effect object #0x7C: EXE4's 0x080EAB58, EXE5's 0x080E98C8, the same code; EXE6's
-- SumnBlk controller's skeleton), made for a game's stand-in (`controller.make`): the usual dimming phases
-- (@exelib/dimming), with an effect (EXE4's 0x080EAB9C, EXE5's 0x080E990C) that sends out BugCharg's stand-in
-- (@exelib/bugcharg/actor) with the damage plus the bonus on its first tick and ends once it is done (at once if it
-- couldn't be spawned).

local dimming_chips = require("@exelib/dimming")

local controller = {}

export type State = {
    -- The Atk+ / cross bonus (+0x32), which the shots' damage adds.
    bonus: number,
    -- The stand-in is out (+0x0C).
    busy: boolean,
}

export type Look = {
    -- The kind's id.
    id: string,
    -- The game's stand-in's spawn.
    actor: (controller: Object, user: Object, damage: number) -> Object?,
}

export type Controller = {
    kind: Kind,
    -- BugCharg's `dimming` hook (EXE4's 0x080EABD6, EXE5's 0x080E9946: only the alliance byte of its user's).
    hook: (user: Object, spec: DimmingChipSpec) -> Object?,
}

function controller.make(look: Look): Controller
    local function effect(me: Object)
        local s = me.state :: State
        if me.phase_init == 0 then
            look.actor(me, me.related1 :: Object, dimming_chips.damage_plus(me, s.bonus))
            me.phase_init = 4
        end
        if not s.busy then
            dimming_chips.done(me)
        end
    end

    local kind = new.kind {
        id = look.id,
        pool = "effect",
        state = { bonus = "u16", busy = "bool" },
        update = dimming_chips.phases { name = "BugCharg", effect = effect },
    }

    local function hook(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec, true)
        if o then
            (o.state :: State).bonus = spec.bonus
        end
        return o
    end

    return { kind = kind, hook = hook }
end

return controller
