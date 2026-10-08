--!strict
-- HeatBrth, Blizzard, ElecShok and WoodPwdr (EXE4 chips 0x006, 0x005, 0x02A and 0x02B; their records at 0x080197EC +
-- n * 0x2C): the breath action (0x33, `breath.action`): the navi holds the chip's nozzle and breathes on the panel
-- ahead, then the column beyond, a hit of the chip's element where it reaches, and leaves those panels its panel type
-- (`breath.make`, attack object 0x36: their parameters 0, 1, 3 and 2 pick each one's row of its tables).
-- Their records are written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local breath = require("@self/breath")
local sparks = require("../lib/sparks")
local statuses = require("../rules/status")

local NOZZLE = asset.sprite("breath-nozzle")

-- HeatBrth's (its parameter 0): fire (effect rows 0x38 and 0x39, its fire lasting 40 ticks) leaving lava; the roar
-- every 8 ticks.
local HEAT_BREATH = asset.sprite("heat-breath")
local heat_breath = breath.make {
    id = "heatbrth/breath",
    spark = sparks.fire,
    element = 1,
    hit_mod = 3,
    hit_effect = new.effect { sprite = HEAT_BREATH, anim = 0, palette = 0 },
    hit_effect_lasts = true,
    panel_effect = new.effect { sprite = HEAT_BREATH, anim = 1, palette = 0 },
    panel = "lava",
    -- 0x080D3C6C: its panels left alone until it leaves them.
    step = nil,
    sound = breath.every_8(asset.sound("roar")),
}

-- Blizzard's (its parameter 1): frost (effect rows 0x3A and 0x3C) leaving ice.
local BLIZZARD = asset.sprite("blizzard")
local BLIZZARD_SOUND = asset.sound("blizzard")
-- The frost that settles on a panel it hit (effect row 0x3B, for 35 ticks): 5 ticks in on its own, 20 on the column's.
local FROST = new.effect { sprite = BLIZZARD, anim = 1, palette = 0 }
local FROST_TICKS, FROST_AT, FROST_COLUMN_AT = 35, 50, 35
-- Its sound's second blow, when the column ahead was hit (0x080D3E96).
local SECOND_BLOW = 16

-- 0x080D3CCA.
local function frost(x: number, y: number)
    local o = breath.effect_at(x, y, FROST)
    if o then
        o.timer = FROST_TICKS
    end
end

local blizzard_breath = breath.make {
    id = "blizzard/breath",
    spark = sparks.aqua,
    element = 2,
    hit_mod = 3,
    hit_effect = new.effect { sprite = BLIZZARD, anim = 0, palette = 0 },
    hit_effect_lasts = false,
    panel_effect = new.effect { sprite = BLIZZARD, anim = 2, palette = 0 },
    panel = "ice",
    -- 0x080D3C74: the frost.
    step = function(me: Object, s: BreathState)
        if me.timer == FROST_AT and s.hit then
            frost(me.panel_x, me.panel_y)
        end
        if me.timer == FROST_COLUMN_AT then
            breath.on_column(me, s, frost)
        end
    end,
    -- 0x080D3E8A: on its first tick, and on its 16th when the column ahead was hit.
    sound = function(_me: Object, s: BreathState)
        s.count += 1
        if s.count == 1 or (s.count == SECOND_BLOW and (s.ahead or s.above or s.below)) then
            battle.play_sound(BLIZZARD_SOUND)
        end
    end,
}

-- ElecShok's (its parameter 3): lightning (effect rows 0x3E and 0x3F, its lightning lasting 40 ticks) leaving cracked
-- panels; the thunder ball's sound every 8 ticks.
local LIGHTNING = asset.sprite("element-pillar-lightning")
-- Its own panel highlighted until 15 ticks before its end, the column's until its last tick.
local HIGHLIGHT_UNTIL, COLUMN_UNTIL = 15, 1
local elec_breath = breath.make {
    id = "elecshok/breath",
    spark = sparks.elec,
    element = 3,
    hit_mod = 3,
    hit_effect = new.effect { sprite = LIGHTNING, anim = 0, palette = 0 },
    hit_effect_lasts = true,
    panel_effect = new.effect { sprite = LIGHTNING, anim = 1, palette = 0 },
    panel = "cracked",
    -- 0x080D3D60: the panels it hit highlighted.
    step = function(me: Object, s: BreathState)
        if me.timer >= HIGHLIGHT_UNTIL and s.hit then
            field.highlight(me.panel_x, me.panel_y)
        end
        if me.timer >= COLUMN_UNTIL then
            breath.on_column(me, s, field.highlight)
        end
    end,
    sound = breath.every_8(asset.sound("thunder-ball")),
}

-- WoodPwdr's (its parameter 2): powder (effect row 0x3D) leaving grass, its hits confusing (hit modifier 1, status
-- 0x20); the powder's sound every 8 ticks.
local POWDER = new.effect { sprite = asset.sprite("wood-powder"), anim = 0, palette = 0 }
-- More powder on the panels it hit: on its own 16 and 32 ticks in, on the column's 30 and 46.
local POWDER_AT, COLUMN_POWDER_AT = { 39, 23 }, { 25, 9 }

-- 0x080D3D48.
local function powder(x: number, y: number)
    breath.effect_at(x, y, POWDER)
end

local wood_breath = breath.make {
    id = "woodpwdr/breath",
    spark = sparks.wood,
    element = 4,
    hit_mod = 1,
    status = statuses.confuse_480,
    hit_effect = POWDER,
    hit_effect_lasts = false,
    panel_effect = nil,
    panel = "grass",
    -- 0x080D3CEA: more powder.
    step = function(me: Object, s: BreathState)
        local t = me.timer
        if (t == POWDER_AT[1] or t == POWDER_AT[2]) and s.hit then
            powder(me.panel_x, me.panel_y)
        end
        if t == COLUMN_POWDER_AT[1] or t == COLUMN_POWDER_AT[2] then
            breath.on_column(me, s, powder)
        end
    end,
    sound = breath.every_8(asset.sound("wood-powder")),
}

-- A chip's nozzle in its palette (attachment rows 0x18 to 0x1B: the original's row 0x17 and the attack's element).
local function nozzle(palette: number): AttachmentLook
    return attachment.look { sprite = NOZZLE, palette = palette, lift = 0 }
end

local heatbrth: Chip = {
    action = breath.action { id = "heatbrth/action", nozzle = nozzle(0), spawn = heat_breath },
    codes = { "D", "K", "O", "*" },
    element = "fire",
    family = "fire",
    class = "standard",
    rarity = 1,
    mb = 30,
    damage = 90,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local blizzard: Chip = {
    action = breath.action { id = "blizzard/action", nozzle = nozzle(2), spawn = blizzard_breath },
    codes = { "H", "J", "V", "*" },
    element = "aqua",
    family = "aqua",
    class = "standard",
    rarity = 1,
    mb = 26,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local elecshok: Chip = {
    action = breath.action { id = "elecshok/action", nozzle = nozzle(4), spawn = elec_breath },
    codes = { "J", "L", "S", "*" },
    element = "elec",
    family = "elec",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 80,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

local woodpwdr: Chip = {
    action = breath.action { id = "woodpwdr/action", nozzle = nozzle(6), spawn = wood_breath },
    codes = { "F", "T", "W", "*" },
    element = "wood",
    family = "wood",
    class = "standard",
    rarity = 2,
    mb = 40,
    damage = 100,
    hit_param = 30,
    lockout = 0,
    flags = { "has_damage", "standard_library", "library" },
}

return { heatbrth = heatbrth, blizzard = blizzard, elecshok = elecshok, woodpwdr = woodpwdr }
