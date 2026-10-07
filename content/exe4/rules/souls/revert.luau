--!strict
-- EXE4's revert out of a soul (MegaMan's action 0x11, 0x080EBE90; its one step 0x080EBEAC): once the soul's turns are
-- over the intake asks for it (./init: 0x0800B6C4) and his idle starts it (0x080EEBB4: the engine's unpaused revert),
-- the battle not paused for it. Every soul names this module's action as its `revert`.
--
-- Its step: MegaMan white; first: the revert's sound (0x159), the flash (effect row 3, 20 pixels up), onto his future
-- panel, no longer moving (flag 0x40); back in his base form (NaviStats +0x24 0: his base sprite, 0x0800B90A; what
-- the soul wore taken off, 0x0800B7E8), the status reset (0x0800E05C) and the charge's state (0x0800C5EA), related1
-- none, the dark soul's ticks (0x0800B93C), no longer asked to revert (+0x44's 0x40), its timer 0; and on that first
-- tick (the timer run out) +0x44's flags 0x3D cleared and the attack's end (0x0800C9FC).

local base = require("../../navis/megaman/forms/base").base
local hooks = require("./hooks")
local turns = require("./turns")

local revert = {}

export type State = {
    -- AIAttackVars+0x10: ticks left.
    timer: number,
}

local REVERT_SOUND = asset.sound("soul-revert")
-- Effect row 3, the deletion's flash.
local FLASH = new.effect { sprite = asset.sprite("deletion"), anim = 0, palette = 0 }

-- 0x080EBEAC
function revert.update(me: Object, s: State)
    me.sprite.white = true
    if me.step_init == 0 then
        battle.play_sound(REVERT_SOUND)
        battle.effect(me.pos + Vec3.new(0, 0, 0x14_0000), FLASH)
        me:snap_to_future_panel()
        me:set_status("moving", false)
        local stats = battle.navi(me.alliance)
        hooks.finish(me, stats.form)
        me:load_form_sprite(base)
        stats.form = base
        hooks.identity(me, base)
        me.sprite.white = true
        hooks.start(me, base)
        turns.start(base)
        hooks.settle(me)
        me.related1 = nil
        me:set_request("revert_form", false)
        s.timer = 0
        me.step_init = 4
    end
    s.timer -= 1
    if int.i16(s.timer) > 0 then
        return
    end
    -- (+0x44's 0x3D: the dark soul's, the dimming's and the revert's flags.)
    me:set_navi_state("reverting_form", false)
    me:exit_attack()
end

-- The revert out of a soul, MegaMan's action 0x11: one of rules/souls's actions (./init).
revert.action = new.action {
    id = "souls/revert",
    state = { timer = "u16" },
    update = revert.update,
}

return revert
