--!strict
-- EXE4's Anti traps (AntiFire, AntiWatr, AntiElec, AntiWood: the dimming chips' action 0x0C, its variant 0x13, their
-- parameter 0 to 3): the trap chips' controller (lib/traps) sets the shared ElemTrap trap (@exelib/elemtrap, attack
-- object 0x4D, 0x080D7100: EXE5's code, as EXE5's Anti chips set it, 0x080E51A4), watching the chip's element (its
-- parameter plus one), EXE4's nothing reaching EXE4's row 0x18, and sparkling 16 pixels up from each enemy panel's
-- center (0x0800815C: effect row 0x48, sound 0x10D). Sprung, its counterattack (effect object 0x2B, 0x080E51BC) strikes
-- with EXE5's table (0x080E52E8: the same rows) and EXE4's sounds (0x080E5310), but where EXE4's says (0x080E5244):
-- only the other side's navi's panels; then the other side's mood falls to 1 (0x0800F4DE: a mood of 0 stays) and the
-- bursts (objects/panel_bursts, the rows 0x080E531A names by element) go over the other side's area alone
-- (0x080E531F).

local collision = require("../../rules/collision")
local common_strike = require("@exelib/elemtrap/strike")
local common_trap = require("@exelib/elemtrap/trap")
local panel_bursts = require("../../objects/panel_bursts")
local panels = require("@exelib/panels")
local regions = require("@exelib/regions")
local sparks = require("../sparks")
local statuses = require("../../rules/status")
local traps = require("../traps")

local trap = {}

-- The bursts' looks (the panel bursts' rows 3, 0, 2 and 4, 0x080E4A00: fire's, and the null element's, a heat burst;
-- aqua's a bubble's; elec's lightning; wood's leaves).
local HEAT = new.effect { sprite = asset.sprite("heat-burst"), anim = 0, palette = 0 }
local BUBBLE = new.effect { sprite = asset.sprite("bubble-burst"), anim = 0, palette = 0 }
local LIGHTNING = new.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0 }
local LEAVES = new.effect { sprite = asset.sprite("hit-sparks"), anim = 5, palette = 0 }

-- 0x080E5244: the other side's navi's panels (0x080E52D8: the panels of the other side's player navi, but the
-- counterattack's own), last to first.
local function targets(me: Object): { { x: number, y: number } }
    local marked = regions.other_navi[me.alliance].field :: PanelCondition
    local list: { { x: number, y: number } } = {}
    for _, p in panels.filtered_except(me, marked.require, marked.forbid) do
        local x: number, y: number = panels.x(p), panels.y(p)
        table.insert(list, { x = x, y = y })
    end
    return list
end

-- 0x080E529E: the other side's mood falls to 1 (`battle.set_mood`: EXE4's mood is held at 0; under event flag 0x1187,
-- which no netbattle sets, it wouldn't change).
local function after(me: Object)
    battle.set_mood(bit32.bxor(me.alliance, 1), 1)
end

trap.strike_kind = common_strike.make_kind("anti-trap/strike", {
    strikes = {
        [0] = { sound = nil, spark = sparks.breaking, hit_mod = 3, status = nil, burst = HEAT },
        [1] = { sound = asset.sound("heat-burst"), spark = sparks.fire, hit_mod = 3, status = nil, burst = HEAT },
        [2] = { sound = asset.sound("bubble-burst"), spark = sparks.aqua, hit_mod = 3, status = nil, burst = BUBBLE },
        [3] = {
            sound = asset.sound("lightning"),
            spark = sparks.elec,
            hit_mod = 3,
            status = statuses.paralyze_90,
            burst = LIGHTNING,
        },
        [4] = {
            sound = asset.sound("wood-strike"),
            spark = sparks.wood,
            hit_mod = 3,
            status = statuses.confuse_480,
            burst = LEAVES,
        },
    },
    self_type = collision.thrown_piercing_break,
    target = collision.hits_navis,
    bursts = panel_bursts.kind,
    targets = targets,
    after = after,
    bursts_over = function(me: Object): Region
        return regions.area[bit32.bxor(me.alliance, 1)]
    end,
})

trap.kind = common_trap.make_kind("anti-trap/trap", {
    sparkle = new.effect { sprite = asset.sprite("hit-sparks"), anim = 7, palette = 0 },
    sparkle_sound = asset.sound("dimming-sparkle"),
    watches = "one",
    self_type = collision.nothing,
    target = collision.hit_by_other_attacks,
    sparkle_dy = 0,
    sparkle_z = 0x10_0000,
    strike_kind = trap.strike_kind,
})

-- The `dimming` hook of the Anti chip of `element` (1 fire, 2 aqua, 3 elec, 4 wood: its first parameter plus one).
function trap.hook(element: number)
    return traps.hook(traps.trap {
        set = function(controller: Object, user: Object, chip: Chip?, bonus: number): Object?
            return common_trap.spawn(trap.kind, controller, user, chip, bonus, element)
        end,
    })
end

return trap
