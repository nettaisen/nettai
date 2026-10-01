--!strict
-- What every dimming chip's controller shares (docs/design/content-model-v2.md
-- §5.4): the update that runs the dimming service's steps in the original's
-- order around the chip's effect, and the spawn every `off_802CCB4` entry
-- does. A chip's `dimming` hook spawns its controller
-- (`dimming_chips.spawn`) and sets the controller's own state.

local dimming_chips = {}

-- The controller's actions: dim the screen, the telop (which moves on to
-- the effect, or past it), the chip's effect, undim.
local DIM, TELOP, EFFECT, UNDIM = 0x0, 0x4, 0x8, 0xC

export type Phases = {
    -- For the error past the action table.
    name: string,
    -- The chip's effect, run each tick of its action until it calls
    -- `dimming_chips.done`.
    effect: (me: Object) -> (),
}

-- A controller's update: `object_timefreezeBegin` at its init; by its
-- action `object_dimScreen`, `object_drawChipName` (the telop and the wait
-- for a cut-in), the effect and `object_undimScreen`; and
-- `object_timefreezeEnd` when it is destroyed.
function dimming_chips.phases(spec: Phases): (me: Object) -> ()
    return function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            dimming.begin(me)
        elseif l == "update" then
            local a = me.action
            if a == DIM then
                dimming.dim_screen(me)
            elseif a == TELOP then
                dimming.show_telop(me)
            elseif a == EFFECT then
                spec.effect(me)
            elseif a == UNDIM then
                dimming.undim_screen(me)
            else
                error(string.format("%s's controller: action %#x reads past its table", spec.name, a))
            end
        else
            dimming.finish(me)
        end
    end
end

-- The effect is over: undim next.
function dimming_chips.done(me: Object)
    me:set_action(UNDIM)
end

-- The spawn every `off_802CCB4` entry does: the controller of `kind` for
-- `user`, on the user's panel and side, with the chip's element and damage
-- word. (Its position is the spawner's registers, which nothing reads; the
-- telop draws the chip the dimming registered, so the controller keeps no
-- chip of its own.)
function dimming_chips.spawn(kind: Kind, user: Object, spec: DimmingChipSpec): Object?
    local o = battle.spawn(kind, Vec3.zero)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance, o.flip = user.alliance, user.flip
    o:set_damage_word(spec.damage)
    return o
end

return dimming_chips
