--!strict
-- EXE5's Anti traps (AntiFire, AntiWatr, AntiElec, AntiWood: action 0x15, subtype 12): the trap chips' controller
-- (lib/traps) sets the shared ElemTrap trap (@exelib/elemtrap, attack object 0x4D, 0x080CF590) as EXE5's code does
-- (0x080E3778,
-- which EXE6 stubs), with the chip's first parameter. EXE5's trap watches only that parameter's element (fire,
-- aqua, elec, wood: 0x080CF644, EXE6's `sub_80CE034` the first of the four), is EXE5's nothing reaching EXE5's row
-- 0x18, and sparkles 16 pixels up from each enemy panel's center (0x080093A2). Sprung, its counterattack (effect
-- object 0x2B, 0x080E3818) strikes as EXE6's ElemTrap's with EXE5's table (0x080E38D8): EXE5's sparks, row 0x17 for
-- its hits (EXE6's row 4), hit modifier 3 for every element (EXE6's 1 for elec and wood) and EXE5's statuses.

local collision = require("../../rules/collision")
local common_strike = require("@exelib/elemtrap/strike")
local common_trap = require("@exelib/elemtrap/trap")
local effects = require("../effects")
local panel_bursts = require("../../objects/panel-bursts")
local sparks = require("../sparks")
local statuses = require("../../rules/status")
local traps = require("../traps")

local trap = {}

-- The bursts' looks (the panel bursts' rows 0, 2 and 4; fire and the null element row 3's fiery burst): EXE5's
-- sheets, whose animations last as EXE5's do (its hit sparks a tick longer than EXE6's). (Elec's is sheet 14-14,
-- `lightning`.)
local SPLASH = define.effect { sprite = asset.sprite("spout-splash"), anim = 0, palette = 0 }
local LIGHTNING = define.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0 }
local LEAVES = define.effect { sprite = asset.sprite("hit-sparks"), anim = 5, palette = 0 }

trap.strike_kind = common_strike.make_kind("anti-trap/strike", {
    strikes = {
        [0] = { sound = nil, spark = sparks.breaking, hit_mod = 3, status = nil, burst = effects.shell_burst },
        [1] = {
            sound = asset.sound("target-move"),
            spark = sparks.fire,
            hit_mod = 3,
            status = nil,
            burst = effects.shell_burst,
        },
        [2] = { sound = asset.sound("spout-ball"), spark = sparks.aqua, hit_mod = 3, status = nil, burst = SPLASH },
        [3] = {
            sound = asset.sound("beast-over-burst"),
            spark = sparks.elec,
            hit_mod = 3,
            status = statuses.paralyze_90,
            burst = LIGHTNING,
        },
        [4] = {
            sound = asset.sound("junk-shot"),
            spark = sparks.wood,
            hit_mod = 3,
            status = statuses.confuse_480,
            burst = LEAVES,
        },
    },
    self_type = collision.thrown_piercing_break,
    target = collision.hits_navis,
    bursts = panel_bursts.kind,
})

-- (Its sparkle and the sparkle's sound EXE6's numbers.)
trap.kind = common_trap.make_kind("anti-trap/trap", {
    sparkle = define.effect { sprite = asset.sprite("hit-sparks"), anim = 7, palette = 0 },
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
