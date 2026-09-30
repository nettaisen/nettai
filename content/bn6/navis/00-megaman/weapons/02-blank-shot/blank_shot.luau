--!strict
-- The blank shot: weapon routine 2 (`sub_8011ADA`) and action 0x33
-- (`sub_80ED748`), what the buster fires when the NaviCust's buster bug
-- picks a blank. The navi raises the buster and it clicks, but nothing is
-- fired; then the buster's recovery, counted from the navi's own panel (a
-- shot's counts from the panel in front; the navi's body is off the field
-- while it updates, so its own panel counts as open). See
-- docs/engine/objects-and-player.md §B6.

local buster = require("../../../../lib/buster")

export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
}

local blank_shot = {
    state = { timer = "u16" },
}

local ACTION = 0x33

-- The phases, on the attack's step.
local RAISE, RECOVER = 0x0, 0x4

-- `sub_8011ADA`: nothing to fire.
function blank_shot.setup(navi: Object): number
    navi.attack_element = 0
    navi.variant = 0
    navi.charged = 0
    navi.attack_lockout = 0
    navi.extra = 0
    navi.hit_param = 0
    navi.attack_damage = 0
    for i = 1, 4 do
        navi:set_attack_param(i, 0)
    end
    return ACTION
end

-- `sub_80ED764`: raise the buster; the click on the second tick; after 5
-- ticks, the recovery.
local function raise(me: Object, s: State)
    if me.step_init == 0 then
        me:set_animation(0x0E)
        buster.raise_arm(me)
        me:set_status("using_action", true)
        s.timer = 0
        me.step_init = 4
    end
    if s.timer == 1 then
        battle.play_sound(0xF8)
    end
    s.timer += 1
    if s.timer > 4 then
        me.step = RECOVER
        me.step_init = 0
    end
end

-- `sub_80ED7A2`: the recovery; a move may cut it short.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        s.timer = buster.recovery(me, me.panel_x, me.panel_y)
        me.step_init = 4
    end
    local left = s.timer - 1
    s.timer = left
    if left < 0 then
        me.related1 = nil
        me.overlay = nil
        me:exit_attack()
        return
    end
    buster.move_cancel(me)
end

-- `sub_80ED748`.
function blank_shot.update(me: Object, s: State)
    local step = me.step
    if step == RAISE then
        raise(me, s)
    elseif step == RECOVER then
        recover(me, s)
    else
        error(string.format("blank shot phase %#x reads past its phase table", step))
    end
end

return blank_shot
