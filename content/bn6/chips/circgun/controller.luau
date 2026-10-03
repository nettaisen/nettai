--!strict
-- CircGun's dimming controller (the original's effect object #0x5E,
-- `sub_80E7568`; docs/engine/dimming-chip-effects.md §8): the usual dimming
-- phases (lib/dimming), with an effect that puts the gun (./gun) on the
-- far column's top panel, with the damage plus the bonus, and lasts until
-- it is done; with none of the other side's panels in that column, it ends
-- at once.

local dimming_chips = require("../../lib/dimming")
local gun = require("./gun")

export type State = {
    variant: CircGunVariant,
    -- The Atk+ / cross bonus the telop shows (+0x32), which the shots'
    -- damage adds.
    bonus: number,
    -- The gun is out (the original's Unk_0c).
    busy: boolean,
}

-- `dword_80E75FC`: the start panel, by the way it faces: the far column's
-- top.
local START: { [number]: { number } } = { [0] = { 6, 1 }, [1] = { 1, 1 } }

-- `sub_800D5F0`: column `x` has a panel that isn't `side`'s.
local function foreign(x: number, side: number): boolean
    for y = 1, 3 do
        local p = field.panel(x, y)
        if p and p.alliance ~= side then
            return true
        end
    end
    return false
end

-- `sub_80E75AC`: the gun on the first tick; the effect ends once it is
-- done (at once if it couldn't be spawned).
local function effect(me: Object)
    local s = me.state :: State
    if me.phase_init == 0 then
        local start = START[bit32.bxor(me.alliance, me.flip)]
        if not foreign(start[1], me.alliance) then
            dimming_chips.done(me)
            return
        end
        gun.spawn(me, s.variant, start[1], start[2], me.element, dimming_chips.damage_plus(me, s.bonus))
        me.phase_init = 4
    end
    if not s.busy then
        dimming_chips.done(me)
    end
end

local kind = define.kind {
    id = "bn6:circgun/controller",
    pool = "effect",
    state = { variant = "record:circgun-variant", bonus = "u16", busy = "bool" },
    -- `sub_80E7568`.
    update = dimming_chips.phases { name = "CircGun", effect = effect },
}

-- A chip's `dimming` hook (`sub_80E7600`): the controller with a gun of
-- `variant`.
local function hook(variant: CircGunVariant): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = dimming_chips.spawn(kind, user, spec)
        if o then
            local s = o.state :: State
            s.variant, s.bonus = variant, spec.bonus
        end
        return o
    end
end

-- A gun's variant.
local function variant(v: CircGunVariant): CircGunVariant
    return define.record("circgun-variant", v)
end

return { hook = hook, variant = variant }
