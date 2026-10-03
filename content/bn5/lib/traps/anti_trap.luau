--!strict
-- BN5's Anti traps (AntiFire, AntiWatr, AntiElec, AntiWood: action 0x15, subtype 12): the trap chips' controller
-- (lib/traps) sets the shared ElemTrap trap (@common/elemtrap, attack object 0x4D, 0x080CF590) as BN5's code does
-- (0x080E3778,
-- which BN6 stubs), with the chip's first parameter. BN5's trap watches only that parameter's element (fire,
-- aqua, elec, wood: 0x080CF644, BN6's `sub_80CE034` the first of the four), is BN5's nothing reaching BN5's row
-- 0x18, and sparkles 16 pixels up from each enemy panel's center (0x080093A2). Sprung, its counterattack (effect
-- object 0x2B, 0x080E3818) strikes as BN6's ElemTrap's with BN5's table (0x080E38D8): BN5's sparks, row 0x17 for
-- its hits (BN6's row 4), hit modifier 3 for every element (BN6's 1 for elec and wood) and BN5's statuses.

local collision = require("../../rules/collision")
local common_strike = require("@common/elemtrap/strike")
local common_trap = require("@common/elemtrap/trap")
local effects = require("@bn6/lib/effects")
local panel_bursts = require("../../objects/panel-bursts/panel_bursts")
local sparks = require("../sparks")
local statuses = require("../../rules/status")
local traps = require("./traps")

local trap = {}

-- The bursts' looks (the panel bursts' rows 0, 2 and 4; fire and the null element row 3's fiery burst): BN5's
-- sheets, whose animations last as BN5's do (its hit sparks a tick longer than BN6's). (Elec's, 14-14, is BN6's
-- lightning's sheet, which BN5's pack doesn't name yet.)
local SPLASH = define.effect { sprite = asset.sprite("bn5:spout-splash"), anim = 0, palette = 0 }
local LIGHTNING = define.effect { sprite = asset.sprite("bn6:lightning"), anim = 0, palette = 0 }
local LEAVES = define.effect { sprite = asset.sprite("bn5:hit-sparks"), anim = 5, palette = 0 }

trap.strike_kind = common_strike.make_kind("bn5:anti-trap/strike", {
    strikes = {
        [0] = { sound = nil, spark = sparks.breaking, hit_mod = 3, status = nil, burst = effects.shell_burst },
        [1] = {
            sound = asset.sound("bn6:target-move"),
            spark = sparks.fire,
            hit_mod = 3,
            status = nil,
            burst = effects.shell_burst,
        },
        [2] = { sound = asset.sound("bn6:spout-ball"), spark = sparks.aqua, hit_mod = 3, status = nil, burst = SPLASH },
        [3] = {
            sound = asset.sound("bn6:beast-over-burst"),
            spark = sparks.elec,
            hit_mod = 3,
            status = statuses.paralyze_90,
            burst = LIGHTNING,
        },
        [4] = {
            sound = asset.sound("bn6:junk-shot"),
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

-- (Its sparkle and the sparkle's sound BN6's.)
trap.kind = common_trap.make_kind("bn5:anti-trap/trap", {
    sparkle = define.effect { sprite = asset.sprite("bn6:hit-sparks"), anim = 7, palette = 0 },
    sparkle_sound = asset.sound("bn6:dimming-sparkle"),
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
