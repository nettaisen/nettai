--!strict
-- Roll, the navi of EXE5's Roll chips (EXE5's actor object #0x2C, 0x080C06A4): EXE6's (@exelib/roll/navi; the same code
-- but for her sprite, her ribbon, EXE5's attachment row 6, 0c-0f, and these) with her heart (./heart), and EXE5's
-- additions:
--
-- - RollSP's, as she appears (0x080C0794): outside the operation battle, a panel changer on her panel turning
--   her side's holy panels Normal (EXE5's change 14: all panels, flickering, 30 ticks, no tick sound).
-- - Her target in the operation battle (0x080C0CA4): the panel in front of the enemy her side tracks, from her
--   side, if it holds no body or object; none otherwise.
-- - As her heart drops (0x080C0B66): her side's navi's mood rises by 50 (0 for a level 5, which no chip has).
--
-- Her palette and her ribbon's are RollDS's 1 (her level 4).

local common = require("@exelib/roll/navi")
local attachment = require("../../objects/attachment")
local panel_changer = require("../../objects/panel_changer")
local heart = require("./heart")
local collision = require("../../rules/collision")
local sparks = require("../../lib/sparks")
local operation_battle = require("../../lib/operation_battle")

-- The SP's change: its side's holy panels Normal (EXE5's `byte_80E272C` row 14, 0x080E2A04).
local HOLY_TO_NORMAL = panel_changer.change {
    panel = "normal",
    panels = "all",
    flicker = true,
    ticks = 0x1E,
    sides = { [0] = { require = 0x2000, forbid = 0x8020 }, [1] = { require = 0x2020, forbid = 0x8000 } },
    change_sound = asset.sound("panel-change"),
}

-- What her level's heart adds to the mood (0x080C0B90).
local MOOD: { [number]: number } = { [0] = 50, 50, 50, 50, 50, 0 }

-- What a panel in front of the tracked enemy must be free of (0x080C0CE4): the bodies and objects of a side.
local OCCUPIED = { [0] = 0x0780_0000, 0x0B80_0000 }

-- 0x080C0794: RollSP's change, outside the operation battle.
local function appeared(me: Object)
    local s = me.state :: any
    if s.level == 3 and not operation_battle() then
        panel_changer.spawn(me, me.panel_x, me.panel_y, HOLY_TO_NORMAL, nil)
    end
end

-- 0x080C0CA4: the panel in front of the enemy her side tracks (`sub_802EFEE`), from her side, if free; 0 otherwise.
local function tracked_target(me: Object): (number, number)
    local t = battle.tracked(me.alliance)
    if not t then
        error("Roll looks for the enemy her side tracks, but it tracks none (the game reads through a null pointer)")
    end
    local x, y = t.panel_x - (1 - me.alliance * 2), t.panel_y
    if field.check(x, y, 0, OCCUPIED[me.alliance]) then
        return x, y
    end
    return 0, 0
end

-- 0x080C0B66: her side's navi's mood rises by her level's amount.
local function heart_dropped(me: Object)
    local level = (me.state :: any).level :: number
    local n = MOOD[level]
    if n == nil then
        error(string.format("Roll's level %d reads past her mood table (0x080C0B90)", level))
    end
    battle.gain_mood(me.alliance, n)
end

return common.make {
    id = "roll/navi",
    sprite = asset.sprite("roll-heart"),
    appear = asset.sound("appear"),
    warp = asset.sound("roll"),
    swish = asset.sound("roll-2"),
    sparkle = heart.sparkle,
    ribbon = attachment.look { sprite = asset.sprite("roll-ribbon"), palette = 0 },
    ribbon_anim = 0,
    attach = attachment.attach,
    heart = heart.spawn,
    spawner_address = 0x080C0D2D,
    hits = collision.hits_navis,
    hit_type = collision.attack,
    spark = sparks.plain,
    appeared = appeared,
    tracked_target = tracked_target,
    heart_dropped = heart_dropped,
}
