--!strict
-- BugBomb (EXE4 chip 0x029; its record at 0x080197EC + 0x29 * 0x2C): the throw, action 0x09's variant 7: the navi holds
-- the bomb in its animation 4 (attachment row 4: 0x080EB4EC) and lobs a bug bomb (attack object 0x77, 0x080DC048,
-- thrown by 0x080DC30C) at the panel three ahead, where it sits with 40 HP for a second and then bursts over the 3x3
-- around it: the shared BugBomb (@exelib/bugbomb/bomb, which EXE5's grew from) in EXE4's look: one of the side's own
-- objects hit by the other side's and blockers (collision rows 12 and 13), its hits confusing (status 0x20, 480 ticks);
-- a crushing hit as it sits bursts it at once, its burst hitting with those same types; broken, it explodes (effect
-- row 0) with the bomb's other sound (0x6F); its burst the explosions of effect row 0x5A (in palette 2) and the bomb's
-- sound (0x70); its landing's sound 0xAC.
--
-- Its bug (0x080DC0F8): one of the six NaviCust bugs of 0x080DC134 (the move, HP drain, custom-screen drain, panel
-- trail, buster blanks and weapon bugs: 1, 2, 3, 4, 6, 8) at random (RNG2), whatever its target has, raised on the
-- side it hits (code 0xFF: rules/navicust/bugs).
-- Its record is written by tools/exe4/gen_content.py (bn6battle-verify) from the EXE4 ROMs.

local attachment = require("../objects/attachment")
local bombs = require("../lib/bombs")
local collision = require("../rules/collision")
local common = require("@exelib/bugbomb/bomb")
local rising_bubble = require("../objects/rising_bubble")
local statuses = require("../rules/status")
local trajectory = require("../lib/trajectory")

local EXPLOSION = asset.sprite("explosion")
-- The bug kinds it may raise (0x080DC134).
local BUG_KINDS: { number } = { 1, 2, 3, 4, 6, 8 }
-- What the navi holds while throwing one: the bomb (attachment row 4) in its animation 4.
local HELD = attachment.look { sprite = asset.sprite("bomb"), palette = 0 }
local HELD_ANIM = 4

-- 0x080DC0F8: one of the kinds at random, raised by code 0xFF.
local function pick_bug(me: Object)
    local kind = common.pick(BUG_KINDS, BUG_KINDS)
    me.collision.bugs = kind * 0x100 + 0xFF
end

local KIND = common.make_kind("bugbomb/bomb", {
    sprite = asset.sprite("bomb"),
    anim = 5,
    is = collision.own_object,
    hit_by = collision.hit_by_other_side_or_blockers,
    status = statuses.confuse_480,
    burst_is = nil,
    burst_hits = nil,
    crushing_hit = "bursts",
    crushed = new.effect { sprite = EXPLOSION, anim = 0, palette = 0 },
    burst = new.effect { sprite = EXPLOSION, anim = 0, palette = 2 },
    land_sound = asset.sound("top-spin"),
    crushed_sound = asset.sound("hit-bomb-0"),
    burst_sound = asset.sound("hit-bomb-1"),
    pick_bug = pick_bug,
    bubble = rising_bubble.kind,
    trajectory = trajectory,
})

local bugbomb: Chip = {
    action = bombs.throw { id = "bugbomb/action", held = HELD, held_anim = HELD_ANIM, thrower = common.thrower(KIND) },
    codes = { "B", "G", "Z", "*" },
    element = "null",
    family = "null",
    class = "standard",
    rarity = 2,
    mb = 30,
    damage = 0,
    hit_param = 0,
    lockout = 0,
    flags = { "standard_library", "library" },
}

return { bugbomb = bugbomb }
