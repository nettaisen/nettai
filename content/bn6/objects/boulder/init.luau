--!strict
-- A boulder (the original's attack object #0x6E, `sub_80D2290`): the field
-- obstacle some stages start with (the actor lists' entry type 3,
-- `sub_8007450`: the kind's `place`), 500 HP, on
-- its panel's side. It stands through the round's start and takes its
-- place once the fight is on; it has no lifetime. Broken, it leaves two
-- chunks of rock debris and a splash of dust; a chip can remove or absorb
-- it. It is one of the field's two stage objects (the field-object
-- registry's stage slots, which it leaves when it goes). See
-- docs/engine/field-objects.md §3.1. The behavior is BN5's too
-- (@exelib/boulder/boulder), made here with BN6's look.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("./absorbed-obstacle")
local collision = require("../rules/collision")
local common = require("@exelib/boulder")
local debris = require("../chips/rockcube/debris")
local effects = require("../lib/effects")

local boulder = {}

local SPRITE = asset.sprite("boulder")

boulder.kind = common.make {
    id = "boulder",
    sprite = SPRITE,
    -- A field object, with the look something that takes its place shows
    -- (DustMan's junk).
    identity = define.identity {
        id = "boulder",
        class = "field_object",
        version = 1,
        ai_index = 1,
        look = { sprite = SPRITE, anim = 0, palette = 0, shadow = false },
    },
    debris = debris.kind,
    effects = { puff = effects.puff, splash = effects.splash },
    -- A neutral body that everything reaches, while dimmed too (rows 0x0E
    -- and 0x0F).
    is = collision.neutral_body,
    hits = collision.everything,
    -- The original's obstacle kind 5: its own sprite.
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    damage = 200,
}

return boulder
