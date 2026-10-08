--!strict
-- EXE5's Snake (the dimming handler's subtype 16): EXE6's controller (effect object #0x32, EXE6's #0x45, the same code:
-- the dimming's phases, a nest on the user's panel, 41 ticks once it is done) with EXE5's nest and snakes (the nest and
-- controller @exelib/snake/batch, which EXE4 shares).
--
-- - The nest (attack object #0x53, 0x080D03E4, EXE6's #0x72 reworked) scans the user's side's holes from the far
--   column back (the shared scan, @exelib/snake/snake) and sends snakes out of up to three at once, each with its
--   flag (ExtraVars+4: a byte each); when every flag is clear again it sends the next three; once the holes run out
--   and the flags are clear, it goes.
-- - The snake (attack object #0x54, EXE6's #0x73: the shared snake) waits 48 ticks (EXE6 20), strikes as wood (EXE6
--   null) with the wood spark (spark 4, EXE6's 5) and clears its flag when it goes.

local collision = require("../../rules/collision")
local batch = require("@exelib/snake/batch")
local common = require("@exelib/snake")
local trajectory = require("../../lib/trajectory")
local sparks = require("../../lib/sparks")

local snake = {}

-- ---- The snake ---------------------------------------------------------------------------------------------------

snake.kind = common.make_kind("snake/snake", {
    trajectory = trajectory,
    sprite = asset.sprite("snake"),
    rise_sound = asset.sound("appear"),
    leap_sound = asset.sound("aqua-needle-2"),
    wait = 0x30,
    strike_element = 4,
    strike_spark = sparks.wood,
    strike_hit_mod = 3,
    is = collision.attack,
    hits = collision.hits_navis,
    joined = function(_me: Object) end,
    -- 0x080D07D8: its flag cleared.
    left = batch.left,
})

-- ---- The nest and the controller (@exelib/snake/batch) ---------------------------------------------------------

local made = batch.make { nest_id = "snake/nest", controller_id = "snake/controller", snake = snake.kind }
snake.nest = made.nest
snake.controller = made.controller
snake.hook = made.hook

return snake
