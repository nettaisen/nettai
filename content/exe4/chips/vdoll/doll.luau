--!strict
-- EXE4's VDoll doll (attack object #0x7A, 0x080DC7AC): the shared one (@exelib/vdoll/doll, which EXE5's grew from) in
-- EXE4's look, and EXE4's differences:
--
-- - its init (0x080DC7CC) writes no NameID; from the start a body of no side that anything reaches, hit modifier 3
--   (as EXE5's), the plain spark; its setting down (row 6, 0x080DC956) does nothing; no hit breaks it at no damage;
--   a touch destroys it with its HP as it is (0x08013F3E);
-- - where it lands the panel turns to its hole (type 11, 0x08009DC4) with the holes' sound (0x105);
-- - its action table (0x080DC888, 7 rows): row 1 (0x0801013A) also stops it running while paused, row 3 (0x08010276)
--   does nothing, row 5 the knock-back (0x080112C4);
-- - its leaving (0x080DC974): no blinking out, no absorbing, no tracking to release; carried off, gone; held while
--   dimmed; the battle over or not hit, a puff (effect 21) 16 pixels up; flying, gone; else the curse, effect row 0x2B
--   (14-00 in palette 1) 16 up and its sound (0x141), whether or not a chip removed it.

local collision = require("../../rules/collision")
local common = require("@exelib/vdoll/doll")
local curse = require("./curse")
local rock = require("../../lib/rock")
local sparks = require("../../lib/sparks")
local trajectory = require("../../lib/trajectory")

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

return common.make {
    id = "vdoll/doll",
    sprite = asset.sprite("voodoo-doll"),
    identity = nil,
    is = collision.neutral_body,
    hits = collision.everything,
    hit_mod = 3,
    spark = sparks.plain,
    set_down = nil,
    breaking_hits = nil,
    landing_panel = "hole",
    landing_sound = asset.sound("panel-hole"),
    puff = rock.effects.puff,
    trajectory = trajectory,
    curse = curse,
    tracked = false,
    crush = "destroys",
    -- 0x080DC888, by the framework's numbers (its rows 0 to 6: 0, 1, 2, 3, 4, 5, 8).
    actions = function(fly: (Object) -> (), destroyed: (Object) -> (), set_down: (Object) -> ())
        return {
            [0] = fly,
            [1] = return_to_idle,
            [2] = destroyed,
            [3] = function(_me: Object) end,
            [4] = obstacle.paralyzed,
            [5] = obstacle.knocked_back,
            [8] = set_down,
        }
    end,
    exe4_leaving = {
        flash = new.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 1 },
        sound = asset.sound("curse"),
    },
}
