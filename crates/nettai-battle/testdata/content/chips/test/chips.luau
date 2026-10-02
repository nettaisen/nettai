--!strict
-- The engine's test chips: made-up records (damage, flags, codes) whose
-- uses are the BN6 overlay's own builders and actions, so the engine's
-- tests run the real scripts on data they can reason about. Each is
-- `test/<name>` (crates/nettai-battle/src/content/testing.rs names them).

local antinavi = require("../antinavi/chip")
local assnswrd = require("../assnswrd/chip")
local bass = require("../bass/navi")
local blastman = require("../blastman/navi")
local bugbomb = require("../bugbomb/chip")
local chrgeman = require("../chrgeman/navi")
local elecman = require("../elecman/navi")
local elemtrap = require("../elemtrap/chip")
local elmntman = require("../elmntman/navi")
local eraseman = require("../eraseman/navi")
local fist = require("../firehit/fist")
local flshbom = require("../flshbom/chips")
local gundels = require("../gundels/chips")
local heatman = require("../heatman/navi")
local hook = require("../flmhook/hook")
local invisible = require("../invisibl/controller")
local lockon = require("../../rules/lockon")
local minibomb = require("../minibomb/chip")
local plus = require("../../lib/instant/plus")
local poisseed = require("../poisseed/chip")
local slashman = require("../slashman/navi")
local sparks = require("../../lib/sparks")
local spoutman = require("../spoutman/navi")
local stepswrd = require("../stepswrd/chip")
local sunmoon = require("../sunmoon/sun")
local tenguman = require("../tenguman/navi")
local timebom = require("../timebom/chips")
local tmhkman = require("../tmhkman/navi")
local traps = require("../../lib/traps/controller")
local wideswrd = require("../wideswrd/chip")

local dustbrk = require("../../navis/dustman/chip")
local delecswd = require("../../navis/elecman/chip")
local edeletbm = require("../../navis/eraseman/chip")
local etomahwk = require("../../navis/tomahawkman/chip")
local ftornado = require("../../navis/tenguman/chip")
local heatpres = require("../../navis/heatman/chip")
local rc_brakr = require("../../navis/groundman/chip")
local rslash = require("../../navis/slashman/chip")
local volcchrg = require("../../navis/chargeman/chip")
local dripshwr = require("../../navis/spoutman/chip")

-- A chip with the fields tests don't care about filled in (codes A and
-- `*`, 10 MB, three to a folder; `n` its place in the library), and
-- `fields` over them.
local function chip(key: string, name: string, n: number, fields: { [string]: any }): Chip
    local spec: { [string]: any } = {
        id = "test/" .. key,
        name = name,
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
        library = { number = n, index = bit32.band(n, 0xFF), sort = n },
        slot_in_limit = 3,
    }
    for k, v in fields do
        spec[k] = v
    end
    return define.chip(spec :: any)
end

-- The SunGuns: BN6's GunDelSols' actions (chips/gundels), by level.
local function sun_gun(key: string, name: string, n: number, level: number): Chip
    return chip(key, name, n, { beast = { lockon = lockon.cannon }, action = gundels[level].action })
end

-- A thrown chip.
local function thrown(key: string, name: string, n: number, damage: number, action: Action?): Chip
    return chip(key, name, n, {
        flags = { "has_damage", "standard_library" },
        hit_param = 30,
        damage = damage,
        action = action,
    })
end

-- A sword chip.
local function blade(key: string, name: string, n: number, action: Action?): Chip
    return chip(key, name, n, {
        flags = { "has_damage", "standard_library" },
        family = "sword",
        hit_param = 30,
        damage = 80,
        action = action,
    })
end

-- A navi chip.
type Summon = (user: Object, controller: Object, spec: NaviChipSpec) -> Object?
local function navi_chip(key: string, name: string, n: number, damage: number, summon: Summon, fields: { [string]: any }?): Chip
    local all: { [string]: any } = {
        flags = { "has_damage", "navi", "library" },
        class = "mega",
        damage = damage,
        navi = summon,
    }
    for k, v in fields or {} do
        all[k] = v
    end
    return chip(key, name, n, all)
end

-- An instant chip whose effect spawns an object.
local function spawning(key: string, name: string, n: number, instant: (user: Object, spec: InstantChipSpec) -> ()): Chip
    return chip(key, name, n, {
        flags = { "has_damage", "standard_library" },
        hit_param = 0x30,
        damage = 40,
        lockout = 20,
        instant = instant,
    })
end

-- A link navi's own chip (any navi can use it here): BN6's action
-- (navis/<navi>/chip), with made-up damage.
local function link_chip(key: string, name: string, n: number, of: Chip): Chip
    return chip(key, name, n, {
        flags = { "has_damage", "library" },
        class = "special",
        hit_param = 0x30,
        damage = 60,
        action = of.action,
    })
end

return {
    -- What a zeroed chip field reads (the roles' `chips.zeroed`): a chip
    -- no test uses.
    blank = chip("blank", "Blank", 0, {
        class = "special",
        codes = {},
        instant = function(_user: Object, _spec: InstantChipSpec)
            error("the test content's blank chip has no use")
        end,
    }),
    sun_gun_1 = sun_gun("sun-gun-1", "SunGun1", 0x01, 1),
    sun_gun_2 = sun_gun("sun-gun-2", "SunGun2", 0x02, 2),
    sun_gun_3 = sun_gun("sun-gun-3", "SunGun3", 0x03, 3),
    sun_gun_ex = sun_gun("sun-gun-ex", "SunGunX", 0x04, 4),
    -- Invisibl's six seconds.
    veil = chip("veil", "Veil", 0x05, {
        flags = { "dimming", "standard_library" },
        extra_flags = { "rush_cancels" },
        family = "plus",
        dimming = invisible.hook(360),
    }),
    eraser = navi_chip("eraser", "Eraser", 0x06, 60, eraseman.summon { aim_ticks = 16 }, { family = "cursor", hit_param = 100 }),
    -- A trap chip that sets no object.
    trap = chip("trap", "Trap", 0x09, { flags = { "dimming", "standard_library" }, dimming = traps.hook() }),
    elem_trap = chip("elem-trap", "ElemTrap", 0x30, {
        flags = { "dimming", "has_damage", "standard_library" },
        damage = 40,
        dimming = elemtrap.dimming,
    }),
    time_bomb = chip("time-bomb", "TimeBomb", 0x31, {
        flags = { "dimming", "has_damage", "standard_library" },
        hit_param = 100,
        damage = 50,
        dimming = timebom.timebom1.dimming,
    }),
    time_bomb_plus = chip("time-bomb-plus", "TimeBomb+", 0x32, {
        flags = { "dimming", "has_damage", "standard_library" },
        hit_param = 100,
        damage = 70,
        dimming = timebom.timebom.dimming,
    }),
    -- A plus chip used on its own.
    plus = chip("plus", "Plus", 0x41, { family = "plus", damage = 10, instant = plus.attack }),
    fist = spawning("fist", "Fist", 0x46, fist.instant(3)),
    flame_hook = spawning("flame-hook", "FlmHook", 0x48, hook.instant { big = false, spark = sparks.fire }),
    heatpres = link_chip("heatpres", "HeatPres", 0x50, heatpres),
    delecswd = link_chip("delecswd", "DElecSwd", 0x51, delecswd),
    rslash = link_chip("rslash", "RSlash", 0x52, rslash),
    edeletbm = link_chip("edeletbm", "EDeletBm", 0x53, edeletbm),
    volcchrg = link_chip("volcchrg", "VolcChrg", 0x54, volcchrg),
    dripshwr = link_chip("dripshwr", "DripShwr", 0x55, dripshwr),
    etomahwk = link_chip("etomahwk", "ETomahwk", 0x56, etomahwk),
    ftornado = link_chip("ftornado", "FTornado", 0x57, ftornado),
    rc_brakr = link_chip("rc-brakr", "RCBrakr", 0x58, rc_brakr),
    dustbrk = link_chip("dustbrk", "DustBrk", 0x59, dustbrk),
    -- A blank chip that is the AntiNavi trap when a side's defensive-chip
    -- record holds it.
    anti_navi = chip("anti-navi", "AntiNavi", 0xBA, {
        class = "special",
        codes = {},
        trap = "anti_navi",
        dimming = antinavi.dimming,
    }),
    bomb = thrown("bomb", "Bomb", 0x103, 50, minibomb.action),
    seed = thrown("seed", "Seed", 0x104, 10, poisseed.action),
    flash = thrown("flash", "Flash", 0x105, 40, flshbom[2].action),
    bug = thrown("bug", "Bug", 0x106, 0, bugbomb.action),
    -- A column of three panels ahead; the same after a step two panels
    -- ahead; a strike at stunned or grounded opponents.
    blade = blade("blade", "Blade", 0x109, wideswrd.action),
    step_blade = blade("step-blade", "StepBld", 0x10A, stepswrd.action),
    stun_blade = blade("stun-blade", "StunBld", 0x10B, assnswrd.action),
    elements = navi_chip("elements", "Elements", 0x110, 50, elmntman.summon { cycle_ticks = 4 }, { hit_param = 100 }),
    spout = navi_chip("spout", "Spout", 0x111, 40, spoutman.summon, { element = "aqua" }),
    heat = navi_chip("heat", "Heat", 0x112, 40, heatman.summon),
    elec = navi_chip("elec", "Elec", 0x113, 40, elecman.summon),
    slash = navi_chip("slash", "Slash", 0x114, 40, slashman.summon { wave_damage = 10 }),
    charge = navi_chip("charge", "Charge", 0x115, 40, chrgeman.summon),
    tomahawk = navi_chip("tomahawk", "Tomahawk", 0x116, 40, tmhkman.summon),
    tengu = navi_chip("tengu", "Tengu", 0x117, 40, tenguman.summon),
    blast = navi_chip("blast", "Blast", 0x118, 40, blastman.summon),
    shooter = navi_chip("shooter", "Shooter", 0x119, 30, bass.summon, { class = "giga" }),
    sun_moon = navi_chip("sun-moon", "SunMoon", 0x11A, 90, sunmoon.summon, { class = "giga" }),
}
