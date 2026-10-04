--!strict
-- BN5's boulder (attack object #0x6E, 0x080D38C0): BN6's (@exelib/boulder) but for
--
-- - the damage it is thrown for: 100 (its stage spawner's word, 0x080068D0; BN6's 200);
-- - how it leaves (0x080D39C4): any removal leaves a puff (BN5 neither absorbs nor blinks obstacles out);
-- - its identity (actor record 0xCE): a field object of AI index 0x21 (BN6's 1);
-- - BN5's sheet (10-31), debris (objects/rock), effects and collision types, and its action table of 7 rows (no
--   frozen or bubbled: standing is its 6, BN6's 8, the same action).
--
-- The stages' boulders (an actor list's entry type 3, 0x080068A0, BN6's entry type 0xA's code).

local collision = require("../../rules/collision")
local common = require("@exelib/boulder/boulder")
local rock = require("../rock/rock")

local boulder = {}

local SPRITE = asset.sprite("boulder")

boulder.kind = common.make {
    id = "boulder",
    sprite = SPRITE,
    identity = define.identity {
        id = "boulder",
        class = "field_object",
        version = 1,
        ai_index = 0x21,
        look = { sprite = SPRITE, anim = 0, palette = 0, shadow = false },
    },
    debris = rock.debris,
    effects = { puff = rock.effects.puff, splash = rock.effects.splash },
    is = collision.neutral_body,
    hits = collision.everything,
    absorbed = nil,
    damage = 100,
}

return boulder
