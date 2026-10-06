--!strict
-- Rocks (the original's attack object #0x59, `sub_80CF954`): field
-- obstacles with HP that block their panel, break into debris when
-- destroyed, and leave after 6000 ticks. A rock is spawned with its variant
-- (the original's first parameter, a row of `byte_80CF934`: the rock cubes
-- and the ice block), the class of field-object slot it takes (the second:
-- 0, two a side; 1, one) and how it comes onto the field (the third, its
-- entrance). RockCube and IceCube place one (./cube), and others use it
-- too: an obstacle encased in ice becomes an ice block (the encasing's
-- replacement, objects/encased_bubble), and some stages place them when
-- the round starts (stages.luau: the actor lists' entry type 8, the
-- kind's `place`, with the variant the stage names). See
-- docs/engine/field-objects.md. The behavior is EXE5's too
-- (@exelib/rock/rock), made here with EXE6's look.

local absorbed_obstacle = require("../../objects/absorbed_obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/rock")
local debris = require("./debris")
local effects = require("../../lib/effects")
local sparks = require("../../lib/sparks")

local rock = {}

-- The cubes' sheet (animation 0 a cube rising; the rows' are the cubes
-- standing).
local SPRITE = asset.sprite("rock-cubes")

-- What a rock is taken for: a rock cube, an ice block (field objects, with
-- the looks something that takes their place shows).
local ROCK = new.identity {
    id = "rock",
    class = "field_object",
    version = 3,
    ai_index = 1,
    look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
}
local ICE = new.identity {
    id = "ice-block",
    class = "field_object",
    version = 4,
    ai_index = 1,
    look = { sprite = SPRITE, anim = 2, palette = 0, shadow = true },
}

-- The rows of `byte_80CF934` (animation, HP in twos, debris palette, break
-- sound, identity; a row past the second is aqua, and a row without HP has
-- 1).
local ROWS: { [number]: RockVariant } = {
    -- A rock cube anything breaks (no chip or stage places it).
    [0] = common.variant {
        id = "rockcube/rock/brittle",
        anim = 1,
        hp = 1,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- RockCube's, and the stages'.
    [1] = common.variant {
        id = "rockcube/rock/cube",
        anim = 1,
        hp = 200,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- A hard one (no chip or stage places it).
    [2] = common.variant {
        id = "rockcube/rock/hard",
        anim = 1,
        hp = 500,
        element = "null",
        debris_palette = 0,
        break_sound = asset.sound("panel-crack"),
        identity = ROCK,
    },
    -- The ice block: IceCube's, the stages', and what an obstacle encased
    -- in ice becomes.
    [3] = common.variant {
        id = "rockcube/rock/ice",
        anim = 2,
        hp = 200,
        element = "aqua",
        debris_palette = 1,
        break_sound = asset.sound("falling-rock"),
        identity = ICE,
    },
}
rock.variants = { brittle = ROWS[0], cube = ROWS[1], hard = ROWS[2], ice = ROWS[3] }

local made = common.make {
    id = "rockcube/rock",
    sprite = SPRITE,
    debris = debris.kind,
    effects = { puff = effects.puff, dust = effects.dust },
    -- The breaking spark; a neutral body that everything reaches, while
    -- dimmed too (rows 0x0E and 0x0F).
    spark = sparks.breaking,
    is = collision.neutral_body,
    hits = collision.everything,
    land_sound = asset.sound("land"),
    -- The original's obstacle kind 4: its own sprite, in the animation it
    -- stood in.
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    stage_damage = 200,
}
rock.kind = made.kind
rock.spawn = made.spawn

return rock
