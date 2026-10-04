--!strict
-- The Guardian statue (the original's attack object #0x7D, `sub_80D4C84`;
-- docs/engine/dimming-chip-effects.md §5): a field obstacle with 1 HP that
-- stands for 6000 ticks. Broken by an attack from one side only, it takes
-- the other side's part: in a dimming of its own (./strike, which the
-- other player can't cut in on) it strikes every panel the attacker's navi
-- stands on. Broken otherwise (by a body, by hits from both sides, by its
-- time running out) it just crumbles. The Guardian chip places one
-- (./controller, with the user's side); stages can place one at the start
-- (the kind's `place`, with its panel's side). The behavior is BN5's too
-- (@exelib/guardian/statue), made here with BN6's look.
--
-- An obstacle: its update runs the obstacle framework's steps (the
-- `obstacle` service) around its own actions.

local absorbed_obstacle = require("../../objects/absorbed-obstacle")
local collision = require("../../rules/collision")
local common = require("@exelib/guardian/statue")
local effects = require("../../lib/effects")
local rising_bubble = require("../../objects/rising-bubble")
local strike = require("./strike")

local statue = {}

local SPRITE = asset.sprite("guardian-statue")

local made = common.make {
    id = "guardian/statue",
    sprite = SPRITE,
    -- Its identity: a field object, with the look something that takes its
    -- place shows (DustMan's junk).
    identity = define.identity {
        id = "guardian",
        class = "field_object",
        version = 4,
        ai_index = 1,
        look = { sprite = asset.sprite("guardian-statue"), anim = 0, palette = 0, shadow = true },
    },
    hp = 1,
    sounds = {
        place = asset.sound("appear"),
        crumble = asset.sound("panel-poison"),
        strike = asset.sound("beast-over-burst"),
    },
    -- The flash it arrives in, 16 pixels up (effect 0x15).
    arrival = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 },
    -- The burst on each panel it strikes (effect 0x45).
    burst = define.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0 },
    -- A neutral obstacle that everything reaches (row 0x0E, while dimmed
    -- too, breaking).
    is = collision.neutral_body,
    hits = collision.everything,
    -- The strike: an attack that pierces invisibility and breaks (row
    -- 0x17), hitting navis.
    strike_is = collision.thrown_piercing_break,
    strike_hits = collision.hits_navis,
    strike = strike,
    absorbed = absorbed_obstacle.look { sprite = SPRITE },
    effects = { puff = effects.puff },
    bubble = rising_bubble.kind,
}
statue.kind = made.kind
statue.spawn = made.spawn

return statue
