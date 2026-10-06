--!strict
-- EXE5's revert out of a soul (0x080121D8, EXE6's `sub_801562C`'s place): when the soul's turns run out (./init),
-- MegaMan's side asks for it; while the battle is paused he is back in his base form. Every soul names this
-- module's action as its `revert`. Unlike EXE6's revert it saves no state to resume, spends nothing and leaves the mood to the status
-- reset.
--
-- Its one step: onto his future panel (its reservation given up), facing the default way, a flash (effect 3) and
-- the fade sound, his moves stopped; the soul's end hook (./hooks), his base form's sprite, the status reset (with
-- the weapons and the mood: as the change's, ./change), his base form's start hook; 30 ticks later the attack
-- requests and the reverting state are dropped and he idles.

local base = require("../../navis/megaman/forms/base").base
local effects = require("../../lib/effects")
local hooks = require("./hooks")

local revert = {}

export type State = {
    -- AIAttackVars+0x10: ticks left.
    timer: number,
}

local FADE = asset.sound("fade")

-- 0x080121D8
function revert.update(me: Object, s: State)
    me.sprite.white = true
    if me.step_init == 0 then
        battle.play_sound(FADE)
        me:snap_to_future_panel()
        me:face_default()
        local e = battle.effect(me.pos + Vec3.new(0, 0, 0x14_0000), effects.deletion)
        if e then
            e.run_while_paused = true
        end
        me:stop_moving()
        local stats = battle.navi(me.alliance)
        hooks.finish(me, stats.form)
        -- sub_800FC9E(0, 0): MegaMan's base sprite.
        me:load_form_sprite(base)
        stats.form = base
        hooks.identity(me, base)
        hooks.settle(me)
        hooks.start(me, base)
        me.related1 = nil
        me.overlay = nil
        s.timer = 30
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) > 0 then
        return
    end
    for _, request in { "buster", "chip", "charged_chip", "back_special", "forced_charged_shot" } do
        me:set_request(request :: Request, false)
    end
    me:set_navi_state("reverting_form", false)
    me:exit_attack()
end

-- The revert out of a soul, as the pause handler runs it: one of rules/souls's actions (./init).
revert.action = new.action {
    id = "souls/revert",
    state = { timer = "u16" },
    update = revert.update,
}

return revert
