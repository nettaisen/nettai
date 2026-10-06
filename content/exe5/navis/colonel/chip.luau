--!strict
-- Colonel's own chip, C-Cannon (EXE5 chip 0x19D), which his custom screen offers until it is picked (the navi's
-- `own_chip`): his own action 8 (0x080F8B30), the tank cannon's shot as he fires it. He raises the cannon (animation
-- 10, the attachment table's row 0x25, the TankCans' cannon; the counter window open for 15 ticks) for 8 ticks, waits
-- what the attack gives (none: the chip's own routine, its record's +0x1F, routine 5 of 0x0800EC80, clears it), then
-- fires (animation 9, the cannon's recoil): on the 5th tick the screen shakes and a TankCan shell (attack object
-- #0x3C, chips/tankcan) leaves the panel ahead, 16 pixels up, without the TankCans' cracking; 18 ticks, then the
-- recovery its parameters give (none) and the cannon let go. Its damage is by his level (the chips' damage formula
-- 61: the damage rows' row 12, lib/navi_level).
--
-- The action is also his in an auto battle, with a wait. (Its wait's start also calls 0x080E9FD2, the operation
-- battle's mark, which isn't ported: lib/swords.)
-- Its record is written by tools/exe5/gen_content.py (bn6battle-verify) from the EXE5 ROMs.

local navi_level = require("../../lib/navi_level")
local attachment = require("../../objects/attachment")
local shell = require("@exelib/tankcan/shell")
local tankcan = require("../../chips/tankcan/tankcan")

export type State = {
    -- AIAttackVars +0x10: the step's count.
    timer: number,
    -- AIAttackVars +0x18: the wait before the shot.
    wait: number,
}

-- The steps, on the attack's step.
local RAISE, WAIT, FIRE, RECOVER = 0x0, 0x4, 0x8, 0xC

local ANIM_RAISE, ANIM_FIRE = 10, 9
local COUNTER_TICKS = 15
local RAISE_TICKS, SHELL_TICK, FIRE_TICKS, RECOVER_TICKS = 8, 5, 18, 0
local SHELL_Z = 0x10_0000
-- The cannon's recoil (its second animation).
local CANNON_FIRE = 1
-- The shell's parameters (none: its collision row 0, hit modifier 0x4B; no cracking, its blast unshifted).
local SHELL: ShellSpec = { hit_mod = 0x4B, cracks = false, shift = 0 }

-- 0x080F8B54: the cannon raised; 8 ticks.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_RAISE)
        me:open_counter_window(COUNTER_TICKS)
        s.timer = RAISE_TICKS
        me:set_status("using_action", true)
        attachment.attach(me, tankcan.cannon, "overlay")
        me.step_init = 4
        return
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.step, me.step_init = WAIT, 0
    end
end

-- 0x080F8B96: the wait the attack gives.
local function wait(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = s.wait
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.step, me.step_init = FIRE, 0
    end
end

-- 0x080F8BC2: the cannon recoils; the shell on the count's 5; 18 ticks.
local function fire(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(ANIM_FIRE)
        s.timer = 0
        local cannon = me.overlay
        if cannon then
            -- 0x080B9C06 (`sub_80B8E70`): restarted.
            cannon.anim = CANNON_FIRE
            cannon.anim_loaded = 0xFF
        end
        me.step_init = 4
        return
    end
    if s.timer == SHELL_TICK then
        battle.shake_camera(1, 0x14)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        shell.fire(tankcan.kind, me, me.panel_x + me.facing, me.panel_y, SHELL_Z, damage, SHELL)
    end
    local t = s.timer + 1
    s.timer = t
    if t >= FIRE_TICKS then
        me.step, me.step_init = RECOVER, 0
    end
end

-- 0x080F8C24: the recovery the attack's parameters give (their first byte: the chip's record's, 0); then the cannon
-- let go and back to idle.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = RECOVER_TICKS
        me.step_init = 4
    end
    local t = s.timer - 1
    s.timer = bit32.band(t, 0xFFFF)
    if t <= 0 then
        me.related1 = nil
        me.overlay = nil
        me:set_animation(0)
        me:exit_attack()
    end
end

local action: Action
action = define.action {
    id = "c-cannon/action",
    state = { timer = "u16", wait = "u16" },
    -- 0x080F8B30
    update = function(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == WAIT then
            wait(me, s)
        elseif step == FIRE then
            fire(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("Colonel's C-Cannon's step %#x reads past its table (0x080F8B44)", step))
        end
    end,
}

return define.chip {
    id = "c-cannon",
    codes = { "C" },
    element = "null",
    family = "null",
    class = "special",
    rarity = 0,
    mb = 99,
    damage = navi_level.row({ 80, 80, 100, 100, 120, 120, 120 }),
    hit_param = 148,
    lockout = 0,
    flags = { "has_damage", "library", "variable_damage" },
    slot_in_limit = 1,
    action = action,
    -- 0x0800EC68's routine 5: no wait.
    setup = function(navi: Object)
        (navi:action_state(action) :: State).wait = 0
    end,
}
