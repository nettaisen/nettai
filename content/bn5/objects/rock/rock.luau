--!strict
-- BN5's rocks (attack object #0x59, 0x080D0FA0) and their debris (effect object #0x38, 0x080E46F4): BN6's
-- (@exelib/rock) but for
--
-- - its rows (0x080D0F80, 8 bytes: animation, HP in twos, debris palette, break sound, NameID): the ice block has
--   100 HP (BN6's 200), its break sound BN5's 0xD9;
-- - how it leaves (0x080D1170): any removal leaves a puff (BN5 neither absorbs nor blinks obstacles out);
-- - the rocks' identities (actor records 0xD0 and 0xD1): field objects of AI index 0x21 (BN6's 1);
-- - BN5's sheets (rocks 10-02, debris 10-09), effects, sounds and collision types, and its action table of 7 rows
--   (no frozen or bubbled: standing is its 6, BN6's 8, the same action).
--
-- RockCube's and the stages' rocks (an actor list's entry type 8, 0x08006910: the variant the entry names, placed,
-- thrown for 200).

local collision = require("../../rules/collision")
local common = require("@exelib/rock/rock")
local common_debris = require("@exelib/rock/debris")
local sparks = require("../../lib/sparks")

local rock = {}

local SPRITE = asset.sprite("bn5:sprite-10-02")
local SMALL_PUFF = asset.sprite("bn5:small-puff")
-- Effects 1, 2 and 0x14 (BN5's table, 0x080DFC94, as BN6's rows): a splash, dust, a puff.
local SPLASH = define.effect { sprite = SMALL_PUFF, anim = 0, palette = 0 }
local DUST = define.effect { sprite = SMALL_PUFF, anim = 1, palette = 0 }
local PUFF = define.effect { sprite = asset.sprite("bn5:puff"), anim = 2, palette = 0 }

-- The debris (0x080E46F4: sheet 10-09, landing in a splash).
rock.debris = common_debris.make_kind("bn5:rock/debris", asset.sprite("bn5:sprite-10-09"), SPLASH)

-- What a rock is taken for: a rock cube (actor record 0xD0), an ice block (0xD1).
local ROCK = define.identity {
    id = "bn5:rock",
    class = "field_object",
    version = 3,
    ai_index = 0x21,
    look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
}
local ICE = define.identity {
    id = "bn5:ice-block",
    class = "field_object",
    version = 4,
    ai_index = 0x21,
    look = { sprite = SPRITE, anim = 2, palette = 0, shadow = true },
}

local CRACK = asset.sound("bn5:panel-crack")

-- The rows of 0x080D0F80 (a row past the second is aqua, and a row without HP has 1).
local ROWS: { [number]: RockVariant } = {
    -- A rock cube anything breaks (no chip or stage places it).
    [0] = common.variant {
        id = "bn5:rock/brittle",
        anim = 1,
        hp = 1,
        element = "null",
        debris_palette = 0,
        break_sound = CRACK,
        identity = ROCK,
    },
    -- RockCube's, and the stages'.
    [1] = common.variant {
        id = "bn5:rock/cube",
        anim = 1,
        hp = 200,
        element = "null",
        debris_palette = 0,
        break_sound = CRACK,
        identity = ROCK,
    },
    -- A hard one (no chip or stage places it).
    [2] = common.variant {
        id = "bn5:rock/hard",
        anim = 1,
        hp = 500,
        element = "null",
        debris_palette = 0,
        break_sound = CRACK,
        identity = ROCK,
    },
    -- The ice block.
    [3] = common.variant {
        id = "bn5:rock/ice",
        anim = 2,
        hp = 100,
        element = "aqua",
        debris_palette = 1,
        break_sound = asset.sound("bn5:falling-rock"),
        identity = ICE,
    },
}
rock.variants = { brittle = ROWS[0], cube = ROWS[1], hard = ROWS[2], ice = ROWS[3] }

local made = common.make {
    id = "bn5:rock",
    sprite = SPRITE,
    debris = rock.debris,
    effects = { puff = PUFF, dust = DUST },
    spark = sparks.breaking,
    is = collision.neutral_body,
    hits = collision.everything,
    land_sound = asset.sound("bn5:land"),
    absorbed = nil,
    stage_damage = 200,
}
rock.kind = made.kind
rock.spawn = made.spawn
rock.effects = { splash = SPLASH, dust = DUST, puff = PUFF }

return rock
