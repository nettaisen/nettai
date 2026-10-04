--!strict
-- BN5's Guardian statue and its controller (the Guardian chip, the dimming handler's subtype 31: effect object
-- #0x52, 0x080E67DC, and attack object #0x7D, 0x080D5EB4): BN6's (@exelib/guardian) but for
--
-- - its HP: 10 (0x080D5F12; BN6's 1);
-- - how it leaves (0x080D6124): any removal leaves a puff (BN5 neither absorbs nor blinks obstacles out);
-- - its strike's burst, BN5's effect 0x36 (0x080D60B2; BN6's 0x45: the same sheet, 14-14, `lightning`);
-- - its identity (actor record 0xE3): a field object of AI index 0x21 (BN6's 1);
-- - BN5's look and collision types, and its action table of 8 rows (no frozen or bubbled: standing and striking
--   back are its 6 and 7, BN6's 8 and 9, the same actions).
--
-- Its strike's dimming is the shared one (./strike: @exelib/guardian/strike, BN5's effect object #0x53, BN6's code:
-- 0x080E6890), whose telop names BN5's Punisher (chip 0x175 in both games).

local collision = require("../../rules/collision")
local controller = require("@exelib/guardian/controller")
local effects = require("../../lib/effects")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local statue = require("@exelib/guardian/statue")
local strike = require("./strike")

local guardian = {}

local SPRITE = asset.sprite("guardian-statue")

guardian.statue = statue.make {
    id = "guardian/statue",
    sprite = SPRITE,
    identity = define.identity {
        id = "guardian",
        class = "field_object",
        version = 4,
        ai_index = 0x21,
        look = { sprite = SPRITE, anim = 0, palette = 0, shadow = true },
    },
    hp = 10,
    stage_damage = 0x00A8_00C8,
    sounds = {
        place = asset.sound("appear"),
        crumble = asset.sound("panel-poison"),
        strike = asset.sound("beast-over-burst"),
    },
    -- Effects 0x15 and 0x36 (BN5's table, 0x080DFC94): sheet 14-0d's animation 3, and 14-14.
    arrival = define.effect { sprite = asset.sprite("puff"), anim = 3, palette = 0 },
    burst = define.effect { sprite = asset.sprite("lightning"), anim = 0, palette = 0 },
    is = collision.neutral_body,
    hits = collision.everything,
    strike_is = collision.thrown_piercing_break,
    strike_hits = collision.hits_navis,
    strike = strike,
    effects = { puff = effects.puff },
    bubble = rising_bubble.kind,
}

guardian.controller = controller.make { id = "guardian/controller", statue = guardian.statue.spawn }

return guardian
