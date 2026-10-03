--!strict
-- HolyDrem's dimming controller (BN5's effect object #0x59, 0x080E6DFC: the same as BN6's BurnSqr controller,
-- `sub_80E702C`, but for what its effect sends out and where it keeps its flag): the usual dimming phases
-- (@bn6/lib/dimming), with an effect that sends out HolyDrem's stand-in (./actor) with the damage plus the bonus
-- and lasts until it is done.

local actor = require("./actor")
local dimming_chips = require("@bn6/lib/dimming")

export type State = {
    -- The Atk+ / cross bonus the telop shows (+0x32), which the shots' damage adds.
    bonus: number,
    -- The stand-in is out (+0x0C).
    busy: boolean,
}

-- 0x080E6E40: the stand-in on the first tick; the effect ends once it is done (at once if it couldn't be
-- spawned). (BN5's spawner, 0x080BD93E, is passed the controller's row but not its column: the stand-in starts on
-- column 0 until its init places it on the user.)
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local user = me.related1 :: Object
        local o = battle.spawn(actor.kind)
        if o then
            o.panel_x, o.panel_y = 0, me.panel_y
            o.element = me.element
            o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
            o.related1 = user
            -- (`actor.State`: a type of another module, which the checker
            -- reads as any.)
            ;(o.state :: any).controller = me
            s.busy = true
            o.alliance, o.flip = user.alliance, user.flip
            o.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn5:holydrem/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- 0x080E6DFC.
    update = dimming_chips.phases { name = "HolyDrem", effect = effect },
}

-- The chip's `dimming` hook (0x080E6E78, the dimming spawner's entry 0x23): the controller, of the user's side
-- and facing.
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
