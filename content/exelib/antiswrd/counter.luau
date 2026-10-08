--!strict
-- AntiSwrd's counter, as EXE5 and EXE6 share it (EXE6's navi action 0x48,
-- `sub_80EEA3C`; EXE5's 0x48; EXE4's 0x39, 0x080EDA68, where its spec says):
-- each game makes its own (`counter.action_with`: its key, blade, booms,
-- sounds and attachment kind or its own hold). When
-- an AntiSwrd trap (or the AntiDmg program's stance, struck by a sword)
-- catches a hit, the navi, invulnerable, swings a blade three times, each
-- swing sending a sonic boom (@exelib/swords/sonic_boom) along its row; the
-- last one's hit knocks back. The rules start it by role
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
    -- The blade's attachment kind, or the game's own hold of it (EXE4's
    -- attachment in the related slot, its animation the navi's number:
    -- 0x080EDAC2).
    attachment: Kind?,
    hold: ((me: Object, blade: AttachmentLook) -> ())?,
    -- Every boom's hit modifier (EXE4's 1: 0x080EDB00; none: by the swings
    -- left, `dword_80EEB14`, the last one's knocking back).
    boom_hit_mod: number?,
    -- The blade's animation by its holder (none: the shared one's,
    -- @exelib/swords/parts; EXE6's gives its Beast forms' arms).
    blade_anim: ((Object) -> number)?,
    -- EXE6's counter makes the navi invulnerable from its start to its end
    -- (`sub_80EEA5C`, `sub_800EB08` in `sub_80EEB18`) and restarts what his
    -- form wears at each swing (`sub_8011450`); EXE5's (action 0x2D,
    -- 0x080EDAF4, 0x080EDB0A, 0x080EDBA8) does neither (`plain = true`).
    plain: boolean?,
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
    if not spec.plain then
        me:set_status_timer("invulnerable", 0xFFFF)
        me:set_status("invulnerable", true)
    end
    set_step(me, 4)
end

-- `sub_80EEA78`: a swing: the blade out, the side's trap record gone; on
-- its 10th tick a sonic boom from the panel ahead; after 22 ticks the next
-- swing, or the end.
local function swing(me: Object, s: State, spec: Spec)
    if me.step_init == 0 then
        me:set_animation(SWING_ANIM)
        if not spec.plain then
            me:refresh_form_overlay()
        end
        battle.clear_linked(me.alliance)
        battle.play_sound(spec.swing_sound)
        if spec.hold then
            spec.hold(me, spec.blade)
        else
            local blade_anim = spec.blade_anim
            parts.hold(spec.attachment :: Kind, me, spec.blade, 0, if blade_anim ~= nil then blade_anim(me) else nil)
        end
        me:set_status("using_action", true)
        s.timer = SWING_TICKS
        me.step_init = 4
    end
    if s.timer == WAVE_AT then
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        local boom = if spec.boom_hit_mod then { hit_mod = spec.boom_hit_mod, pierces = false } else BOOMS[me.attack_count]
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
local function finish(me: Object, s: State, spec: Spec)
    if me.step_init == 0 then
        me:set_request("anti_sword_triggered", false)
        -- sub_800EB08
        if not spec.plain then
            me:set_status_timer("invulnerable", 0)
            me:set_status("invulnerable", false)
        end
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
    return new.action {
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
                finish(me, s, spec)
            else
                error(string.format("AntiSwrd's counter: phase %#x reads past off_80EEA50", step))
            end
        end,
    }
end

return counter
