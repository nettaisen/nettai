--!strict
-- EXE4's boulder (attack object #0x6E, 0x080DB0EC): the field obstacle its netbattle stages start with (the actor
-- lists' entry type 3, 0x08006880), 500 HP, on its panel's side. EXE6's code (@exelib/boulder, `sub_80D2290`) with
-- EXE4's look:
--
-- - its init (0x080DB10C) writes no NameID: it has no identity;
-- - its tick (0x080DB178) takes its hits never pushed (0x080139A0, EXE6's `sub_801AD6A`) and reacts by 0x08014058
--   (EXE6's `sub_801B4D4`, which its boulder doesn't call: destroyed, the HP as it is, by an obstacle's touch or a
--   breaking hit but no body's, and no removal request read: "destroys_sparing_bodies");
-- - its waiting (0x080DB1BC) keeps action 0 once the fight is on (EXE6's goes to its idle, 8) and, before it, hides
--   it where the battle's settings byte 6 has 0x20 (0x0801466A: no netbattle's, 0x1F);
-- - its action table (0x080DB1A0, 7 rows): row 1 (0x0801013A) also stops it running while paused, row 3
--   (0x08010276) does nothing, row 5 is the knock-back (0x080112C4, EXE6's `sub_8017CC0`), row 6 its idle, which
--   nothing starts but row 1;
-- - its leaving (0x080DB1EC) releases no tracking, and goes by its HP: left, a puff (effect 21) 12 pixels up; none,
--   two chunks of debris and a splash (effect 1) 16 up;
-- - its end (state 8) frees the collision and the object (0x08010560);
-- - EXE4's sheet (10-31), debris and effects (lib/rock), collision types (rows 14 and 15) and the damage it is thrown
--   for, 100 (its stage spawner's word, 0x080068A8).

local collision = require("../rules/collision")
local common = require("@exelib/boulder")
local rock = require("../lib/rock")

local boulder = {}

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

boulder.kind = common.make {
    id = "boulder",
    sprite = asset.sprite("boulder"),
    debris = rock.debris,
    effects = { puff = rock.effects.puff, splash = rock.effects.splash },
    is = collision.neutral_body,
    hits = collision.everything,
    absorbed = nil,
    damage = 100,
    push = "ignored",
    crush = "destroys_sparing_bodies",
    keeps_waiting = true,
    -- 0x080DB1A0, by the framework's numbers (its rows 0 to 6: 0, 1, 2, 3, 4, 5, 8).
    actions = function(wait: (Object) -> (), destroyed: (Object) -> (), standing: (Object) -> ())
        return {
            [0] = wait,
            [1] = return_to_idle,
            [2] = destroyed,
            [3] = function(_me: Object) end,
            [4] = obstacle.paralyzed,
            [5] = obstacle.knocked_back,
            [8] = standing,
        }
    end,
    leaves_by_hp = true,
}

return boulder
