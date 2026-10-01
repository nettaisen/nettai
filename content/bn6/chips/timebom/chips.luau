--!strict
-- TimeBom1-3 and TimeBom+ (the Program Advance of three TimeBoms of one
-- level in a run of codes): a bomb on the first free enemy panel ahead
-- that counts 3, 2, 1 and blows up the whole enemy area unless it is
-- broken first (./controller, ./countdown). TimeBom+'s has four times the
-- HP, its own look, and shakes the camera harder.
--
-- Their records stay the pack's for now (TimeBom+ is a Program Advance,
-- and its recipes name TimeBom1-3 by number): these are their `dimming`
-- hooks, which registration by number runs (chips/090-timebom1).

local collision = require("../../rules/collision")
local controller = require("./controller")
local countdown = require("./countdown")

-- What both bombs are and what they react to: a neutral body, hit by
-- everything; the one field object of their side's own class.
local BODY = { is = collision.neutral_body, hits = collision.everything }

-- The bombs (the original's rows 0 and 1): the animations they rise and
-- stand with, their HP and their NameIDs (the field objects' looks,
-- numbers until identities come).
local BOMB = countdown.variant {
    appear_anim = 0,
    idle_anim = 1,
    hp = 50,
    name = 0xD8,
    class = 1,
    collision = BODY,
}
local BIG_BOMB = countdown.variant {
    appear_anim = 2,
    idle_anim = 3,
    hp = 200,
    name = 0xD9,
    class = 1,
    collision = BODY,
    strong_shake = true,
}

local timebom = { dimming = controller.hook(BOMB) }

return {
    timebom1 = timebom,
    timebom2 = timebom,
    timebom3 = timebom,
    -- TimeBom+.
    timebom = { dimming = controller.hook(BIG_BOMB) },
}
