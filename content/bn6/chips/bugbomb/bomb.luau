--!strict
-- A BugBomb (the original's attack object #0xA5, `sub_80D9D4C`): the
-- shared BugBomb (@exelib/bugbomb/bomb) with BN6's look and bugs. Lobbed
-- at the panel three ahead, it sits there with 40 HP for a second and then
-- bursts over the 3x3 around it, giving what it hits a NaviCust bug: one
-- its target's side doesn't have yet (HP drain, custom-screen HP drain, a
-- panel trail or buster blanks), picked at random. Breaking it or a
-- crushing hit first ends it in an explosion; landing in a hole, in a puff
-- of smoke.

local collision = require("../../rules/collision")
local common = require("@exelib/bugbomb/bomb")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")

local bug_bomb = {}

-- The explosion (effect 0x4A).
local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 2 }
local SOUND_HIT_BOMB = asset.sound("hit-bomb-1")

-- The bugs (`byte_80DA034`): the code and its argument, by what the target
-- lacks: HP drain, custom-screen HP drain, a panel trail, buster blanks.
local BUGS = {
    [0] = { bug = 0x18, arg = 1 },
    [1] = { bug = 0x19, arg = 1 },
    [2] = { bug = 0xF5, arg = 0 },
    [3] = { bug = 0x14, arg = 4 },
}
-- The panel trail that means the target already has that bug.
local TRAIL_BUGGED = 3

-- `sub_80D9FC2`: a bug its target's side doesn't have yet, at random (one
-- RNG draw), or any of the four when it has them all.
local function pick_bug(me: Object)
    local target = battle.navi(bit32.bxor(me.alliance, 1))
    local choices = {}
    if target.hp_drain == 0 then
        table.insert(choices, 0)
    end
    if target.custom_drain == 0 then
        table.insert(choices, 1)
    end
    if target.panel_trail ~= TRAIL_BUGGED then
        table.insert(choices, 2)
    end
    if target.buster_blanks == 0 then
        table.insert(choices, 3)
    end
    local b = BUGS[common.pick(choices, { 0, 1, 2, 3 })]
    me.collision.bugs = b.arg * 0x100 + b.bug
end

bug_bomb.kind = common.make_kind("bugbomb/bomb", {
    sprite = asset.sprite("bomb"),
    anim = 5,
    -- Its status 0x20.
    is = collision.own_thrown_body,
    hit_by = collision.hit_by_other_side,
    status = statuses.confuse_480,
    burst_is = collision.attack,
    burst_hits = collision.hits_navis,
    crushed = EXPLOSION,
    burst = EXPLOSION,
    land_sound = asset.sound("bug-bomb-land"),
    crushed_sound = SOUND_HIT_BOMB,
    burst_sound = SOUND_HIT_BOMB,
    pick_bug = pick_bug,
    bubble = rising_bubble.kind,
    trajectory = trajectory,
})

-- `sub_80D9FA8`: a thrower for `throw.action` that throws a BugBomb at the
-- panel three ahead of the user.
function bug_bomb.thrower(): Thrower
    return common.thrower(bug_bomb.kind)
end

return bug_bomb
