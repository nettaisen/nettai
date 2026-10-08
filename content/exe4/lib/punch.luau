--!strict
-- EXE4's punch (action 0x12, 0x080EBF6A), which weapon routines 0x08 and 0x24 start (GutsSoul's charged shot): the navi
-- swings its arm (animation 10) with the fist held (attachment row 5, 0c-04) and a counter window open, sounding as it
-- starts (0xA4); on its eighth tick a hitbox strikes the panel ahead, and after 24 ticks a recovery of 6 lets the fist
-- go and exits the attack, handing the lockout on (0x0800C9FC). What the hitbox is comes from the weapon's attack
-- parameters (its spawn parameters: region, spark, target and self type) and the hit modifier the action passes
-- (0x21); each user makes its own (`punch.make { ... }`). (The action strikes otherwise, 0x080D0B9E, for an attack of
-- variant other than 0, which neither routine sets.)

local attachment = require("../objects/attachment")

local punch = {}

local SWING, RECOVER = 0x0, 0x4
-- The arm swung, and the fist (attachment row 5).
local ANIM = 10
local FIST = attachment.look { sprite = asset.sprite("guts-fist"), palette = 0 }
-- Its sound as it starts (0xA4).
local SOUND = asset.sound("fist-swing")
-- Its ticks: 24 swinging, striking on the tick its count reaches 16, then 6 recovering.
local SWING_TICKS, STRIKE_AT, RECOVER_TICKS = 23, 16, 5
-- The hitbox's height and hit modifier.
local STRIKE_Z = 16 * 0x10000
local HIT_MOD = 0x21

type State = { timer: number }

-- A punch's action `id`, its hitbox the region, spark and collision types its weapon's parameters give.
function punch.make(spec: {
    id: string,
    region: Region,
    spark: Spark?,
    target: CollisionType,
    self_type: CollisionType,
}): Action
    -- 0x080EBF88.
    local function swing(me: Object, s: State)
        if me.step_init == 0 then
            me.anim = ANIM
            me:open_counter_window()
            attachment.attach(me, FIST, "related", 0)
            me:set_status("using_action", true)
            s.timer = SWING_TICKS
            battle.play_sound(SOUND)
            me.step_init = 4
        end
        if s.timer == STRIKE_AT then
            battle.hitbox(me, {
                panel_x = me.panel_x + me.facing,
                panel_y = me.panel_y,
                element = me.attack_element,
                z = STRIKE_Z,
                region = spec.region,
                hit_effect = spec.spark,
                target = spec.target,
                self_type = spec.self_type,
                damage = me.attack_damage,
                stamina = me.hit_param,
                hit_mod = HIT_MOD,
            })
        end
        s.timer -= 1
        if s.timer < 0 then
            me.step = RECOVER
            me.step_init = 0
        end
    end

    -- 0x080EBFFC.
    local function recover(me: Object, s: State)
        if me.step_init == 0 then
            s.timer = RECOVER_TICKS
            me.step_init = 4
        end
        s.timer -= 1
        if s.timer < 0 then
            me.related1 = nil
            me:exit_attack()
        end
    end

    return new.action {
        id = spec.id,
        state = { timer = "i16" },
        update = function(me: Object, s: State)
            local step = me.step
            if step == SWING then
                swing(me, s)
            elseif step == RECOVER then
                recover(me, s)
            else
                error(string.format("the punch's phase %#x reads past its phase table", step))
            end
        end,
    }
end

return punch
