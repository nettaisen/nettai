--!strict
-- EXE5's Anubis statue and its controller (Anubis and PoisPhar, the dimming handler's subtype 17: effect object
-- #0x33, 0x080E411C, and attack object #0x55, 0x080D0868): EXE6's (@exelib/anubis) but for
--
-- - its lifetime: 3000 ticks for either chip's statue (0x080D08E2; EXE6 reads its variant's);
-- - how it leaves (0x080D0970): any removal leaves a puff (EXE5 neither absorbs nor blinks obstacles out);
-- - its variants (0x080D0858), each chip's own (`anubis.variant`): PoisPhar's statue has 300 HP (EXE6's 100);
-- - its drain's self type, EXE5's row 0x31 (0x080D0A98; EXE6's 0x52), and EXE5's look and collision types;
-- - the statues' identities (actor records 0xDB and 0xDC): field objects of AI index 0x21 (EXE6's 1);
-- - its action table of 7 rows (no frozen or bubbled: the drain is its 6, EXE6's 8, the same action).

local collision = require("../../rules/collision")
local controller = require("@exelib/anubis/controller")
local effects = require("../../lib/effects")
local rising_bubble = require("../../objects/rising_bubble")
local statue = require("@exelib/anubis/statue")

local anubis = {}

local SPRITE = asset.sprite("anubis")

local made = statue.make {
    id = "anubis/statue",
    sprite = SPRITE,
    sounds = { land = asset.sound("land"), ["break"] = asset.sound("hit-bomb-1") },
    is = collision.own_body,
    hits = collision.hit_by_other_side,
    drain = collision.collision_31,
    nothing = collision.nothing,
    -- The puff and the explosion (lib/effects: EXE5's effect table's rows 0x14 and 0, EXE6's rows).
    effects = { puff = effects.puff, explosion = effects.explosion },
    bubble = rising_bubble.kind,
}
anubis.statue = made

local LIFETIME = 3000

-- A chip's statue (a row of 0x080D0858): its animation, its drain's period, its HP, and its identity's version (a field
-- object of AI index 0x21, in the statue's animation), with EXE5's lifetime.
function anubis.variant(v: { anim: number, period: number, hp: number, version: number }): AnubisVariant
    return statue.variant {
        anim = v.anim,
        period = v.period,
        hp = v.hp,
        lifetime = LIFETIME,
        identity = new.identity {
            class = "field_object",
            version = v.version,
            ai_index = 0x21,
            look = { sprite = SPRITE, anim = v.anim, palette = 0, shadow = true },
        },
    }
end

anubis.controller = controller.make { id = "anubis/controller", statue = made.spawn }

return anubis
