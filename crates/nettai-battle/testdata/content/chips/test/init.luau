--!strict
-- The engine's test chips: made-up records (damage, flags, codes) whose
-- uses are the EXE6 overlay's own builders and actions, so the engine's
-- tests run the real scripts on data they can reason about. Each is
-- `test/<name>`, its key in the table this returns (crates/nettai-battle/src/content/testing.rs names
-- them).

local antinavi = require("./antinavi").antinavi
local assnswrd = require("./assnswrd").assnswrd
local bass = require("./bass/navi")
local blastman = require("./blastman/navi")
local bugbomb = require("./bugbomb").bugbomb
local chrgeman = require("./chrgeman/navi")
local elecman = require("./elecman/navi")
local elemtrap = require("./elemtrap").elemtrap
local elmntman = require("./elmntman/navi")
local eraseman = require("./eraseman/navi")
local fist = require("./firehit/fist")
local flshbom_chips = require("./flshbom")
local gundels_chips = require("./gundels")
local heatman = require("./heatman/navi")
local hook = require("./flmhook/hook")
local invisible = require("./invisibl/controller")
local lockon = require("../rules/lockon")
local minibomb = require("./minibomb").minibomb
local navi_chips = require("../lib/navi_chips/chip")
local plus = require("../lib/instant/plus")
local poisseed = require("./poisseed").poisseed
local slashman = require("./slashman/navi")
local sparks = require("../lib/sparks")
local spoutman = require("./spoutman/navi")
local stepswrd = require("./stepswrd").stepswrd
local sunmoon = require("./sunmoon/sun")
local tenguman = require("./tenguman/navi")
local timebom_chips = require("./timebom")
local tmhkman = require("./tmhkman/navi")
local traps = require("../lib/traps/controller")
local wideswrd = require("./wideswrd").wideswrd

local dustbrk = require("../navis/dustman/chip").dustbrk
local delecswd = require("../navis/elecman/chip").delecswd
local edeletbm = require("../navis/eraseman/chip").edeletbm
local etomahwk = require("../navis/tomahawkman/chip").etomahwk
local ftornado = require("../navis/tenguman/chip").ftornado
local heatpres = require("../navis/heatman/chip").heatpres
local rc_brakr = require("../navis/groundman/chip")["rc-brakr"]
local rslash = require("../navis/slashman/chip").rslash
local volcchrg = require("../navis/chargeman/chip").volcchrg
local dripshwr = require("../navis/spoutman/chip").dripshwr

-- A chip with the fields tests don't care about filled in (codes A and
-- `*`, 10 MB, three to a folder; `n` its place in the library), and
-- `fields` over them.
local function chip(key: string, n: number, fields: { [string]: any }): Chip
    local spec: { [string]: any } = {
        codes = { "A", "*" },
        element = "null",
        family = "null",
        class = "standard",
        rarity = 0,
        mb = 10,
        damage = 0,
        hit_param = 0,
        lockout = 0,
        flags = { "standard_library" },
        slot_in_limit = 3,
    }
    for k, v in fields do
        spec[k] = v
    end
    return spec :: any
end

-- The SunGuns: EXE6's GunDelSols' actions (chips/gundels), each a GunDelSol's.
local function sun_gun(key: string, n: number, gundel: Chip): Chip
    return chip(key, n, { beast = { lockon = lockon.cannon }, action = gundel.action })
end

-- A thrown chip.
local function thrown(key: string, n: number, damage: number, action: Action?): Chip
    return chip(key, n, {
        flags = { "has_damage", "standard_library" },
        hit_param = 30,
        damage = damage,
        action = action,
    })
end

-- A sword chip.
local function blade(key: string, n: number, action: Action?): Chip
    return chip(key, n, {
        flags = { "has_damage", "standard_library" },
        family = "sword",
        hit_param = 30,
        damage = 80,
        action = action,
    })
end

-- A navi chip, with its own controller (EXE6's lib/navi_chips/chip).
local function navi_chip(key: string, n: number, damage: number, summon: NaviChipSummon, fields: { [string]: any }?): Chip
    local all: { [string]: any } = {
        flags = { "has_damage", "navi", "library" },
        class = "mega",
        damage = damage,
        navi = navi_chips.make { id = "test/" .. key .. "/controller", summon = summon },
    }
    for k, v in fields or {} do
        all[k] = v
    end
    return chip(key, n, all)
end

-- An instant chip whose effect spawns an object.
local function spawning(key: string, n: number, instant: (user: Object, spec: InstantChipSpec) -> ()): Chip
    return chip(key, n, {
        flags = { "has_damage", "standard_library" },
        hit_param = 0x30,
        damage = 40,
        lockout = 20,
        instant = instant,
    })
end

-- A link navi's own chip (any navi can use it here): EXE6's action
-- (navis/<navi>/chip), with made-up damage.
local function link_chip(key: string, n: number, of: Chip): Chip
    return chip(key, n, {
        flags = { "has_damage", "library" },
        class = "special",
        hit_param = 0x30,
        damage = 60,
        action = of.action,
    })
end

local chips: { [string]: Chip } = {
    -- What a zeroed chip field reads (the roles' `chips.zeroed`): a chip
    -- no test uses.
    ["test/blank"] = chip("blank", 0, {
        class = "special",
        codes = {},
        instant = function(_user: Object, _spec: InstantChipSpec)
            error("the test content's blank chip has no use")
        end,
    }),
    ["test/sun-gun-1"] = sun_gun("sun-gun-1", 0x01, gundels_chips.gundels1),
    ["test/sun-gun-2"] = sun_gun("sun-gun-2", 0x02, gundels_chips.gundels2),
    ["test/sun-gun-3"] = sun_gun("sun-gun-3", 0x03, gundels_chips.gundels3),
    ["test/sun-gun-ex"] = sun_gun("sun-gun-ex", 0x04, gundels_chips.gundelex),
    -- Invisibl's six seconds.
    ["test/veil"] = chip("veil", 0x05, {
        -- (The counter part's made-up extension.)
        test_weight = 3,
        test_tag = { level = 2, kind = "b" },
        flags = { "dimming", "standard_library" },
        extra_flags = { "rush_cancels" },
        family = "plus",
        dimming = invisible.hook(360),
    }),
    ["test/eraser"] = navi_chip("eraser", 0x06, 60, eraseman.summon { aim_ticks = 16 }, { family = "cursor", hit_param = 100 }),
    -- A trap chip that sets no object.
    ["test/trap"] = chip("trap", 0x09, { flags = { "dimming", "standard_library" }, dimming = traps.hook() }),
    ["test/elem-trap"] = chip("elem-trap", 0x30, {
        flags = { "dimming", "has_damage", "standard_library" },
        damage = 40,
        dimming = elemtrap.dimming,
    }),
    ["test/time-bomb"] = chip("time-bomb", 0x31, {
        flags = { "dimming", "has_damage", "standard_library" },
        hit_param = 100,
        damage = 50,
        dimming = timebom_chips.timebom1.dimming,
    }),
    ["test/time-bomb-plus"] = chip("time-bomb-plus", 0x32, {
        flags = { "dimming", "has_damage", "standard_library" },
        hit_param = 100,
        damage = 70,
        dimming = timebom_chips.timebom.dimming,
    }),
    -- A plus chip used on its own.
    ["test/plus"] = chip("plus", 0x41, { family = "plus", damage = 10, instant = plus.attack }),
    ["test/fist"] = spawning("fist", 0x46, fist.instant(3)),
    ["test/flame-hook"] = spawning("flame-hook", 0x48, hook.instant { big = false, spark = sparks.fire }),
    ["test/heatpres"] = link_chip("heatpres", 0x50, heatpres),
    ["test/delecswd"] = link_chip("delecswd", 0x51, delecswd),
    ["test/rslash"] = link_chip("rslash", 0x52, rslash),
    ["test/edeletbm"] = link_chip("edeletbm", 0x53, edeletbm),
    ["test/volcchrg"] = link_chip("volcchrg", 0x54, volcchrg),
    ["test/dripshwr"] = link_chip("dripshwr", 0x55, dripshwr),
    ["test/etomahwk"] = link_chip("etomahwk", 0x56, etomahwk),
    ["test/ftornado"] = link_chip("ftornado", 0x57, ftornado),
    ["test/rc-brakr"] = link_chip("rc-brakr", 0x58, rc_brakr),
    ["test/dustbrk"] = link_chip("dustbrk", 0x59, dustbrk),
    -- A blank chip that is the AntiNavi trap when a side's defensive-chip
    -- record holds it.
    ["test/anti-navi"] = chip("anti-navi", 0xBA, {
        class = "special",
        codes = {},
        trap = "anti_navi",
        dimming = antinavi.dimming,
    }),
    ["test/bomb"] = thrown("bomb", 0x103, 50, minibomb.action),
    ["test/seed"] = thrown("seed", 0x104, 10, poisseed.action),
    ["test/flash"] = thrown("flash", 0x105, 40, flshbom_chips.flshbom2.action),
    ["test/bug"] = thrown("bug", 0x106, 0, bugbomb.action),
    -- A column of three panels ahead; the same after a step two panels
    -- ahead; a strike at stunned or grounded opponents.
    ["test/blade"] = blade("blade", 0x109, wideswrd.action),
    ["test/step-blade"] = blade("step-blade", 0x10A, stepswrd.action),
    ["test/stun-blade"] = blade("stun-blade", 0x10B, assnswrd.action),
    ["test/elements"] = navi_chip("elements", 0x110, 50, elmntman.summon { cycle_ticks = 4 }, { hit_param = 100 }),
    ["test/spout"] = navi_chip("spout", 0x111, 40, spoutman.summon, { element = "aqua" }),
    ["test/heat"] = navi_chip("heat", 0x112, 40, heatman.summon),
    ["test/elec"] = navi_chip("elec", 0x113, 40, elecman.summon),
    ["test/slash"] = navi_chip("slash", 0x114, 40, slashman.summon { wave_damage = 10 }),
    ["test/charge"] = navi_chip("charge", 0x115, 40, chrgeman.summon),
    ["test/tomahawk"] = navi_chip("tomahawk", 0x116, 40, tmhkman.summon),
    ["test/tengu"] = navi_chip("tengu", 0x117, 40, tenguman.summon),
    ["test/blast"] = navi_chip("blast", 0x118, 40, blastman.summon),
    ["test/shooter"] = navi_chip("shooter", 0x119, 30, bass.summon, { class = "giga" }),
    ["test/sun-moon"] = navi_chip("sun-moon", 0x11A, 90, sunmoon.summon, { class = "giga" }),
}

return chips
