--!strict
-- EXE5's action 0x4C, the roles' `switch_knockout` (`sub_802DAA8`, BRBE 0x0802CE7C; the DS's 0x0202C3C4, the same code,
-- its steps the GBA's with the DS's flags and member blocks): a navi that came in by a switch and falls back instead of
-- being deleted (the engine's fall-back, the DS's 0x0200DFB0: a team navi of a party at 0 HP) is knocked out as the
-- fight runs. It lands with the battle dimmed, turns, and member 0, MegaMan, comes back in its place (the engine's
-- take-back, the DS's 0x0202C4CC); then the battle undims. The GBA's runs only in the gate's navi switch; the DS's in
-- every party battle (a recording's team navi at 0 HP: action 0x4C, back to MegaMan). Its steps (table 0x020D9B34), on
-- the attack's step; the dispatcher steps no sprite.
--
-- - 0 (0x0202C3D4): the battle dims (battle flag 4); the navi shows white, lands on its destination panel on the ground
--   facing the default way, holding it, out of the collision; out of a move, a guard, a flinch, a paralysis, a slide
--   or a drag; animation 2; its overlay told (its third parameter, 1); two flashes (effect 3); its links, overlay and
--   Full Synchro aura cut; the knockout's sound. 10 ticks. (The music and the event flag 0x19F + its navi it sets,
--   0x080010A2, are the console's and the save's.)
-- - 4 (0x0202C4A4): white, animation 4, 4 ticks.
-- - 8 (0x0202C4CC): the take-back, then 120 ticks of flashing (`sub_801A66C`) and its destination's reservation gone.
-- - 0xC (0x0202C62C): its collision back; 4 ticks, then it is no longer switched or knocked out, the requests of a
--   switch and the traps' gone, no flinch asked for, the battle undims, and it idles.

local effects = require("../../lib/effects")
local regions = require("@exelib/regions")

local knockout = {}

export type State = {
    -- AIAttackVars+0x10: ticks left of the step.
    timer: number,
}

local LAND, TURN, TAKE_BACK, SETTLE = 0x0, 0x4, 0x8, 0xC
-- 0x6C.
local SOUND = asset.sound("deleted")
-- `sub_801A66C`'s flashing.
local FLASHING = 0x78

local function set_step(me: Object, step: number)
    me.step = step
    me.step_init = 0
end

local function count_down(me: Object, s: State, next: number)
    local left = s.timer - 1
    s.timer = int.u16(left)
    if left <= 0 then
        set_step(me, next)
    end
end

-- 0x0202C3D4
local function land(me: Object, s: State)
    me.sprite.white = true
    if me.step_init == 0 then
        battle.set_dimmed(true)
        me:snap_to_future_panel()
        me.pos = Vec3.new(me.pos.x, me.pos.y, 0)
        me:update_collision_panels()
        me:face_default()
        me:reserve_panel(me.future_panel_x, me.future_panel_y)
        me.collision:set_region(nil)
        me:stop_moving()
        me:set_status("guard", false)
        me:set_animation(2)
        me:overlay_stepping("told")
        -- (The second flash is spawned straight after the first, its X and Y what the first's spawn left.)
        battle.effect(me.pos, effects.deletion)
        battle.effect_after_spawn(me.pos.z, effects.deletion)
        me.related1 = nil
        me.overlay = nil
        me.full_synchro_aura = nil
        battle.play_sound(SOUND)
        s.timer = 10
        me.step_init = 4
    end
    count_down(me, s, TURN)
end

-- 0x0202C4A4
local function turn(me: Object, s: State)
    me.sprite.white = true
    if me.step_init == 0 then
        me:set_animation(4)
        s.timer = 4
        me.step_init = 4
    end
    count_down(me, s, TAKE_BACK)
end

-- 0x0202C4CC
local function take_back(me: Object)
    me:take_back()
    me:set_status_timer("flash", FLASHING)
    me:set_status("flashing", true)
    me:unreserve_panel(me.future_panel_x, me.future_panel_y)
    set_step(me, SETTLE)
end

-- 0x0202C62C
local function settle(me: Object, s: State)
    if me.step_init == 0 then
        me.collision:set_region(regions.single)
        s.timer = 4
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = int.u16(left)
    if left > 0 then
        return
    end
    for _, state in { "trap_armed", "switch_knockout", "switched", "hovering" } :: { NaviStateFlag } do
        me:set_navi_state(state, false)
    end
    for _, r in { "navi_switch", "switch_knockout", "anti_sword_triggered", "anti_damage_triggered", "flinch" } :: { Request } do
        me:set_request(r, false)
    end
    battle.set_dimmed(false)
    me:exit_attack()
end

-- 0x0202C3C4
local function update(me: Object, s: State)
    local step = me.step
    if step == LAND then
        land(me, s)
    elseif step == TURN then
        turn(me, s)
    elseif step == TAKE_BACK then
        take_back(me)
    elseif step == SETTLE then
        settle(me, s)
    else
        error(string.format("the knockout's step %#x reads past its table (0x020D9B34)", step))
    end
end

knockout.action = new.action { id = "party/knockout", state = { timer = "u16" }, update = update }

return knockout
