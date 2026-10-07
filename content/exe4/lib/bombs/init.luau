--!strict
-- EXE4's throw (the chips' action 0x09, 0x080EB3FE) and thrown bomb (attack object #8, 0x080CE0A4): EXE6's code
-- (@exelib/bombs/throw and bomb) with EXE4's look: the throw's sound (0x90), the navi holding what it throws in its
-- related slot as EXE4's own attachment (nothing in hand for a navi of number 5 or 10, NaviStats +0x23), the thrown
-- thing leaving 4 pixels right of the navi whichever way it faces (0x080EB47A works out the facing's 4 and then loads
-- the X over it); the bomb, each variant a row of 0x080CE088 (what it is and hits, its hit modifier and palette), its
-- blast the region of 0x080CE270 and its explosion the effect row of 0x080CE277. Each chip composes its throw from
-- its variant's rows: what it holds (0x080EB4EC: an attachment row and its animation) and what throws it
-- (0x080EB4C4: the bomb's thrower, of the bomb variant its parameter +0x10 names).

local attachment = require("../objects/attachment")
local bomb = require("@exelib/bombs/bomb")
local throw = require("@exelib/bombs/throw")

local bombs = {}

local SPRITE = asset.sprite("bomb")

-- What a navi holds while throwing a bomb (attachment row 4, 0x080CD524: 0c-02 in palette 0; DarkBomb's row 0x11,
-- palette 3).
bombs.held = attachment.look { sprite = SPRITE, palette = 0 }
bombs.held_dark = attachment.look { sprite = SPRITE, palette = 3 }

-- A bomb variant (a row of 0x080CE088).
bombs.variant = bomb.variant

-- The effect rows the bomb shows (0x080E1698): a blast's explosion (row 0, DarkBomb's row 0x2B in palette 1), the
-- splash in a hole (row 1), the puff when the battle ends (row 0x15).
local EXPLOSION = asset.sprite("explosion")
bombs.effects = {
    explosion = new.effect { sprite = EXPLOSION, anim = 0, palette = 0 },
    dark_explosion = new.effect { sprite = EXPLOSION, anim = 0, palette = 1 },
    splash = new.effect { sprite = asset.sprite("small-puff"), anim = 0, palette = 0 },
    puff = new.effect { sprite = asset.sprite("puff"), anim = 2, palette = 0 },
}

local made = bomb.make {
    id = "bomb",
    sprite = SPRITE,
    -- 0x080CE216.
    hit_sound = asset.sound("hit-bomb-1"),
    splash = bombs.effects.splash,
    explosion = bombs.effects.explosion,
    puff = bombs.effects.puff,
}

-- A thrower that throws a bomb of `variant` (0x080CE2B8), null-element.
bombs.thrower = made.thrower

-- The navis that throw with nothing in hand (NaviStats +0x23: 5 and 10), and MegaMan's number.
local EMPTY_HANDED: { number } = { 5, 10 }
local NAVI_NUMBER = 0

local LOOK: ThrowLook = {
    sound = asset.sound("minibomb-throw"),
    attachment = attachment.kind,
    -- 0x080EB436: EXE4's own attachment in the related slot, but in the hands of navis 5 and 10.
    hold = function(me: Object, held: AttachmentLook, anim: number)
        if not table.find(EMPTY_HANDED, NAVI_NUMBER) then
            attachment.attach(me, held, "related", anim)
        end
    end,
    releases_right = true,
}

-- A chip's throw.
function bombs.throw(spec: ThrowSpec): Action
    return throw.action(spec, LOOK)
end

return bombs
