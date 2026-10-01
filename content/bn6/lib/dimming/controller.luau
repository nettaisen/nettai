--!strict
-- What every dimming chip's controller shares (docs/engine/chips.md §3.6,
-- docs/engine/dimming-chip-effects.md §0): the update that runs the
-- dimming service's steps in the original's order around the chip's
-- effect, and the spawn every `off_802CCB4` entry does.
--
-- A controller is an effect object whose update is
-- `controller.phases { ... }`: `object_timefreezeBegin` (its init), then
-- by action 0 `object_dimScreen`, 4 the telop (`object_drawChipName`, the
-- wait for a counter cut-in), 8 the chip's effect, 0xC
-- `object_undimScreen`, and `object_timefreezeEnd` (its destroy state).
-- The effect ends with `controller.end_effect`, which undims from the next
-- tick.

local controller = {}

export type Phases = {
    -- What the errors call it.
    name: string,
    -- The effect (action 8).
    effect: (me: Object) -> (),
    -- `sub_800BBA8` rather than `object_drawChipName`: a hidden chip's
    -- telop.
    hidden_telop: boolean?,
}

-- The controller's update.
function controller.phases(p: Phases): (me: Object) -> ()
    local effect, name, hidden = p.effect, p.name, p.hidden_telop == true
    return function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            dimming.begin(me)
        elseif l == "update" then
            local a = me.action
            if a == 0 then
                dimming.dim_screen(me)
            elseif a == 4 then
                if hidden then
                    dimming.show_hidden_telop(me)
                else
                    dimming.show_telop(me)
                end
            elseif a == 8 then
                effect(me)
            elseif a == 0xC then
                dimming.undim_screen(me)
            else
                error(string.format("%s's action %#x reads past its table", name, a))
            end
        else
            dimming.finish(me)
        end
    end
end

-- The effect is over: action 0xC from its first phase (a halfword store
-- of 0 to the phase and its init flag), which undims from the next tick.
function controller.end_effect(me: Object)
    me:set_action(0xC)
end

-- `off_802CCB4[subtype]`'s common part: a controller of `kind` for `user`
-- on its panel, with the attack's element and damage word, the user as
-- its related object, and the user's side and facing (`side_only`: only
-- the side, as BurnSqr's spawner copies). Its position is the spawner's
-- registers, which nothing reads. The chip and its bonus (+0x30) are the
-- kind's state, where its effect reads them. nil if the effect pool is
-- full (nothing registers).
function controller.spawn(kind: Kind, user: Object, spec: DimmingChipSpec, side_only: boolean?): Object?
    local o = battle.spawn(kind)
    if not o then
        return nil
    end
    o.panel_x, o.panel_y = user.panel_x, user.panel_y
    o.element = spec.element
    o.related1 = user
    o.alliance = user.alliance
    if not side_only then
        o.flip = user.flip
    end
    o:set_damage_word(spec.damage)
    return o
end

-- The damage word (damage | counter byte << 16) plus `n`, added to the
-- whole word as the effects' spawns do with the bonus.
function controller.damage_plus(me: Object, n: number): number
    return int.u32(bit32.bor(me.damage, bit32.lshift(me.stamina, 16)) + n)
end

return controller
