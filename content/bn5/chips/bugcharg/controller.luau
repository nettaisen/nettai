--!strict
-- BugCharg's dimming controller (BN5's effect object #0x7C, 0x080E98C8; BN6's SumnBlk controller's skeleton): the usual
-- dimming phases (@exelib/dimming), with an effect (0x080E990C) that sends out BugCharg's stand-in (./actor) with the
-- damage plus the bonus and lasts until it is done.

local actor = require("./actor")
local dimming_chips = require("@exelib/dimming")

export type State = {
    -- The Atk+ / cross bonus (+0x32), which the shots' damage adds.
    bonus: number,
    -- The stand-in is out (+0x0C).
    busy: boolean,
}

-- 0x080E990C: the stand-in on the first tick (0x080C3C3A: on the controller's panel, its element the controller's
-- side byte, the user's side, facing and NameID); the effect ends once it is done (at once if it couldn't be
-- spawned).
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local user = me.related1 :: Object
        local o = battle.spawn(actor.kind)
        if o then
            o.panel_x, o.panel_y = me.panel_x, me.panel_y
            o.element = me.alliance
            o:set_damage_word(dimming_chips.damage_plus(me, s.bonus))
            o.related1 = user
            -- (`actor.State`: a type of another module, which the checker reads as any.)
            ;(o.state :: any).controller = me
            s.busy = true
            o.alliance, o.flip = user.alliance, user.flip
            -- (The user's NameID, which its charger's attach point reads.)
            o.identity = user.identity
            o.run_while_dimmed = true
        end
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn5:bugcharg/controller",
    pool = "effect",
    state = { bonus = "u16", busy = "bool" },
    -- 0x080E98C8.
    update = dimming_chips.phases { name = "BugCharg", effect = effect },
}

-- 0x080E9946: BugCharg's `dimming` hook (only the alliance byte of its user's).
local function hook(user: Object, spec: DimmingChipSpec): Object?
    local o = dimming_chips.spawn(kind, user, spec, true)
    if o then
        (o.state :: State).bonus = spec.bonus
    end
    return o
end

return { hook = hook }
