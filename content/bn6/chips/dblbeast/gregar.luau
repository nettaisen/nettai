--!strict
-- DblBeast's Gregar beast (the original's actor object #0x33,
-- `sub_80C1A10`; docs/engine/dimming-chip-effects.md §19): it appears in
-- front of the user, lists the enemy's bodies (./beast), and attacks from
-- next to them: a claw, a bite (three hits), a gun (six shots down the
-- row) and a slam (two hits); then it goes, and tells the controller.

local beast = require("./beast")
local attachment = require("../../objects/attachment/attachment")
local buster = require("../../lib/buster")
local collision = require("../../rules/collision")
local regions = require("@exelib/regions")
local sparks = require("../../lib/sparks")
local gregar_beast = require("../../navis/megaman/forms/gregar-beast/form")
local parts = require("../../lib/swords/parts")

local gregar = {}

local SPRITE = asset.sprite("gregar-beast")
-- Its identity: Gregar Beast Out's (the attach points its attachments
-- take).
local IDENTITY = gregar_beast.identity

-- The attacks' patterns (`byte_80C1F94`, `byte_80C1FA8`, `byte_80C1FB8`,
-- `byte_80C1FD8`): {flip, dx, dy} around the target. (The gun's has ten
-- entries, of which the first six are tried.)
local CLAW: DblBeastPattern = { { 0, -1, 0 }, { 1, 1, 0 }, { 0, -1, -1 }, { 1, 1, -1 }, { 0, -1, 1 }, { 1, 1, 1 } }
local BITE: DblBeastPattern = { { 0, -1, 0 }, { 1, 1, 0 }, { 0, -2, 0 }, { 1, 2, 0 } }
local GUN: DblBeastPattern = {
    { 0, -5, 0 }, { 1, 5, 0 }, { 0, -4, 0 }, { 1, 4, 0 }, { 0, -3, 0 },
    { 1, 3, 0 }, { 0, -2, 0 }, { 1, 2, 0 }, { 0, -1, 0 }, { 1, 1, 0 },
}
local SLAM: DblBeastPattern = { { 0, -1, 0 }, { 1, 1, 0 } }

-- The hits (`byte_80C202C`, `byte_80C2048`): the claw's, the bite's, and
-- the slam's two, on the panel in front. (Hit 2's row is zeros, unused.)
local BITE_SPARK = define.spark { sprite = asset.sprite("hit-sparks"), anim = 1, palette = 0 }
local HITS: { [string]: DblBeastHit } = {
    claw = { ahead = 1, region = regions.wide, spark = sparks.heavy, is = collision.attack, damage = 50 },
    bite = { ahead = 1, region = regions.long, spark = BITE_SPARK, is = collision.breaking, damage = 20 },
    slam = { ahead = 1, region = regions.long, spark = sparks.heavy, is = collision.attack, damage = 50 },
    slam_2 = { ahead = 1, region = regions.wide, spark = sparks.heavy, is = collision.attack, damage = 50 },
}
-- The gun's shot (`sub_80C20B4`, 0x04050501): on the first panel down the
-- row with an enemy body or a neutral object on it, 16 pixels up.
local SHOT_SPARK = define.spark { sprite = asset.sprite("pink-flash"), anim = 0, palette = 0 }
local SHOT: DblBeastHit = { ahead = 0, region = regions.single, spark = SHOT_SPARK, is = collision.attack, damage = 5, z = 0x10_0000 }
local SHOT_TARGET = { [0] = 0x05800000, [1] = 0x0A800000 }

-- The bite's drill (attachment row 0x20: GroundCross's drill arm).
local MOUTH = attachment.look { sprite = asset.sprite("drill-arm"), palette = 0, attach_point = 19 }
-- The claw's swipe and the slam's two blows (effects 0x16, 0x3A, 0x39),
-- 16 pixels up on the panel in front.
local SLAM_FX = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 1, palette = 0 }
local SLAM_FX_2 = define.effect { sprite = asset.sprite("slash-man-effect"), anim = 0, palette = 0 }

local SOUND_CLAW = asset.sound("sword-swing")
local SOUND_BITE = asset.sound("drilarm")
local SOUND_GUN = asset.sound("col-army-2")
local SOUND_SLAM = asset.sound("spin")
local SOUND_SLAM_2 = asset.sound("flame-hook-fire")

local ANIM_IDLE, ANIM_CLAW, ANIM_BITE, ANIM_SLAM, ANIM_GUN, ANIM_LEAVE = 0, 5, 0xA, 0xC, 0xE, 4
-- When the claw lands; the bite's and the gun's periods; when the slam's
-- second blow lands.
local CLAW_AT, BITE_EVERY, GUN_EVERY, SLAM_AT = 0x12, 10, 5, 0xF
local LIST_TICKS, LEAVE_TICKS = 0x1E, 3

-- An effect on the panel in front, 16 pixels up, facing the beast's way.
local function effect_ahead(me: Object, fx: Effect)
    local cx, cy = field.center(me.panel_x + me.facing, me.panel_y)
    battle.effect(Vec3.new(cx, cy, 0x10_0000), fx, bit32.bxor(me.alliance, me.flip))
end

-- The attack ends: no miss, the next action.
local function finish(me: Object, next: number)
    (me.state :: DblBeastState).missed = false
    me.action, me.phase, me.phase_init = next, 0, 0
end

-- An attachment ends (`sub_80B8E58`: its lifecycle word goes to destroy;
-- the slot keeps the handle).
local function detach(a: Object?)
    if a then
        a:set_lifecycle("destroy")
    end
end

-- `sub_80C1AEE`: the targets; 31 ticks; then the claw, or the end when
-- there is no target.
local function list(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.anim = ANIM_IDLE
        beast.list_targets(me)
        me.timer = LIST_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        return
    end
    finish(me, if (me.state :: DblBeastState).count ~= 0 then 8 else 0x24)
end

-- A missed attack waits out its ticks.
local function missed_wait(me: Object, next: number): boolean
    if not (me.state :: DblBeastState).missed then
        return false
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        finish(me, next)
    end
    return true
end

-- `sub_80C1B2E`: the claw (a miss doesn't go back to rest).
local function claw(me: Object)
    local s = me.state :: DblBeastState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, CLAW, false) then
            me.anim = ANIM_CLAW
            battle.play_sound(SOUND_CLAW)
            attachment.attach(me, parts.blades.sword, "held", { anim = 0xC, while_dimmed = true })
        end
        return
    end
    if missed_wait(me, 0xC) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t == CLAW_AT then
        effect_ahead(me, parts.effects.wide)
        beast.hit(me, HITS.claw)
        return
    end
    if t > 0 then
        return
    end
    me.anim = ANIM_IDLE
    detach(s.held)
    finish(me, 0xC)
end

-- `sub_80C1BEC`: the bite, a hit every 10 ticks.
local function bite(me: Object)
    local s = me.state :: DblBeastState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, BITE, true) then
            me.anim = ANIM_BITE
            attachment.attach(me, MOUTH, "held", { anim = 1, while_dimmed = true })
            beast.hit(me, HITS.bite)
            battle.play_sound(SOUND_BITE)
            me.timer2 = BITE_EVERY
        else
            beast.rest(me, true)
        end
        return
    end
    if missed_wait(me, 0x14) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 <= 0 then
            me.timer2 = BITE_EVERY
            beast.hit(me, HITS.bite)
        end
        return
    end
    me.anim = ANIM_IDLE
    detach(s.held)
    finish(me, 0x14)
end

-- `sub_80C20B4`: a shot at the first panel down the row with a target.
local function shoot(me: Object)
    local mask = SHOT_TARGET[me.alliance]
    local k = 1
    while true do
        local x = me.panel_x + me.facing * k
        if x < 1 or x > 6 then
            return
        end
        if bit32.btest(field.flags(x, me.panel_y), mask) then
            beast.hit(me, SHOT, x)
            return
        end
        k += 1
    end
end

-- `sub_80C1CC4`: the gun, a shot every 5 ticks, the arm and the muzzle
-- flash restarting with each.
local function gun(me: Object)
    local s = me.state :: DblBeastState
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, GUN, true) then
            me.anim = ANIM_GUN
            shoot(me)
            attachment.attach(me, buster.arm, "held", { anim = 0xB, while_dimmed = true })
            attachment.attach(me, buster.flash, "held2", { anim = 0, while_dimmed = true })
            battle.play_sound(SOUND_GUN)
            me.timer2 = GUN_EVERY
        else
            beast.rest(me, true)
        end
        return
    end
    if missed_wait(me, 0x1C) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t > 0 then
        local t2 = me.timer2 - 1
        me.timer2 = t2
        if t2 > 0 then
            return
        end
        me.timer2 = GUN_EVERY
        me.anim, me.anim_loaded = ANIM_GUN, 0xFF
        shoot(me)
        battle.play_sound(SOUND_GUN)
        -- sub_80B8E7C: the attachments' animations restart.
        if s.held then
            s.held.anim_loaded = 0xFF
        end
        if s.held2 then
            s.held2.anim_loaded = 0xFF
        end
        return
    end
    me.anim = ANIM_IDLE
    detach(s.held)
    detach(s.held2)
    finish(me, 0x1C)
end

-- `sub_80C1DC4`: the slam, a second blow after 15 ticks.
local function slam(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me.timer = beast.TICKS
        if beast.take_panel(me, SLAM, true) then
            me.anim = ANIM_SLAM
            beast.hit(me, HITS.slam)
            effect_ahead(me, SLAM_FX)
            battle.play_sound(SOUND_SLAM)
            battle.shake_camera(2, me.timer + 0x1E)
        else
            beast.rest(me, true)
        end
        return
    end
    if missed_wait(me, 0x24) then
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t == SLAM_AT then
        me.anim, me.anim_loaded = ANIM_SLAM, 0xFF
        beast.hit(me, HITS.slam_2)
        battle.play_sound(SOUND_SLAM_2)
        effect_ahead(me, SLAM_FX_2)
        return
    end
    if t > 0 then
        return
    end
    finish(me, 0x24)
end

-- `sub_80C1E7A`: the last panel let go, 3 ticks, then gone.
local function leave(me: Object)
    if me.phase_init == 0 then
        me.phase_init = 4
        me:unreserve_panel(me.future_panel_x, me.future_panel_y)
        me.anim = ANIM_LEAVE
        me.timer = LEAVE_TICKS
        return
    end
    local t = me.timer - 1
    me.timer = t
    if t <= 0 then
        beast.leave(me, "gregar_out")
    end
end

-- `sub_80C1A7C`: by action (`off_80C1A90`).
local function tick(me: Object)
    local a = me.action
    if a == 0 then
        beast.appear(me)
    elseif a == 4 then
        list(me)
    elseif a == 8 then
        claw(me)
    elseif a == 0xC then
        beast.wait(me, 0x10)
    elseif a == 0x10 then
        bite(me)
    elseif a == 0x14 then
        beast.wait(me, 0x18)
    elseif a == 0x18 then
        gun(me)
    elseif a == 0x1C then
        beast.wait(me, 0x20)
    elseif a == 0x20 then
        slam(me)
    elseif a == 0x24 then
        leave(me)
    else
        error(string.format("DblBeast's Gregar: action %#x reads past off_80C1A90", a))
    end
end

gregar.kind = define.kind {
    id = "dblbeast/gregar",
    pool = "actor",
    state = beast.STATE,
    -- `sub_80C1A10`: by state, then the sprite (stepping while dimmed).
    update = function(me: Object)
        local l = me.lifecycle
        if l == "init" then
            -- sub_80C1A34: at rest in front of the user; its update first
            -- runs next tick.
            beast.init(me, SPRITE, true, IDENTITY)
            me:set_lifecycle("update")
        elseif l == "update" then
            tick(me)
        else
            me:free()
            return
        end
        me:update_sprite_while_dimmed()
    end,
}

-- `sub_80C1EAC`: Gregar's beast for `user`.
function gregar.spawn(controller: Object, user: Object, element: number, base: number): Object?
    return beast.spawn(gregar.kind, controller, user, element, base, "gregar_out")
end

return gregar
