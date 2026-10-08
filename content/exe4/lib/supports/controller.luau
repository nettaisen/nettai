--!strict
-- EXE4's NaviCust supports' dimming controller (effect object #0x74, 0x080EA2BC: EXE5's number, EXE6's #0x79 by its
-- phases): the shared controller (@exelib/supports/controller) with EXE4's supports, Rush (./rush), Beat (./beat) and
-- Tango (./tango), and EXE4's warp: the host's side's navi warps out and back in (0x080E2D26, by the controller's
-- side). The rules spawn it by the role `kinds.support` (rules/roles.luau): a support's trigger (0x0800C7D8, 0x0800C838,
-- 0x0800C8C0) through 0x080EA3D6, then a dimming no one can cut in on (0x080090B8). See docs/engine/chips.md §2.10.
--
-- The chips its telop names (0x17F to 0x181: chips/supports) are dimming chips of their own, the dimming handler's
-- variant 0x42 (0x080220E0's), whose use spawns it through the same 0x080EA3D6 (`supports.dimming`). No folder holds
-- them.

local beat = require("./beat")
local common = require("@exelib/supports/controller")
local navi_chips = require("../navi_chips")
local rush = require("./rush")
local tango = require("./tango")

local supports = {}

export type Support = "rush" | "beat" | "tango"

supports.kind = common.make {
    id = "support/controller",
    rush = rush,
    beat = beat,
    tango = tango,
    warp = function(c: Object, out: boolean)
        navi_chips.warp(c.alliance, out)
    end,
}

-- 0x080EA3D6 as the dimming handler (0x080EB9EA) calls it for a support's chip: `support`'s controller on the user's
-- panel, the user its host, of the handler's element and damage word, the chip its telop. (Its position is the
-- handler's registers.)
function supports.dimming(support: Support): (user: Object, spec: DimmingChipSpec) -> Object?
    return function(user: Object, spec: DimmingChipSpec): Object?
        local o = battle.spawn(supports.kind, Vec3.zero)
        if not o then
            return nil
        end
        o.panel_x, o.panel_y = user.panel_x, user.panel_y
        o.element = spec.element
        o.related1 = user
        o.alliance = user.alliance
        o:set_damage_word(spec.damage)
        local s = o.state :: { support: Support, telop_chip: Chip? }
        s.support = support
        s.telop_chip = spec.chip
        return o
    end
end

return supports
