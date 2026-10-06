--!strict
-- BugCharg's stand-in (EXE5's actor object #0x59, 0x080C3A40; EXE5's own, on the dimming stand-ins' skeleton:
-- @exelib/dimming/stand_in): drawn as the user (its animation 16), it raises its charger (an attachment: EXE5's row
-- 0x1C, sprite 0C-43, on attach point 0x18) with a sound, takes in its user's NaviCust bugs (each it clears is a shot
-- more, past the first; in the operation battle only three of them, three shots each) and the save's emotion
-- window glitch, and 41 ticks on fires: a heavy shot (objects/flying_shot's row 1) every 16 ticks from the panel ahead,
-- 20 pixels up, each with a sound and the camera shaken. 31 ticks after the last it gives the user back, and its
-- controller (./controller) moves on.

local attachment = require("../../objects/attachment")
local flying_shot = require("../../objects/flying_shot")
local stand_in = require("@exelib/dimming/stand_in")
local operation_battle = require("../../lib/operation_battle")

export type State = {
    -- The controller waiting on it (@exelib/dimming/stand_in).
    controller: Object?,
    -- Its charger (+0x64).
    held: Object?,
}

local actor = {}

local CHARGER = attachment.look { sprite = asset.sprite("bug-charger"), palette = 0, attach_point = 0x18 }
local SOUND_RAISE = asset.sound("appear")
local SOUND_SHOT = asset.sound("junk-shot")
-- The heavy shot's row in the flying shots' table.
local SHOT_ROW = 1
local AIM, BETWEEN, AFTER = 0x28, 0xF, 0x1E

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

-- Phase 0 (0x080C3B50): its charger raised (animation 10, a sound), the bugs taken in; 41 ticks; then the charger's
-- next animation.
local function aim(me: Object)
    if me.phase_init == 0 then
        me.anim = 0xA
        attachment.attach(me, CHARGER, "held", { anim = 1, while_dimmed = true })
        battle.play_sound(SOUND_RAISE)
        me.timer = AIM
        me.timer2 = take_bugs(me)
        battle.clear_emotion_window_glitch()
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    local held = (me.state :: State).held
    if held then
        held.anim += 1
    end
    me.timer = 0
    me.phase, me.phase_init = 4, 0
end

-- Phase 4 (0x080C3BA6): a shot every 16 ticks (a sound, the camera shaken); after the last, the charger's animation
-- back.
local function fire(me: Object)
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    me.timer = BETWEEN
    local x, y = field.center(me.panel_x + me.facing, me.panel_y)
    local shot = flying_shot.launch(me, Vec3.new(x, y, 0x14_0000), SHOT_ROW, bit32.bor(me.damage, bit32.lshift(me.stamina, 16)))
    if shot then
        shot.run_while_dimmed = true
    end
    battle.play_sound(SOUND_SHOT)
    battle.shake_camera(2, 0x14)
    local n = me.timer2 - 1
    me.timer2 = int.u16(n)
    if n <= 0 then
        local held = (me.state :: State).held
        if held then
            held.anim -= 1
        end
        me.phase, me.phase_init = 8, 0
    end
end

-- Phase 8 (0x080C3BF6): 31 ticks; then the charger let go, the user back, and the controller on.
local function finish(me: Object)
    if me.phase_init == 0 then
        me.timer = AFTER
        me.phase_init = 4
    end
    local left = me.timer - 1
    me.timer = int.u16(left)
    if left >= 0 then
        return
    end
    (me.state :: State).held = nil
    stand_in.leave(me)
end

local PHASES = { [0] = aim, [4] = fire, [8] = finish }

-- 0x080C3B16: shown (but to a viewer blind to its side); by phase.
local function tick(me: Object)
    stand_in.show(me)
    local phase = PHASES[me.phase]
    if not phase then
        error(string.format("BugCharg's stand-in: phase %#x reads past its table (0x080C3B44)", me.phase))
    end
    phase(me)
end

-- 0x080C3A64: drawn as the user (stand_in.init), at its animation 16; its first update.
local function init(me: Object)
    stand_in.init(me)
    me.anim = 0x10
    me:set_lifecycle("update")
    tick(me)
end

actor.kind = define.kind {
    id = "bugcharg/actor",
    pool = "actor",
    state = { controller = "object", held = "object" },
    -- 0x080C3A40.
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            init(me)
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite()
    end,
}

return actor
