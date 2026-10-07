--!strict
-- Roll, the navi of EXE4's Roll chips (EXE4's actor object #0x2C, 0x080C151C), and each chip's controller (effect
-- object #0x2D, 0x080E5458: lib/navi_chips' with no warp back, `RETURNED`: she brings her user back herself).
--
-- Roll is EXE6's and EXE5's (@exelib/roll/navi; the same code) with EXE4's look (her sprite, 08-01, in her chip's
-- palette, 0x0800B950: two palettes a variant; her ribbon, attachment row 7, 0c-0f, animating while dimmed, kept in
-- her first related slot; her sounds and sparkle) and its differences:
--
-- - her spawner (0x080C1B30) keeps neither her user nor its facing, and her height is her side, which her init keeps;
-- - her vanish's animation is 4 and her wind-up's 10;
-- - she lands in front of the enemy on a panel no body of her side's nor object is on (0x080C1AE8, by side), her
--   user's panel no exception;
-- - her user warps back in by side (lib/navi_chips' warp, 0x080E2D26);
-- - her heart (./heart) drops on her side's navi's panel as it stands then (none without one), healing three times
--   her damage's low 14 bits; then her side's mood rises by her chip's amount (0x080C19DC by the chip's level: 50, but
--   a DS's 0; 0x0800F51E, which no netbattle's event flag 0x1187 stops: rules/flow's `navi_win_banner`).

local common = require("@exelib/roll/navi")
local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local heart = require("./heart")
local navi_chips = require("../../lib/navi_chips")
local sparks = require("../../lib/sparks")

-- What a panel she lands on must be free of, by her side (0x080C1AE8).
local OCCUPIED = { [0] = 0x0780_0000, 0x0B80_0000 }

-- 0x080C1988: her heart on her side's navi's panel.
local function heart_at(me: Object): (Object?, number, number)
    local navi = battle.player(me.alliance)
    if not navi then
        return nil, 0, 0
    end
    return me, navi.panel_x, navi.panel_y
end

-- 0x080C19B4: her side's mood rises by her chip's amount (0x0800F51E; under event flag 0x1187, which no netbattle
-- sets, it wouldn't).
local function heart_dropped(me: Object)
    battle.gain_mood(me.alliance, (me.state :: any).mood :: number)
end

-- 0x080CD6E0, as her lash asks (attachment row 7, her ribbon's animation, animating while dimmed).
local function attach(owner: Object, look: AttachmentLook, which: Slot, options: AttachmentOptions?): Object?
    local anim = if options then options.anim or 0 else 0
    local while_dimmed = options ~= nil and options.while_dimmed == true
    return attachment.attach(owner, look, which, anim, while_dimmed)
end

local roll = common.make {
    id = "roll/navi",
    sprite = asset.sprite("roll"),
    appear = asset.sound("appear"),
    warp = asset.sound("roll-warp"),
    swish = asset.sound("roll-whip"),
    sparkle = heart.sparkle,
    ribbon = attachment.look { sprite = asset.sprite("roll-ribbon"), palette = 0 },
    ribbon_anim = 0,
    attach = attach,
    heart = heart.spawn,
    spawner_address = 0,
    hits = collision.hits_navis,
    hit_type = collision.attack,
    spark = sparks.plain,
    heart_dropped = heart_dropped,
    vanish_anim = 4,
    wind_up_anim = 10,
    summoned_alone = true,
    ribbon_slot = "related",
    occupied = OCCUPIED,
    user_panel_free = false,
    recall = function(me: Object)
        navi_chips.warp(me.alliance, false)
    end,
    heart_at = heart_at,
    heal_mask = 0x3FFF,
}

-- What each Roll chip states for its own controller: the controller's id (the original's effect object #0x2D for all
-- three: compat maps each to it), her palette (0x0800B950 by the chip's level: two palettes a variant) and what her
-- heart adds to the mood (0x080C19DC by the level).
export type ChipSpec = { id: string, palette: number, mood: number }

-- A Roll chip's controller.
local function controller(chip: ChipSpec): NaviChipController
    local summon = roll.summon { rounds = 1, palette = chip.palette, mood = chip.mood }
    -- 0x080E54E2: Roll, with the controller's element and the damage plus the bonus.
    local function spawn(c: Object, user: Object, damage: number): Object?
        return summon(user, c, { panel_x = c.panel_x, panel_y = c.panel_y, element = c.element, damage = damage })
    end
    return navi_chips.controller { id = chip.id, name = "Roll", spawn = spawn, steps = navi_chips.RETURNED }
end

return { controller = controller }
