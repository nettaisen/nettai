--!strict
-- The bubble an obstacle encased in a bubble becomes (the original's
-- attack object #0xA3, `sub_80D984C`; docs/engine/field-objects.md §4.5):
-- the shared ice cube (@exelib/ice_cube, EXE4's ColdMan's too), EXE6's
-- kind: an aqua obstacle that bursts when a body or another obstacle
-- touches it, and, pushed, rolls toward the enemy 10 pixels a tick until
-- it leaves solid ground. Only the encasing reaches it (`hooks.encased`,
-- rules/roles.luau), and nothing in EXE6 encases: unverified here.

local collision = require("../../rules/collision")
local common = require("@exelib/ice_cube")
local debris = require("../../chips/rockcube/debris")
local effects = require("../../lib/effects")
local rock = require("../../chips/rockcube/rock")
local sparks = require("../../lib/sparks")

local bubble = {}

local made = common.make {
    id = "encased-bubble",
    -- Sheet 10-00 (`sub_80D9870`).
    sprite = asset.sprite("rock-cubes"),
    sounds = { form = asset.sound("place"), burst = asset.sound("falling-rock") },
    spark = sparks.of_element(2),
    is = collision.neutral_body,
    hits = collision.everything,
    -- A push hit (hit modifier bits 0x3C).
    pushes = 0x3C,
    debris = debris.kind,
    -- Effect 2 (bubbles) as it bursts, 0x14 (a puff) off solid ground.
    effects = { burst = effects.bubbles, puff = effects.puff },
    ice_block = { spawn = rock.spawn, variant = rock.variants.ice },
}

bubble.kind = made.kind

-- The role `hooks.encased` (`sub_801813A`'s end): an ice block, or the
-- bubble (`sub_80D99EC`).
bubble.replace = made.replace

return bubble
