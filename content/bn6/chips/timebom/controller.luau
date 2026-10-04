--!strict
-- The TimeBom chips' dimming controller (the original's effect object
-- #0x27, `sub_80E31D8`): the shared controller (@exelib/timebom/controller)
-- placing BN6's bomb (./countdown) on the first free enemy panel in front of
-- the user. A chip's `dimming` hook is `controller.hook(variant)`.

local common = require("@exelib/timebom/controller")
local countdown = require("./countdown")
local dimming_chips = require("@exelib/dimming")

export type State = { bonus: number, variant: CountdownVariant }

-- `off_80E3280`: where the bomb goes, by side: a solid enemy panel without
-- a body on it (side 0: flags 0x30, not 0x0F800000; side 1: 0x10, not
-- 0x0F800020).
local TARGET: { [number]: PanelCondition } = {
    [0] = { require = 0x30, forbid = 0x0F80_0000 },
    [1] = { require = 0x10, forbid = 0x0F80_0020 },
}

-- `sub_80E3264` (`object_getFirstPanelInDirectionFiltered`): from the
-- controller's panel forward, the first panel meeting the side's
-- condition, while the panels are on the field (nil: none).
local function target_column(me: Object): number?
    local cond = TARGET[me.alliance]
    local x, y = me.panel_x, me.panel_y
    while true do
        if field.check(x, y, cond.require, cond.forbid) then
            return x
        end
        x += me.facing
        if not field.valid(x, y) then
            return nil
        end
    end
end

-- `sub_80E3290`: the bomb, with the damage plus the bonus.
local function place(me: Object)
    local x = target_column(me)
    -- (The game tests the column number for zero: column 0 is off the
    -- field, so a found panel always places.)
    if x then
        local s = me.state :: State
        countdown.spawn(s.variant, x, me.panel_y, me.alliance, dimming_chips.damage_plus(me, s.bonus), s.bonus)
    end
end

local bn6 = common.make("timebom/controller", place)

return { kind = bn6.kind, hook = bn6.hook, make = common.make }
