--!strict
-- EXE4's magnets (the chips' action 0x59, 0x080EE012: EXE4's own; EXE5's MagnetSoul's charged shot grew from it). The
-- navi (animation 13, a counter window) holds out the magnet (attachment row 0x23 on by the variant, 0x080EE060: 0c-39,
-- its animation three times the navi's number); 6 ticks later its animation goes one on and its fields (attack 0x75,
-- @exelib/magbolt/magnet, watching the magnet's slot) sit beside the navi for 30 ticks while, each of 31 ticks, a pull
-- reaches the panels the chip says (no damage, hit modifier 4: what it hits slides toward the navi), with the
-- magnet's sound and its hum every 16 ticks. Then 11 ticks (the magnet's animation one more on as they start), and
-- back to idle without it. What a chip says (its variant, 0x080EE0B2 and 0x080EE10E): MagBolt's field on the panel
-- ahead and a pull over the six ahead (region 8); MagShok's fields on the panel ahead and the two beside it, and a pull
-- over the other side's area (regions 0x81, 0x82 by side, 0x080EE148); a field paralyzes by the chip's parameter (its
-- first byte: MagShok's 1).

local attachment = require("../../objects/attachment")
local collision = require("../../rules/collision")
local common = require("@exelib/magbolt/magnet")
local sparks = require("../../lib/sparks")
local statuses = require("../../rules/status")

local magbolt = {}

-- EXE4's magnet field (attack object 0x75, 0x080DBCA0): its element's spark, paralyzing for 150 ticks (status byte
-- 0x12) when its parameter says; its spawn (0x080DBD72) only watches the magnet's slot.
local FIELD = common.make {
    id = "magbolt/magnet",
    attack = collision.attack,
    hits = collision.hits_navis,
    spark_of = sparks.of_element,
    paralysis = statuses.paralyze_150,
    watches = true,
}

local MAGNET = attachment.look { sprite = asset.sprite("mag-bolt"), palette = 0 }
local PULL_SOUND, HUM = asset.sound("magnet-pull"), asset.sound("spark-charge")

-- The navi's number: what it holds animates by it (NaviStats +0x23, MegaMan's 0).
local NAVI_NUMBER = 0

local ANIM = 13
local RAISE_TICKS, PULL_TICKS, RECOVER_TICKS = 6, 30, 10
local HUM_EVERY = 16
-- The pull's hit modifier: the push of bit 2 (toward the puller, over the panels).
local PULL_MOD = 4

type State = {
    -- Ticks left in the step (AIAttackVars +0x10), and toward the hum (+0x12).
    timer: number,
    hum: number,
}

-- A chip's magnet: its key, the rows its fields sit on (from the navi's), whether they paralyze, and the region its
-- pull reaches from the panel ahead.
export type MagnetSpec = {
    id: string,
    rows: { number },
    paralyzes: boolean,
    pull: (me: Object) -> Region,
}

local function step_magnet(me: Object)
    local m = me.related1
    if m then
        m.anim += 1
    end
end

function magbolt.action(spec: MagnetSpec): Action
    -- 0x080EE034: the magnet held out; 6 ticks on, its fields and the pull.
    local function raise(me: Object, s: State)
        if me.step_init == 0 then
            me:set_status("using_action", true)
            me:set_animation(ANIM)
            me:open_counter_window()
            attachment.attach(me, MAGNET, "related", 3 * NAVI_NUMBER)
            s.timer = RAISE_TICKS
            me.step_init = 4
            return
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t > 0 then
            return
        end
        s.timer = PULL_TICKS
        step_magnet(me)
        local damage = int.u32(me.attack_damage + me.hit_param * 0x10000 + me.extra)
        for _, dy in spec.rows do
            FIELD(me, me.facing, dy, me.attack_element, spec.paralyzes, damage)
        end
        battle.play_sound(PULL_SOUND)
        battle.play_sound(HUM)
        s.hum = 0
        me.step, me.step_init = 4, 0
    end

    -- 0x080EE0EE: each tick the pull; the hum every 16 ticks; 31 ticks.
    local function pull(me: Object, s: State)
        s.hum += 1
        if s.hum == HUM_EVERY then
            s.hum = 0
            battle.play_sound(HUM)
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me.step, me.step_init = 8, 0
            return
        end
        battle.hitbox(me, {
            panel_x = me.panel_x + me.facing,
            panel_y = me.panel_y,
            element = 0,
            z = 0,
            region = spec.pull(me),
            hit_effect = nil,
            target = collision.hits_navis,
            self_type = collision.pull,
            damage = 0,
            stamina = 0,
            hit_mod = PULL_MOD,
        })
    end

    -- 0x080EE16C: the magnet's animation one on; 11 ticks, then back to idle without it.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            step_magnet(me)
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        local t = s.timer - 1
        s.timer = int.u16(t)
        if t < 0 then
            me:set_status("using_action", false)
            me.anim = 0
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "u16", hum = "u16" },
        -- 0x080EE012.
        update = function(me: Object, s: State)
            local step = me.step
            if step == 0 then
                raise(me, s)
            elseif step == 4 then
                pull(me, s)
            elseif step == 8 then
                recover(me, s)
            else
                error(string.format("MagBolt's phase %#x reads past its table (0x080EE028)", step))
            end
        end,
    }
end

return magbolt
