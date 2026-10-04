--!strict
-- BN5's BugBomb (attack object #0x77, 0x080D4E54): the shared BugBomb (@exelib/bugbomb/bomb, BN6's code) with BN5's
-- look and bugs. It explodes as effect 0x32 (BN6's 0x4A, the same look) and sounds 0x6F crushed (BN6's 0x70).
--
-- Its bugs (0x080D50CA, BN6's `sub_80D9FC2`): of HP drain, custom-screen HP drain and emotion swings (NaviStats
-- +0x18, +0x19, +0x24; BN6 has a panel trail and buster blanks for the last), one its target's side lacks, at
-- random, or any of the three; in the operation battle, always HP drain. Its codes (0x080D5148) add 2 to either drain,
-- to at most 7 (argument 0x12: BN5's intake, 0x0801103E, adds with bit 4, rules/status's `drain_bug_flags`), or set
-- the emotion swings to 1.

local collision = require("../../rules/collision")
local common = require("@exelib/bugbomb/bomb")
local rising_bubble = require("../../objects/rising-bubble/rising_bubble")
local statuses = require("../../rules/status")
local trajectory = require("../../lib/trajectory")
local operation_battle = require("../../lib/operation_battle")

local bug_bomb = {}

local EXPLOSION = define.effect { sprite = asset.sprite("explosion"), anim = 0, palette = 2 }

-- The bugs (0x080D5148): HP drain +2, custom-screen HP drain +2, emotion swings 1.
local BUGS = {
    [0] = { bug = 0x18, arg = 0x12 },
    [1] = { bug = 0x19, arg = 0x12 },
    [2] = { bug = 0x24, arg = 1 },
}

-- 0x080D50CA.
local function pick_bug(me: Object)
    local choice = 0
    if not operation_battle() then
        local target = battle.navi(bit32.bxor(me.alliance, 1))
        local choices = {}
        if target.hp_drain == 0 then
            table.insert(choices, 0)
        end
        if target.custom_drain == 0 then
            table.insert(choices, 1)
        end
        if target.emotion_bug == 0 then
            table.insert(choices, 2)
        end
        choice = common.pick(choices, { 0, 1, 2 })
    end
    local b = BUGS[choice]
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
    crushed_sound = asset.sound("hit-bomb-0"),
    burst_sound = asset.sound("hit-bomb-1"),
    pick_bug = pick_bug,
    bubble = rising_bubble.kind,
    trajectory = trajectory,
})

return bug_bomb
