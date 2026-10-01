--!strict
-- DustMan's own chip, DustBrk (the original's `sub_80F1FA0`, his action
-- table's entry 0xA): a dust cloud on him (navis/dustman/clouds) while he
-- draws everything in his row toward him for 30 ticks (a pull of no damage
-- on each of the other side's panels in his row, every tick); then a
-- second look on him while he crushes the panel ahead (10 ticks in), and
-- 30 ticks of recovery. See docs/engine/standard-chips.md, "Action 0x0A".
--
-- Its record stays the pack's (its damage grows with his level, which
-- chip definitions take with step 10's damage formulas) and names this
-- module, whose `action` it runs (docs/design/content-model-v2.md §12).

local clouds = require("./clouds")
local collision = require("../../rules/collision")
local link = require("../../lib/link_chips")
local regions = require("../../lib/regions")
local sparks = require("../../lib/sparks")

export type State = {
    timer: number,
    -- The dust cloud (AIAttackVars+0x30) and his second look (+0x34).
    cloud: Object?,
    overlay: Object?,
}

local STATE = { timer = "u16", cloud = "object", overlay = "object" }

local PULL, CRUSH, RECOVER = 0x0, 0x4, 0x8
local ANIM_PULL, ANIM_CRUSH = 0x12, 0x14
local SOUND_PULL, SOUND_CRUSH = asset.sound("dustbrk"), asset.sound("dustbrk-2")

local PULL_TICKS, CRUSH_TICKS, RECOVER_TICKS = 0x1E, 0x28, 0x1E
-- The timer value the crush lands at.
local CRUSH_AT = 0x1E

-- The cloud's animation.
local CLOUD_ANIM = 0

-- The other side's area in his row, by side (`byte_80F20E0`).
local OTHER_SIDE = { [0] = { 0x20, 0 }, { 0, 0x20 } }

-- The pull: no damage, no spark, of no side, hit modifier 0x10.
local PULL_HIT_MOD = 0x10

-- The crush: a breaking hit on the panel ahead, a heavy impact.
local CRUSH_HIT_MOD = 3

-- `sub_80F20A0`: the pull on each of the other side's panels in his row.
local function pull(me: Object)
    local cond = OTHER_SIDE[me.alliance]
    for x = 1, 6 do
        if field.check(x, me.panel_y, cond[1], cond[2]) then
            battle.hitbox(me, {
                panel_x = x,
                panel_y = me.panel_y,
                element = 0,
                z = 0,
                region = regions.single,
                hit_effect = 0xFF,
                target = collision.hits_navis,
                self_type = collision.pull,
                damage = 0,
                stamina = 0,
                hit_mod = PULL_HIT_MOD,
            })
        end
    end
end

-- `sub_80F1FC0`: the cloud on, the pull every tick (the entry's too); 30
-- ticks after the entry.
local function draw_in(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        link.add_bonus(me)
        me.anim = ANIM_PULL
        s.cloud = clouds.put_cloud_on(me, CLOUD_ANIM)
        battle.play_sound(SOUND_PULL)
        s.timer = PULL_TICKS
    elseif link.run_out(s) then
        link.go(me, CRUSH)
    end
    pull(me)
end

-- `sub_80F207C`: the crush on the panel ahead.
local function crush_hit(me: Object)
    battle.hitbox(me, {
        panel_x = me.panel_x + me.facing,
        panel_y = me.panel_y,
        element = me.attack_element,
        z = 0,
        region = regions.single,
        hit_effect = sparks.impact,
        target = collision.hits_navis,
        self_type = collision.breaking,
        damage = link.damage(me),
        stamina = link.stamina(me),
        hit_mod = CRUSH_HIT_MOD,
    })
end

-- `sub_80F2004`: his second look on, the cloud off; the crush at 30 left;
-- 40 ticks.
local function crush(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        me.anim = ANIM_CRUSH
        s.overlay = clouds.put_overlay_on(me)
        s.timer = CRUSH_TICKS
        local cloud = s.cloud
        if cloud then
            clouds.take_off(cloud)
        end
        return
    end
    local t = s.timer - 1
    s.timer = t
    if t == CRUSH_AT then
        crush_hit(me)
        battle.play_sound(SOUND_CRUSH)
    elseif t <= 0 then
        link.go(me, RECOVER)
    end
end

-- `sub_80F204E`: 30 ticks, then idle, and only then his second look goes.
local function recover(me: Object, s: State)
    if me.step_init == 0 then
        me.step_init = 4
        s.timer = RECOVER_TICKS
    elseif link.run_out(s) then
        -- (The game reads +0x34 after leaving the action; nothing between
        -- changes it.)
        local overlay = s.overlay
        me.anim = 0
        me:exit_attack()
        if overlay then
            clouds.take_off(overlay)
        end
    end
end

local PHASES = { [PULL] = draw_in, [CRUSH] = crush, [RECOVER] = recover }

return {
    -- `sub_80F1FA0` (`off_80F1FB4`).
    action = define.action {
        id = "dustbrk/action",
        state = STATE,
        update = function(me: Object, s: State)
            link.run(me, s, PHASES, "DustBrk (sub_80F1FA0)")
        end,
    },
}
