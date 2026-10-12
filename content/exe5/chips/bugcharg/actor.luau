--!strict
-- BugCharg's stand-in (EXE5's actor object #0x59, 0x080C3A40): @exelib/bugcharg/actor in EXE5's look: of its user's
-- facing and NameID, wearing a player user's overlay, hidden from a blind viewer, standing in animation 16; its charger
-- (an attachment: EXE5's row 0x1C, sprite 0C-43, on attach point 0x18) raised in animation 10 with sound 0x94; its
-- shots (objects/flying_shot's row 1, 0x080C3BCC, sound 0xFF) one, and one more for each of its user's NaviCust bugs
-- it takes in (in the operation battle only three of them, three shots each), the save's emotion window glitch
-- cleared with them.

local attachment = require("../../objects/attachment")
local common = require("@exelib/bugcharg/actor")
local flying_shot = require("../../objects/flying_shot")
local stand_in = require("@exelib/dimming/stand_in")
local operation_battle = require("../../lib/operation_battle")
local sparks = require("../../lib/sparks")

local CHARGER_SPRITE = asset.sprite("bug-charger")
local CHARGER = attachment.look { sprite = CHARGER_SPRITE, palette = 0, attach_point = 0x18 }
-- The heavy shot (the flying shot's row 1, 0x080C3BCC): in the charger's sprite.
local SHOT = flying_shot.variant { hit_mod = 3, element = 0, spark = sparks.heavy, sprite = CHARGER_SPRITE, speed = 0xA_0000 }

-- 0x080C3C6C: the bugs it takes in (each cleared), and so its shots: one, and in the operation battle three more
-- for each of the panel trail, the buster's blanks and the HP drain; else one more for each of the step bug, the
-- panel trail, the blanks, the hit status, the custom screen's damage, the emotion swings, the custom drain, the HP
-- drain and the battle start's.
local function take_bugs(me: Object): number
    local stats = battle.navi(me.alliance)
    local shots = 1
    if operation_battle() then
        if stats.panel_trail_level ~= 0 then
            shots += 3
            stats.panel_trail_level = 0
        end
        if stats.buster_blanks ~= 0 then
            shots += 3
            stats.buster_blanks = 0
        end
        if stats.hp_drain ~= 0 then
            shots += 3
            stats.hp_drain = 0
        end
        return shots
    end
    if stats.step_bug == 1 then
        shots += 1
        stats.step_bug = 0
    end
    if stats.panel_trail_level ~= 0 then
        shots += 1
        stats.panel_trail_level = 0
    end
    if stats.buster_blanks ~= 0 then
        shots += 1
        stats.buster_blanks = 0
    end
    if stats.hit_status ~= 0 then
        shots += 1
        stats.hit_status = 0
    end
    if stats.custom_damage ~= 0 then
        shots += 1
        stats.custom_damage = 0
    end
    if stats.emotion_bug == 1 then
        shots += 1
        stats.emotion_bug = 0
    end
    if stats.custom_drain ~= 0 then
        shots += 1
        stats.custom_drain = 0
    end
    if stats.hp_drain ~= 0 then
        shots += 1
        stats.hp_drain = 0
    end
    if stats.battle_start_bug ~= 0 then
        shots += 1
        stats.battle_start_bug = 0
    end
    return shots
end

return common.make {
    id = "bugcharg/actor",
    adopt = function(me: Object, user: Object)
        me.flip = user.flip
        -- (The user's NameID, which its charger's attach point reads.)
        me.identity = user.identity
    end,
    wear = stand_in.overlay,
    unwear = function(me: Object, user: Object)
        me:remove_parts_of(user)
    end,
    hides_from_blind = true,
    stand_anim = 0x10,
    raise_anim = 0xA,
    raise = function(me: Object)
        attachment.attach(me, CHARGER, "held", { anim = 1, while_dimmed = true })
    end,
    raise_sound = asset.sound("appear"),
    shot_sound = asset.sound("junk-shot"),
    -- (0x080C3B50: the bugs, then the save's emotion window glitch.)
    shots = function(me: Object): number
        local shots = take_bugs(me)
        battle.clear_emotion_window_glitch()
        return shots
    end,
    launch = function(me: Object, pos: Vec3, damage: number): Object?
        return flying_shot.launch(me, pos, SHOT, damage)
    end,
}
