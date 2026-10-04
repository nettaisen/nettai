--!strict
-- AntiSwrd's counter, as BN5 and BN6 share it (BN6's navi action 0x48,
-- `sub_80EEA3C`; BN5's 0x48): each game makes its own (`counter.action_with`:
-- its key, blade, booms, sounds and attachment kind). When
-- an AntiSwrd trap (or the AntiDmg program's stance, struck by a sword)
-- catches a hit, the navi, invulnerable, swings a blade three times, each
-- swing sending a sonic boom (@exelib/swords/sonic_boom) along its row; the
-- last one's hit knocks back. The ruleset starts it by role
-- (`actions.anti_sword_counter`, a game's rules/roles.luau).

local parts = require("../swords/parts")
local sonic_boom = require("../swords/sonic_boom")

local counter = {}

export type State = {
    -- Ticks left in the swing (AIAttackVars+0x10). (The swings left are
    -- the navi's `attack_count`.)
    timer: number,
}

local STATE = { timer = "u16" }

export type Spec = {
    id: string,
    blade: AttachmentLook,
    boom: Kind,
    start_sound: Sound,
    swing_sound: Sound,
    attachment: Kind,
    -- The blade's animation by its holder (none: the shared one's,
    -- @exelib/swords/parts; BN6's gives its Beast forms' arms).
    blade_anim: ((Object) -> number)?,
}

local SWINGS = 3
local SWING_TICKS, WAVE_AT = 0x15, 0xC
local SWING_ANIM = 5
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
local function start(me: Object, s: State, spec: Spec)
    me.attack_count = SWINGS
    battle.play_sound(spec.start_sound)
    me:open_counter_window()
    me:set_status_timer("invulnerable", 0xFFFF)
    me:set_status("invulnerable", true)
    set_step(me, 4)
end

-- `sub_80EEA78`: a swing: the blade out, the side's trap record gone; on
-- its 10th tick a sonic boom from the panel ahead; after 22 ticks the next
-- swing, or the end.
local function swing(me: Object, s: State, spec: Spec)
    if me.step_init == 0 then
        me:set_animation(SWING_ANIM)
        me:refresh_form_overlay()
        battle.clear_linked(me.alliance)
        battle.play_sound(spec.swing_sound)
        local blade_anim = spec.blade_anim
        parts.hold(spec.attachment, me, spec.blade, 0, if blade_anim ~= nil then blade_anim(me) else nil)
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
        sonic_boom.throw(spec.boom, me, me.panel_x + me.facing, me.panel_y, me.attack_element, damage, boom)
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

-- A game's counter: its key, the blade it swings (the original takes it
-- from the swords' table, `byte_80EBB64`, by the attack's variant: the
-- elemental swords' blades for 0xC to 0xF, the cannon arm for 0x10 and
-- 0x13, a sword for the rest), its booms' kind, the sounds of its start and
-- its swings, and its attachment kind (the blade's).
function counter.action_with(spec: Spec): Action
    return define.action {
        id = spec.id,
        state = STATE,
        -- `sub_80EEA3C`, by phase.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                start(me, s, spec)
            elseif step == 4 then
                swing(me, s, spec)
            elseif step == 8 then
                finish(me, s)
            else
                error(string.format("AntiSwrd's counter: phase %#x reads past off_80EEA50", step))
            end
        end,
    }
end

return counter
