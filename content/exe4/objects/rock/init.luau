--!strict
-- EXE4's rocks (attack object #0x59, 0x080D89D8): the shared rock (@exelib/rock, EXE6's and EXE5's code) in EXE4's
-- look: its init (0x080D89F8) writes no NameID and doesn't force its animation's load; the panel it appears on breaks it
-- only for a body or a blocker on it (0x0F800000); its action table (0x080D8ABC, 7 rows: row 1, 0x0801013A, also stops
-- it running while paused; row 3, 0x08010276, does nothing; row 5 the slide, 0x0801143C; row 6 its standing); and it
-- leaves (0x080D8B28) releasing no tracking, by its HP alone: left, a puff (effect 21) 12 pixels up; none, two chunks
-- of debris and dust (effect 2) 16 up and its variant's break sound. EXE4's sheet (10-02), debris and effects
-- (lib/rock) and collision types (rows 14 and 15). Its rows (0x080D89C0, 6 bytes: the standing animation, a byte
-- nothing reads, HP in twos, 0 read as 1, the debris palette, the break sound; aqua from the fourth on) are its
-- spawners' (`rock.variant`): weapon routine 0x1F's rock cube, RockCube's, an obstacle frozen into an ice block.
-- None falls, and no stage places one.

local collision = require("../rules/collision")
local common = require("@exelib/rock")
local rock_lib = require("../lib/rock")
local sparks = require("../lib/sparks")

local rock = {}

-- What on the panel it appears on breaks it: bodies and blockers.
local OCCUPIED = 0x0F80_0000

-- 0x0801013A: back to its idle, no longer running while paused.
local function return_to_idle(me: Object)
    me.run_while_paused = false
    obstacle.return_to_idle(me)
end

-- A rock variant (a row of 0x080D89C0).
rock.variant = common.variant

local made = common.make {
    id = "rock",
    sprite = asset.sprite("rock"),
    reloads_anim = false,
    debris = rock_lib.debris,
    effects = { puff = rock_lib.effects.puff, dust = rock_lib.effects.dust },
    spark = sparks.breaking,
    is = collision.neutral_body,
    hits = collision.everything,
    land_sound = nil,
    absorbed = nil,
    stage_damage = nil,
    occupied = OCCUPIED,
    -- 0x080D8ABC, by the framework's numbers (its rows 0 to 6: 0, 1, 2, 3, 4, 5, 8).
    actions = function(appear: (Object) -> (), destroyed: (Object) -> (), standing: (Object) -> ())
        return {
            [0] = appear,
            [1] = return_to_idle,
            [2] = destroyed,
            [3] = function(_me: Object) end,
            [4] = obstacle.paralyzed,
            [5] = obstacle.slide,
            [8] = standing,
        }
    end,
    leaves_by_hp = true,
}

rock.kind = made.kind
rock.spawn = made.spawn

return rock
