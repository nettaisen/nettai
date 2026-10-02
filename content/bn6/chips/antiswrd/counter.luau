--!strict
-- AntiSwrd's counter (the original's navi action 0x48, `sub_80EEA3C`): when
-- an AntiSwrd trap (or the AntiDmg program's stance, struck by a sword)
-- catches a hit, the navi, invulnerable, swings a blade three times, each
-- swing sending a sonic boom (lib/swords/sonic_boom) along its row; the last
-- one's hit knocks back. The ruleset starts it by role (`actions.anti_sword_counter`,
-- rules/roles.luau).

local parts = require("../../lib/swords/parts")
local sonic_boom = require("../../lib/swords/sonic_boom")

local counter = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are
    -- the navi's `attack_count`.)
    timer: number,
}

local STATE = { timer = "u16" }

local SWINGS = 3
local SWING_TICKS, WAVE_AT = 0x15, 0xC
local SWING_ANIM = 5
local SOUND_START = asset.sound("log-in-77")
local SOUND_SWING = parts.sounds.swing
-- The booms by the swings left (`dword_80EEB14`, their hit modifiers).
local BOOMS: { [number]: SonicBoom } = {
    [0] = { hit_mod = 1, pierces = false },
    { hit_mod = 3, pierces = false },
    { hit_mod = 1, pierces = false },
    { hit_mod = 1, pierces = false },
}

local function set_step(me: Object, step: number)
    me.step, me.step_init = step, 0
end

-- `sub_80EEA5C`: three swings, a counter window, invulnerable until done.
local function start(me: Object, s: State)
    me.attack_count = SWINGS
    battle.play_sound(SOUND_START)
    me:open_counter_window()
    me:set_status_timer("invulnerable", 0xFFFF)
    me:set_status("invulnerable", true)
    set_step(me, 4)
end

-- `sub_80EEA78`: a swing: the blade out, the side's trap record gone; on
-- its 10th tick a sonic boom from the panel ahead; after 22 ticks the next
-- swing, or the end.
local function swing(me: Object, s: State, blade: AttachmentLook)
    if me.step_init == 0 then
        me:set_animation(SWING_ANIM)
        me:refresh_form_overlay()
        battle.clear_linked(me.alliance)
        battle.play_sound(SOUND_SWING)
        parts.hold(me, blade, 0)
        me:set_status("using_action", true)
        s.timer = SWING_TICKS
        me.step_init = 4
    end
    if s.timer == WAVE_AT then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        local boom = BOOMS[me.attack_count]
        if boom == nil then
            error(string.format("AntiSwrd's counter: swing %d reads past dword_80EEB14", me.attack_count))
        end
        sonic_boom.throw(me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, boom)
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t >= 0 then
        return
    end
    local left = me.attack_count - 1
    me.attack_count = int.u16(left)
    if left > 0 then
        parts.drop(me)
        me.step_init = 0
        return
    end
    set_step(me, 8)
end

-- `sub_80EEB18`: its trap request done, vulnerable again; 6 ticks, then
-- idle without the blade.
local function finish(me: Object, s: State)
    if me.step_init == 0 then
        me:set_request("anti_sword_triggered", false)
        -- sub_800EB08
        me:set_status_timer("invulnerable", 0)
        me:set_status("invulnerable", false)
        s.timer = 5
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = int.u16(t)
    if t < 0 then
        parts.drop(me)
        me:exit_attack()
    end
end

-- The counter, swinging `blade`. (The original takes the blade from the
-- swords' table, `byte_80EBB64`, by the attack's variant: the elemental
-- swords' blades for 0xC to 0xF, the cannon arm for 0x10 and 0x13, a sword
-- for the rest.)
function counter.action_with(id: string, blade: AttachmentLook): Action
    return define.action {
        id = id,
        state = STATE,
        -- `sub_80EEA3C`, by phase.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                start(me, s)
            elseif step == 4 then
                swing(me, s, blade)
            elseif step == 8 then
                finish(me, s)
            else
                error(string.format("AntiSwrd's counter: phase %#x reads past off_80EEA50", step))
            end
        end,
    }
end

-- What the ruleset starts: both of the counter's starters, a trap chip's
-- catch (`sub_801056A`) and the AntiDmg program's stance (`sub_80105F2`,
-- with the variant its weapon zeroes), leave the attack's variant 0: a
-- sword.
counter.action = counter.action_with("antiswrd/counter", parts.blades.sword)

return counter
