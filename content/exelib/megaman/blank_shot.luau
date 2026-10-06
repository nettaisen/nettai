--!strict
-- The blank shot, as EXE5 and EXE6 share it (EXE6's weapon routine 2, `sub_8011ADA`, and action 0x33, `sub_80ED748`):
-- what the buster fires when the NaviCust's buster bug picks a blank. The navi raises the buster and it clicks, but
-- nothing is fired; then the buster's recovery, counted from the navi's own panel (a shot's counts from the panel in
-- front; the navi's body is off the field while it updates, so its own panel counts as open). Each game makes its
-- own (`blank_shot.make`: the action's key, its buster parts and its click). See docs/engine/objects-and-player.md
-- §B6.

export type State = {
    -- Ticks into the shot, then ticks of recovery left (AIAttackVars+0x10).
    timer: number,
}

local blank_shot = {}

-- The phases, on the attack's step.
local RAISE, RECOVER = 0x0, 0x4

-- A game's blank shot: its action, and its setup (`sub_8011ADA`: nothing to fire).
function blank_shot.make(look: { id: string, buster: Buster, click: Sound }): { action: Action, setup: (navi: Object) -> Action }
    local buster = look.buster

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
            battle.play_sound(look.click)
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
    local function update(me: Object, s: State)
        local step = me.step
        if step == RAISE then
            raise(me, s)
        elseif step == RECOVER then
            recover(me, s)
        else
            error(string.format("blank shot phase %#x reads past its phase table", step))
        end
    end

    local action = new.action { id = look.id, state = { timer = "u16" }, update = update }

    -- `sub_8011ADA`: nothing to fire.
    local function setup(navi: Object): Action
        navi.attack_element = 0
        navi.charged = 0
        navi.attack_lockout = 0
        navi.extra = 0
        navi.hit_param = 0
        navi.attack_damage = 0
        return action
    end

    return { action = action, setup = setup }
end

return blank_shot
