--!strict
-- BN5's Anubis statue and its controller (Anubis and PoisPhar, the dimming handler's subtype 17: effect object
-- #0x33, 0x080E411C, and attack object #0x55, 0x080D0868): BN6's (@common/anubis) but for
--
-- - its lifetime: 3000 ticks for either variant (0x080D08E2; BN6 reads its variant's);
-- - how it leaves (0x080D0970): any removal leaves a puff (BN5 neither absorbs nor blinks obstacles out);
-- - its variants (0x080D0858): PoisPhar's statue has 300 HP (BN6's 100);
-- - its drain's self type, BN5's row 0x31 (0x080D0A98; BN6's 0x52), and BN5's look and collision types;
-- - the statues' identities (actor records 0xDB and 0xDC): field objects of AI index 0x21 (BN6's 1);
-- - its action table of 7 rows (no frozen or bubbled: the drain is its 6, BN6's 8, the same action).

local collision = require("../../rules/collision")
local controller = require("@common/anubis/controller")
local effects = require("@bn6/lib/effects")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local statue = require("@common/anubis/statue")

local anubis = {}

local SPRITE = asset.sprite("bn5:anubis")

local made = statue.make {
    id = "bn5:anubis/statue",
    sprite = SPRITE,
    sounds = { land = asset.sound("bn5:land"), ["break"] = asset.sound("bn5:hit-bomb-1") },
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    drain = collision.collision_31,
    nothing = collision.nothing,
    -- BN6's puff and explosion (BN5's are the same rows of the effect table, whose sheets BN5's pack doesn't name).
    effects = { puff = effects.puff, explosion = effects.explosion },
    bubble = rising_bubble.kind,
}
anubis.statue = made

local LIFETIME = 3000

anubis.variants = {
    anubis = statue.variant {
        anim = 0,
        period = 6,
        hp = 100,
        lifetime = LIFETIME,
        identity = define.identity {
            id = "bn5:anubis",
            class = "field_object",
            version = 2,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 0, palette = 0, shadow = true },
        },
    },
    poisphar = statue.variant {
        anim = 1,
        period = 1,
        hp = 300,
        lifetime = LIFETIME,
        identity = define.identity {
            id = "bn5:poisphar",
            class = "field_object",
            version = 3,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = 1, palette = 0, shadow = true },
        },
    },
}

anubis.controller = controller.make { id = "bn5:anubis/controller", statue = made.spawn }

return anubis
